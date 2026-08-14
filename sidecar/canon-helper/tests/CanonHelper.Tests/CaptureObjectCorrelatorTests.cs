using CanonHelper.Runtime;
using CanonHelper.Protocol;
using Xunit;

namespace CanonHelper.Tests;

/// <summary>
/// Story 7.3 T4: multi-object download correlation.
/// </summary>
/// <remarks>
/// 이 테스트들이 지키는 계약은 두 가지다.
/// <list type="number">
///   <item><description>측정 lane이 꺼진 제품 경로의 동작이 바뀌지 않는다 (expected = 1).</description></item>
///   <item><description>모든 transfer object가 하나의 request에 묶이고, 버려지는 object가 없다.</description></item>
/// </list>
/// </remarks>
public sealed class CaptureObjectCorrelatorTests
{
    private static readonly DateTimeOffset Arrived = DateTimeOffset.UnixEpoch.AddHours(1);

    private const uint EdsImageFormatJpeg = 0x00000001;
    private const uint EdsImageFormatCr2 = 0x00000006;
    private const uint UnknownFormat = 0x00000000;

    // ---------------------------------------------------------------
    // 기본값 = 오늘과 같은 동작
    // ---------------------------------------------------------------

    [Fact]
    public void Single_object_request_accepts_only_the_first_object_like_today()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 1);

        var first = correlator.TryClaim(11, "IMG_0001.CR2", EdsImageFormatCr2, Arrived);
        var second = correlator.TryClaim(11, "IMG_0001.JPG", EdsImageFormatJpeg, Arrived);

        Assert.True(first.Accepted);
        Assert.Equal(0, first.ObjectIndex);
        Assert.False(second.Accepted);
        Assert.Equal("request-already-complete", second.RejectReason);
        Assert.True(correlator.IsRequestComplete);
    }

    [Fact]
    public void Single_object_request_ignores_a_jpeg_until_the_raw_object_arrives()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 1);

        var jpeg = correlator.TryClaim(11, "IMG_0001.JPG", EdsImageFormatJpeg, Arrived);
        var raw = correlator.TryClaim(11, "IMG_0001.CR2", EdsImageFormatCr2, Arrived);

        Assert.False(jpeg.Accepted);
        Assert.Equal("unexpected-non-raw-object", jpeg.RejectReason);
        Assert.True(raw.Accepted);
        Assert.Equal(0, raw.ObjectIndex);
    }

    // ---------------------------------------------------------------
    // AC 2: 모든 transfer object가 하나의 request에 pair된다
    // ---------------------------------------------------------------

    [Fact]
    public void Both_objects_pair_into_one_request_by_group_id()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        var raw = correlator.TryClaim(42, "IMG_0002.CR2", EdsImageFormatCr2, Arrived);
        var jpeg = correlator.TryClaim(
            42,
            "IMG_0002.JPG",
            EdsImageFormatJpeg,
            Arrived.AddMilliseconds(180)
        );

        Assert.True(raw.Accepted);
        Assert.True(jpeg.Accepted);
        Assert.Equal(0, raw.ObjectIndex);
        Assert.Equal(1, jpeg.ObjectIndex);
        Assert.Equal(CaptureObjectRole.Raw, raw.Role);
        Assert.Equal(CaptureObjectRole.Jpeg, jpeg.Role);
        Assert.False(raw.UsedFallbackCorrelation);
        Assert.False(jpeg.UsedFallbackCorrelation);
        Assert.True(correlator.IsRequestComplete);
        Assert.Equal(2, correlator.AcceptedObjects);
    }

    [Fact]
    public void Jpeg_may_arrive_before_raw_and_the_order_is_preserved()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        var jpeg = correlator.TryClaim(42, "IMG_0003.JPG", EdsImageFormatJpeg, Arrived);
        var raw = correlator.TryClaim(
            42,
            "IMG_0003.CR2",
            EdsImageFormatCr2,
            Arrived.AddMilliseconds(900)
        );

        Assert.True(jpeg.Accepted);
        Assert.True(raw.Accepted);
        Assert.Equal(0, jpeg.ObjectIndex);
        Assert.Equal(1, raw.ObjectIndex);
    }

    [Fact]
    public void Object_from_a_different_group_is_rejected_with_a_reason()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        correlator.TryClaim(42, "IMG_0004.CR2", EdsImageFormatCr2, Arrived);
        var intruder = correlator.TryClaim(
            43,
            "IMG_0005.JPG",
            EdsImageFormatJpeg,
            Arrived.AddMilliseconds(50)
        );

        Assert.False(intruder.Accepted);
        Assert.Equal("group-mismatch", intruder.RejectReason);
        Assert.False(correlator.IsRequestComplete);
    }

    // ---------------------------------------------------------------
    // groupID가 없을 때의 보조 correlation
    // ---------------------------------------------------------------

    [Fact]
    public void Falls_back_to_filename_stem_and_time_window_when_group_id_is_absent()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        var raw = correlator.TryClaim(0, "IMG_0006.CR2", EdsImageFormatCr2, Arrived);
        var jpeg = correlator.TryClaim(
            0,
            "IMG_0006.JPG",
            EdsImageFormatJpeg,
            Arrived.AddMilliseconds(250)
        );

        Assert.True(raw.Accepted);
        Assert.True(jpeg.Accepted);
        Assert.True(raw.UsedFallbackCorrelation);
        Assert.True(jpeg.UsedFallbackCorrelation);
    }

    [Fact]
    public void Fallback_correlation_rejects_a_different_stem()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        correlator.TryClaim(0, "IMG_0007.CR2", EdsImageFormatCr2, Arrived);
        var other = correlator.TryClaim(
            0,
            "IMG_9999.JPG",
            EdsImageFormatJpeg,
            Arrived.AddMilliseconds(60)
        );

        Assert.False(other.Accepted);
        Assert.Equal("correlation-failed", other.RejectReason);
        Assert.True(other.UsedFallbackCorrelation);
    }

    [Fact]
    public void Fallback_correlation_rejects_an_object_outside_the_arrival_window()
    {
        var correlator = new CaptureObjectCorrelator(
            expectedObjectCount: 2,
            fallbackWindow: TimeSpan.FromSeconds(1)
        );

        correlator.TryClaim(0, "IMG_0008.CR2", EdsImageFormatCr2, Arrived);
        var late = correlator.TryClaim(
            0,
            "IMG_0008.JPG",
            EdsImageFormatJpeg,
            Arrived.AddSeconds(30)
        );

        Assert.False(late.Accepted);
        Assert.Equal("correlation-failed", late.RejectReason);
    }

    [Fact]
    public void Fallback_correlation_rejects_a_blank_filename_stem()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        var claim = correlator.TryClaim(0, string.Empty, EdsImageFormatCr2, Arrived);

        Assert.False(claim.Accepted);
        Assert.Equal("correlation-failed", claim.RejectReason);
        Assert.True(claim.UsedFallbackCorrelation);
        Assert.Equal(0, correlator.AcceptedObjects);
    }

    [Fact]
    public void Duplicate_role_is_rejected_rather_than_silently_replacing()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        correlator.TryClaim(42, "IMG_0009.CR2", EdsImageFormatCr2, Arrived);
        var duplicate = correlator.TryClaim(
            42,
            "IMG_0009.CR2",
            EdsImageFormatCr2,
            Arrived.AddMilliseconds(10)
        );

        Assert.False(duplicate.Accepted);
        Assert.Equal("duplicate-raw-object", duplicate.RejectReason);
    }

    // ---------------------------------------------------------------
    // RAW truth는 JPEG 실패에 종속되지 않는다
    // ---------------------------------------------------------------

    [Fact]
    public void Raw_truth_survives_when_the_jpeg_object_never_arrives()
    {
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);

        correlator.TryClaim(42, "IMG_0010.CR2", EdsImageFormatCr2, Arrived);

        Assert.True(correlator.HasRawObject);
        Assert.False(correlator.HasJpegObject);
        Assert.False(correlator.IsRequestComplete);
    }

    // ---------------------------------------------------------------
    // role 판정: 확장자 기본값이 JPEG를 RAW로 저장하지 않게 한다
    // ---------------------------------------------------------------

    [Fact]
    public void Role_comes_from_the_sdk_format_when_it_is_known()
    {
        var (jpegRole, jpegSignal) = CaptureObjectCorrelator.ResolveRole(
            string.Empty,
            EdsImageFormatJpeg
        );
        var (rawRole, rawSignal) = CaptureObjectCorrelator.ResolveRole("", EdsImageFormatCr2);

        Assert.Equal(CaptureObjectRole.Jpeg, jpegRole);
        Assert.Equal(CaptureObjectRoleSignal.SdkFormat, jpegSignal);
        Assert.Equal(CaptureObjectRole.Raw, rawRole);
        Assert.Equal(CaptureObjectRoleSignal.SdkFormat, rawSignal);
    }

    [Fact]
    public void Role_falls_back_to_the_file_extension_when_the_format_is_unknown()
    {
        var (role, signal) = CaptureObjectCorrelator.ResolveRole("IMG_0011.JPG", UnknownFormat);

        Assert.Equal(CaptureObjectRole.Jpeg, role);
        Assert.Equal(CaptureObjectRoleSignal.FileExtension, signal);
    }

    [Fact]
    public void Unresolvable_role_is_reported_as_unknown_instead_of_being_assumed_raw()
    {
        var (role, signal) = CaptureObjectCorrelator.ResolveRole(string.Empty, UnknownFormat);
        var correlator = new CaptureObjectCorrelator(expectedObjectCount: 2);
        var claim = correlator.TryClaim(0, string.Empty, UnknownFormat, Arrived);

        Assert.Equal(CaptureObjectRole.Unknown, role);
        Assert.Equal(CaptureObjectRoleSignal.Unknown, signal);
        Assert.False(claim.Accepted);
        Assert.Equal("unknown-object-role", claim.RejectReason);
        Assert.Equal(0, correlator.AcceptedObjects);
    }

    // ---------------------------------------------------------------
    // RAW 확장자 기본값
    // ---------------------------------------------------------------

    [Fact]
    public void Raw_extension_uses_what_the_sdk_reported()
    {
        Assert.Equal(".CR2", CaptureObjectCorrelator.ResolveRawExtension("IMG_0012.CR2"));
        Assert.Equal(".cr3", CaptureObjectCorrelator.ResolveRawExtension("IMG_0012.cr3"));
    }

    /// <summary>
    /// 확장자가 비었을 때의 기본값은 승인 하드웨어가 실제로 만드는 <c>.cr2</c>다.
    /// 이전 기본값 <c>.cr3</c>는 EOS 700D가 만들지 않는 이름이었다.
    /// </summary>
    [Fact]
    public void Raw_extension_defaults_to_the_extension_the_approved_camera_actually_writes()
    {
        Assert.Equal(".cr2", CaptureObjectCorrelator.ResolveRawExtension(string.Empty));
        Assert.Equal(".cr2", CaptureObjectCorrelator.ResolveRawExtension(null));
    }

    /// <summary>JPEG 확장자가 RAW original 이름으로 새어 나가지 않는다.</summary>
    [Fact]
    public void Raw_extension_never_returns_a_jpeg_extension()
    {
        Assert.Equal(".cr2", CaptureObjectCorrelator.ResolveRawExtension("IMG_0013.JPG"));
    }

    [Fact]
    public void Capture_id_is_fixed_before_either_transfer_object_arrives()
    {
        var request = new CaptureRequestMessage(
            CanonHelperSchemas.CaptureRequest,
            "request-capture",
            "session_1",
            "request_1",
            "2026-08-12T10:15:30Z",
            "preset_1",
            "1"
        );
        var context = new CurrentCaptureContext(
            new SessionPaths(Path.GetTempPath(), "session_1"),
            request,
            null,
            null,
            null,
            "capture_1",
            expectedObjectCount: 2
        );

        Assert.Equal("capture_1", context.CaptureId);
    }
}
