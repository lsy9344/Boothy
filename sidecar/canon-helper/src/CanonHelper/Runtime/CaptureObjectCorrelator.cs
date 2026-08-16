namespace CanonHelper.Runtime;

internal enum CaptureObjectRole
{
    Raw,
    Jpeg,
    Unknown,
}

/// <summary>role을 무엇으로 판정했는지. evidence에 그대로 남긴다.</summary>
internal enum CaptureObjectRoleSignal
{
    /// <summary><c>EdsDirectoryItemInfo.format</c>이 알려진 코드였다.</summary>
    SdkFormat,

    /// <summary>format을 해석할 수 없어 파일 확장자로 판정했다.</summary>
    FileExtension,

    /// <summary>둘 다 실패했다. 이 경우 RAW로 가정하지 않는다.</summary>
    Unknown,
}

internal sealed record CaptureObjectClaim(
    bool Accepted,
    int ObjectIndex,
    CaptureObjectRole Role,
    CaptureObjectRoleSignal RoleSignal,
    bool UsedFallbackCorrelation,
    string? RejectReason
);

/// <summary>
/// 한 촬영(request)에 속한 transfer object들을 묶는 순수 상태 기계.
/// </summary>
/// <remarks>
/// <para>
/// <b>왜 필요한가.</b> 이전 구현은 capture당 첫 transfer object 하나만 download하고 나머지를
/// <c>EdsRelease</c>로 버렸다(<c>Interlocked.Exchange(DownloadStarted, 1)</c>). RAW+JPEG를 켜면
/// 카메라가 object event를 두 번 올리는데 두 번째가 거기서 사라졌다. 그 상태로 측정하면
/// "카메라가 RAW+JPEG를 지원하지 않는다"는 <i>잘못된 결론</i>이 나온다.
/// </para>
/// <para>
/// 완료 판정을 <b>object 단위가 아니라 request 단위</b>로 옮기는 것이 이 타입의 요점이다.
/// in-flight capture는 여전히 1개만 허용한다 — object가 여러 개일 뿐이다.
/// </para>
/// <para>
/// <b>기본값은 오늘과 같다.</b> <c>expectedObjectCount</c>가 1이면 첫 object만 수락하고
/// 나머지를 거부해, 측정 lane이 꺼진 제품 경로의 동작이 바뀌지 않는다.
/// </para>
/// </remarks>
internal sealed class CaptureObjectCorrelator
{
    /// <summary>
    /// <c>groupID</c>를 쓸 수 없을 때 같은 촬영으로 묶어 줄 도착 시각 창.
    /// </summary>
    public static readonly TimeSpan DefaultFallbackWindow = TimeSpan.FromSeconds(5);

    private static readonly string[] RawExtensions = [".cr2", ".cr3", ".crw", ".raw"];
    private static readonly string[] JpegExtensions = [".jpg", ".jpeg"];

    // EDSDK ImageFormat_* (EDSDK.cs)
    private const uint EdsImageFormatJpeg = 0x00000001;
    private const uint EdsImageFormatCrw = 0x00000002;
    private const uint EdsImageFormatRaw = 0x00000004;
    private const uint EdsImageFormatCr2 = 0x00000006;

    // PTP object format codes. 어느 쪽이 오더라도 해석할 수 있게 둘 다 인식한다.
    private const uint PtpFormatExifJpeg = 0x3801;
    private const uint PtpFormatCanonRaw = 0xB103;
    private const uint PtpFormatCanonCrw = 0xB101;

    private readonly object _sync = new();
    private readonly TimeSpan _fallbackWindow;

    private uint? _groupId;
    private string? _firstStem;
    private DateTimeOffset? _firstArrivedAt;
    private int _acceptedObjects;
    private bool _rawAccepted;
    private bool _jpegAccepted;

    public CaptureObjectCorrelator(int expectedObjectCount, TimeSpan? fallbackWindow = null)
    {
        ExpectedObjectCount = Math.Max(1, expectedObjectCount);
        _fallbackWindow = fallbackWindow ?? DefaultFallbackWindow;
    }

    public int ExpectedObjectCount { get; }

    public int AcceptedObjects
    {
        get
        {
            lock (_sync)
            {
                return _acceptedObjects;
            }
        }
    }

    /// <summary>
    /// RAW object가 도착했는가. <b>이 값 하나가 촬영 성공 기준이다.</b>
    /// </summary>
    /// <remarks>
    /// JPEG object가 실패하거나 취소되어도 이 값이 <c>true</c>면 request는 성공이다.
    /// Story 1.5~1.7이 만든 계약이며 Story 7.3은 그 경계를 넓히지 않는다.
    /// </remarks>
    public bool HasRawObject
    {
        get
        {
            lock (_sync)
            {
                return _rawAccepted;
            }
        }
    }

    public bool HasJpegObject
    {
        get
        {
            lock (_sync)
            {
                return _jpegAccepted;
            }
        }
    }

    /// <summary>기대한 object를 모두 받았는가. 더 이상 기다릴 필요가 없다는 뜻일 뿐이다.</summary>
    public bool IsRequestComplete
    {
        get
        {
            lock (_sync)
            {
                return _acceptedObjects >= ExpectedObjectCount;
            }
        }
    }

    /// <summary>
    /// 도착한 transfer object를 이 request의 것으로 받아들일지 판정한다.
    /// </summary>
    /// <remarks>
    /// 거부해도 예외를 던지지 않는다. 호출자는 거부된 object를 <c>EdsRelease</c>하고
    /// 그 사실을 <c>RejectReason</c>과 함께 표본에 남긴다. <b>조용히 버리지 않는다.</b>
    /// </remarks>
    public CaptureObjectClaim TryClaim(
        uint groupId,
        string? fileName,
        uint format,
        DateTimeOffset arrivedAt
    )
    {
        var (role, roleSignal) = ResolveRole(fileName, format);

        lock (_sync)
        {
            if (role == CaptureObjectRole.Unknown)
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    false,
                    "unknown-object-role"
                );
            }

            if (_acceptedObjects >= ExpectedObjectCount)
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    false,
                    "request-already-complete"
                );
            }

            if (ExpectedObjectCount == 1 && role != CaptureObjectRole.Raw)
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    false,
                    "unexpected-non-raw-object"
                );
            }

            var usedFallbackCorrelation = false;
            var candidateStem = StemOf(fileName);

            if (
                ExpectedObjectCount > 1
                && groupId == 0
                && string.IsNullOrWhiteSpace(candidateStem)
            )
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    true,
                    "correlation-failed"
                );
            }

            if (_acceptedObjects == 0)
            {
                // 첫 object가 이 request의 correlation 기준을 세운다.
                _groupId = groupId != 0 ? groupId : null;
                _firstStem = candidateStem;
                _firstArrivedAt = arrivedAt;
                usedFallbackCorrelation = _groupId is null;
            }
            else if (_groupId is not null && groupId != 0)
            {
                if (groupId != _groupId.Value)
                {
                    return new CaptureObjectClaim(
                        false,
                        _acceptedObjects,
                        role,
                        roleSignal,
                        false,
                        "group-mismatch"
                    );
                }
            }
            else
            {
                // groupID를 쓸 수 없다. 파일명 stem + 도착 시각 창으로 보조 판정하고
                // 그 사실을 표본에 남긴다 — correlation 근거가 약하다는 것이 결론에 드러나야 한다.
                usedFallbackCorrelation = true;

                var stemMatches =
                    _firstStem is not null
                    && string.Equals(_firstStem, candidateStem, StringComparison.OrdinalIgnoreCase);
                var withinWindow =
                    _firstArrivedAt is not null
                    && arrivedAt - _firstArrivedAt.Value <= _fallbackWindow
                    && arrivedAt >= _firstArrivedAt.Value;

                if (!stemMatches || !withinWindow)
                {
                    return new CaptureObjectClaim(
                        false,
                        _acceptedObjects,
                        role,
                        roleSignal,
                        true,
                        "correlation-failed"
                    );
                }
            }

            // 같은 role이 두 번 오면 뒤엣것은 이 request의 것이 아니다.
            if (role == CaptureObjectRole.Raw && _rawAccepted)
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    usedFallbackCorrelation,
                    "duplicate-raw-object"
                );
            }

            if (role == CaptureObjectRole.Jpeg && _jpegAccepted)
            {
                return new CaptureObjectClaim(
                    false,
                    _acceptedObjects,
                    role,
                    roleSignal,
                    usedFallbackCorrelation,
                    "duplicate-jpeg-object"
                );
            }

            var objectIndex = _acceptedObjects;
            _acceptedObjects++;

            if (role == CaptureObjectRole.Raw)
            {
                _rawAccepted = true;
            }
            else
            {
                _jpegAccepted = true;
            }

            return new CaptureObjectClaim(
                true,
                objectIndex,
                role,
                roleSignal,
                usedFallbackCorrelation,
                null
            );
        }
    }

    /// <summary>
    /// object의 역할을 판정한다.
    /// </summary>
    /// <remarks>
    /// <b>확장자가 비었다고 RAW로 가정하지 않는다.</b> 이전 구현은 확장자가 없으면
    /// <c>.cr3</c>를 기본값으로 넣었는데, object가 둘이 되면 그 로직이 JPEG를 RAW original로
    /// 저장할 수 있다. 판정이 불가능하면 <see cref="CaptureObjectRoleSignal.Unknown"/>을
    /// 돌려주고 호출자가 RAW 경로로 보내지 않게 한다.
    /// </remarks>
    public static (CaptureObjectRole Role, CaptureObjectRoleSignal Signal) ResolveRole(
        string? fileName,
        uint format
    )
    {
        switch (format)
        {
            case EdsImageFormatJpeg:
            case PtpFormatExifJpeg:
                return (CaptureObjectRole.Jpeg, CaptureObjectRoleSignal.SdkFormat);
            case EdsImageFormatCr2:
            case EdsImageFormatCrw:
            case EdsImageFormatRaw:
            case PtpFormatCanonRaw:
            case PtpFormatCanonCrw:
                return (CaptureObjectRole.Raw, CaptureObjectRoleSignal.SdkFormat);
        }

        var extension = Path.GetExtension(fileName ?? string.Empty);

        if (!string.IsNullOrWhiteSpace(extension))
        {
            if (JpegExtensions.Contains(extension, StringComparer.OrdinalIgnoreCase))
            {
                return (CaptureObjectRole.Jpeg, CaptureObjectRoleSignal.FileExtension);
            }

            if (RawExtensions.Contains(extension, StringComparer.OrdinalIgnoreCase))
            {
                return (CaptureObjectRole.Raw, CaptureObjectRoleSignal.FileExtension);
            }
        }

        // 판정 불가. RAW로 가정하지 않는다.
        return (CaptureObjectRole.Unknown, CaptureObjectRoleSignal.Unknown);
    }

    /// <summary>
    /// RAW original로 저장할 확장자를 정한다.
    /// </summary>
    /// <remarks>
    /// SDK가 준 확장자를 그대로 쓰는 것이 원칙이다. 비어 있을 때의 기본값은 <c>.cr2</c>다 —
    /// 승인 하드웨어인 EOS 700D가 실제로 만드는 확장자이며, 이전 기본값 <c>.cr3</c>는
    /// 이 카메라에서 만들어지지 않는 이름이었다.
    /// <b>기존 <c>.cr3</c> 산출물은 그대로 읽힌다</b> — 이 함수는 새로 쓸 이름만 정한다.
    /// </remarks>
    public static string ResolveRawExtension(string? fileName)
    {
        var extension = Path.GetExtension(fileName ?? string.Empty);

        if (
            !string.IsNullOrWhiteSpace(extension)
            && RawExtensions.Contains(extension, StringComparer.OrdinalIgnoreCase)
        )
        {
            return extension;
        }

        return ".cr2";
    }

    private static string StemOf(string? fileName) =>
        Path.GetFileNameWithoutExtension(fileName ?? string.Empty);
}
