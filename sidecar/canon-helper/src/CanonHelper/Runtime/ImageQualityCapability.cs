namespace CanonHelper.Runtime;

/// <summary>
/// 카메라가 스스로 보고한 image-quality capability descriptor와 그 해석.
/// </summary>
/// <remarks>
/// Story 7.3의 고정 결정: <b>지원 여부를 가정하지 않는다.</b>
/// <c>EdsGetPropertyDesc(PropID_ImageQuality)</c>의 목록이 유일한 truth이며, 목록에 없는
/// 조합은 <i>시도조차 하지 않고</i> <c>unsupported-combination</c>으로 기록한다.
/// 시도해서 실패한 것과 지원되지 않아 시도하지 않은 것은 evidence에서 다른 결과다.
/// <para>
/// EOS 700D가 RAW+small JPEG를 지원한다는 보장은 없다. 그래서 이 타입은 어떤 조합도
/// 하드코딩하지 않고 descriptor가 준 목록 안에서만 고른다.
/// </para>
/// </remarks>
internal sealed record ImageQualityCapability(
    bool DescriptorAvailable,
    int? CurrentValue,
    IReadOnlyList<int> SupportedValues,
    bool RawPlusJpegSupported,
    long ProbedAtHostMicros
)
{
    public static ImageQualityCapability Unavailable(long probedAtHostMicros) =>
        new(false, null, Array.Empty<int>(), false, probedAtHostMicros);
}

/// <summary>
/// Canon <c>EdsImageQuality</c> 32비트 값의 순수 해석기.
/// </summary>
/// <remarks>
/// 값은 네 바이트로 나뉜다.
/// <list type="bullet">
///   <item><description>bits 24-31 — 첫 이미지 크기</description></item>
///   <item><description>bits 16-23 — 첫 이미지 형식/품질</description></item>
///   <item><description>bits 8-15 — 둘째 이미지 크기</description></item>
///   <item><description>bits 0-7 — 둘째 이미지 형식/품질</description></item>
/// </list>
/// 예: <c>EdsImageQuality_LJ = 0x0010ff0f</c>는 둘째 이미지가 없음(<c>0xff</c>/<c>0x0f</c>)이고,
/// <c>EdsImageQuality_LRLJF = 0x00640013</c>는 RAW(<c>0x64</c>) + Large Fine JPEG(<c>0x13</c>)이다.
/// </remarks>
internal static class ImageQualityValue
{
    /// <summary>둘째 이미지가 없음을 뜻하는 크기 코드.</summary>
    public const int NoSecondImageSize = 0xFF;

    /// <summary>둘째 이미지가 없음을 뜻하는 형식 코드.</summary>
    public const int NoSecondImageFormat = 0x0F;

    private const int RawFormat = 0x64;
    private const int CompressedRawFormat = 0x63;

    private const int JpegFormat = 0x10;
    private const int JpegFineFormat = 0x13;
    private const int JpegNormalFormat = 0x12;

    public static int FirstImageSize(int value) => (value >> 24) & 0xFF;

    public static int FirstImageFormat(int value) => (value >> 16) & 0xFF;

    public static int SecondImageSize(int value) => (value >> 8) & 0xFF;

    public static int SecondImageFormat(int value) => value & 0xFF;

    public static bool IsRawFormat(int format) =>
        format == RawFormat || format == CompressedRawFormat;

    public static bool IsJpegFormat(int format) =>
        format == JpegFormat || format == JpegFineFormat || format == JpegNormalFormat;

    public static bool HasSecondImage(int value) =>
        SecondImageFormat(value) != NoSecondImageFormat
        && SecondImageSize(value) != NoSecondImageSize;

    /// <summary>이 값이 RAW와 JPEG를 함께 만드는 조합인가.</summary>
    public static bool IsRawPlusJpeg(int value) =>
        IsRawFormat(FirstImageFormat(value))
        && HasSecondImage(value)
        && IsJpegFormat(SecondImageFormat(value));

    /// <summary>
    /// descriptor가 실제로 보고한 목록 안에서 RAW+JPEG 조합을 고른다.
    /// </summary>
    /// <remarks>
    /// <b>목록에 없으면 <c>null</c>을 돌려준다.</b> 대체 조합을 지어내지 않는다.
    /// 여러 개가 있으면 <i>둘째 이미지가 가장 작은</i> 것을 고른다 — 이 실험의 목적이
    /// 가장 빨리 도착하는 JPEG를 재는 것이기 때문이다. 크기 코드는 값이 클수록 작은 이미지다
    /// (Large=0x00, Middle=0x01, Small1=0x0e, Small2=0x0f).
    /// 동률이면 값 자체로 정렬해 선택이 결정적이게 만든다.
    /// </remarks>
    public static int? SelectRawPlusJpegCandidate(IReadOnlyList<int>? supportedValues)
    {
        if (supportedValues is null || supportedValues.Count == 0)
        {
            return null;
        }

        int? best = null;
        var bestSecondSize = int.MinValue;

        foreach (var value in supportedValues)
        {
            if (!IsRawPlusJpeg(value))
            {
                continue;
            }

            var secondSize = SecondImageSize(value);

            if (
                best is null
                || secondSize > bestSecondSize
                || (secondSize == bestSecondSize && value < best.Value)
            )
            {
                best = value;
                bestSecondSize = secondSize;
            }
        }

        return best;
    }
}
