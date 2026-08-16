using CanonHelper.Runtime;
using Xunit;

namespace CanonHelper.Tests;

/// <summary>
/// Story 7.3 T4: image-quality capability는 <b>가정하지 않고 런타임에서 읽는다.</b>
/// </summary>
/// <remarks>
/// EOS 700D가 RAW+small JPEG를 지원한다는 보장이 없다. descriptor에 없는 조합은
/// 시도 자체를 하지 않고 <c>unsupported-combination</c>으로 기록해야 한다.
/// </remarks>
public sealed class ImageQualityCapabilityTests
{
    // EDSDK.cs의 실제 값들.
    private const int JpegLarge = 0x0010FF0F; // EdsImageQuality_LJ
    private const int JpegLargeFine = 0x0013FF0F; // EdsImageQuality_LJF
    private const int JpegSmall2 = 0x0F10FF0F; // EdsImageQuality_S2J
    private const int RawPlusJpegLarge = 0x00640010; // EdsImageQuality_LRLJ
    private const int RawPlusJpegLargeFine = 0x00640013; // EdsImageQuality_LRLJF
    private const int CrawPlusJpegLargeFine = 0x00630013; // EdsImageQuality_CRLJF
    private const int RawPlusHeifLarge = 0x00640080; // EdsImageQuality_RHEIFL

    [Fact]
    public void Jpeg_only_values_are_not_raw_plus_jpeg()
    {
        Assert.False(ImageQualityValue.IsRawPlusJpeg(JpegLarge));
        Assert.False(ImageQualityValue.IsRawPlusJpeg(JpegLargeFine));
        Assert.False(ImageQualityValue.IsRawPlusJpeg(JpegSmall2));
    }

    [Fact]
    public void Raw_plus_jpeg_values_are_recognised()
    {
        Assert.True(ImageQualityValue.IsRawPlusJpeg(RawPlusJpegLarge));
        Assert.True(ImageQualityValue.IsRawPlusJpeg(RawPlusJpegLargeFine));
        Assert.True(ImageQualityValue.IsRawPlusJpeg(CrawPlusJpegLargeFine));
    }

    /// <summary>RAW+HEIF는 JPEG가 아니다. 이 실험의 대상이 아니므로 골라서는 안 된다.</summary>
    [Fact]
    public void Raw_plus_heif_is_not_treated_as_raw_plus_jpeg()
    {
        Assert.False(ImageQualityValue.IsRawPlusJpeg(RawPlusHeifLarge));
    }

    [Fact]
    public void Jpeg_only_values_report_no_second_image()
    {
        Assert.False(ImageQualityValue.HasSecondImage(JpegLarge));
        Assert.True(ImageQualityValue.HasSecondImage(RawPlusJpegLargeFine));
    }

    // ---------------------------------------------------------------
    // 조합 선택: descriptor 목록 밖으로 나가지 않는다
    // ---------------------------------------------------------------

    /// <summary>
    /// <b>이 테스트가 AC 2의 핵심이다.</b> descriptor에 RAW+JPEG가 없으면 <c>null</c>이어야 하고,
    /// 호출자는 시도조차 하지 않아야 한다.
    /// </summary>
    [Fact]
    public void Returns_null_when_the_descriptor_lists_no_raw_plus_jpeg_combination()
    {
        int[] jpegOnlyCamera = [JpegLarge, JpegLargeFine, JpegSmall2];

        Assert.Null(ImageQualityValue.SelectRawPlusJpegCandidate(jpegOnlyCamera));
    }

    [Fact]
    public void Returns_null_for_an_empty_or_missing_descriptor()
    {
        Assert.Null(ImageQualityValue.SelectRawPlusJpegCandidate(null));
        Assert.Null(ImageQualityValue.SelectRawPlusJpegCandidate(Array.Empty<int>()));
    }

    [Fact]
    public void Picks_a_combination_that_the_descriptor_actually_reported()
    {
        int[] supported = [JpegLarge, RawPlusJpegLargeFine, JpegSmall2];

        var selected = ImageQualityValue.SelectRawPlusJpegCandidate(supported);

        Assert.Equal(RawPlusJpegLargeFine, selected);
        Assert.Contains(selected!.Value, supported);
    }

    /// <summary>
    /// 여러 조합이 있으면 둘째 이미지가 가장 작은 것을 고른다 — 가장 빨리 도착하는 JPEG를
    /// 재는 것이 이 실험의 목적이다. 다만 <b>목록에 있는 것 중에서만</b> 고른다.
    /// </summary>
    [Fact]
    public void Prefers_the_smallest_second_image_among_reported_combinations()
    {
        // 0x00640013 = RAW + Large Fine, 0x0064_0e_13 = RAW + Small1 Fine (가상의 지원 목록)
        const int rawPlusSmall1Fine = 0x00640E13;
        int[] supported = [RawPlusJpegLargeFine, rawPlusSmall1Fine, JpegLarge];

        Assert.Equal(
            rawPlusSmall1Fine,
            ImageQualityValue.SelectRawPlusJpegCandidate(supported)
        );
    }

    /// <summary>
    /// RAW+small을 지원하지 않는 카메라에서 small을 지어내지 않는다.
    /// Large만 보고했다면 Large를 쓰거나 아무것도 쓰지 않는다.
    /// </summary>
    [Fact]
    public void Never_invents_a_raw_plus_small_combination_the_camera_did_not_report()
    {
        int[] largeOnly = [JpegLarge, RawPlusJpegLarge];

        var selected = ImageQualityValue.SelectRawPlusJpegCandidate(largeOnly);

        Assert.Equal(RawPlusJpegLarge, selected);
        Assert.Contains(selected!.Value, largeOnly);
    }

    [Fact]
    public void Selection_is_deterministic_for_the_same_input()
    {
        int[] supported = [CrawPlusJpegLargeFine, RawPlusJpegLargeFine, JpegLarge];

        var first = ImageQualityValue.SelectRawPlusJpegCandidate(supported);
        var second = ImageQualityValue.SelectRawPlusJpegCandidate(supported);

        Assert.Equal(first, second);
    }

    [Fact]
    public void Unavailable_descriptor_reports_no_support()
    {
        var capability = ImageQualityCapability.Unavailable(1_234);

        Assert.False(capability.DescriptorAvailable);
        Assert.False(capability.RawPlusJpegSupported);
        Assert.Null(capability.CurrentValue);
        Assert.Empty(capability.SupportedValues);
        Assert.Equal(1_234, capability.ProbedAtHostMicros);
    }

    [Theory]
    [InlineData("paired")]
    [InlineData(" PAired ")]
    [InlineData("ab")]
    public void Paired_comparison_modes_enable_camera_setting_changes(string mode)
    {
        Assert.True(CanonHelperService.IsPairedComparisonEnabled(mode));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("off")]
    [InlineData("embedded")]
    [InlineData("shell")]
    [InlineData("unknown")]
    public void Other_modes_keep_the_helper_on_the_default_single_object_path(string? mode)
    {
        Assert.False(CanonHelperService.IsPairedComparisonEnabled(mode));
    }
}
