using System.Text.Json;
using EDSDKLib;

static string Hex(int value) => $"0x{unchecked((uint)value):X8}";
static bool IsRawOnly(int value)
{
    var firstFormat = (value >> 16) & 0xFF;
    var secondSize = (value >> 8) & 0xFF;
    var secondFormat = value & 0xFF;
    return (firstFormat is 0x64 or 0x63) && secondSize == 0xFF && secondFormat == 0x0F;
}

var initialized = false;
var cameraList = IntPtr.Zero;
var camera = IntPtr.Zero;
var sessionOpen = false;

try
{
    var init = EDSDK.EdsInitializeSDK();
    if (init != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"EdsInitializeSDK failed: 0x{init:X8}");
    initialized = true;

    var listResult = EDSDK.EdsGetCameraList(out cameraList);
    if (listResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"EdsGetCameraList failed: 0x{listResult:X8}");
    var countResult = EDSDK.EdsGetChildCount(cameraList, out var count);
    if (countResult != EDSDK.EDS_ERR_OK || count != 1) throw new InvalidOperationException($"Expected exactly one camera, count={count}, result=0x{countResult:X8}");
    var childResult = EDSDK.EdsGetChildAtIndex(cameraList, 0, out camera);
    if (childResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"EdsGetChildAtIndex failed: 0x{childResult:X8}");
    var openResult = EDSDK.EdsOpenSession(camera);
    if (openResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"EdsOpenSession failed: 0x{openResult:X8}");
    sessionOpen = true;

    var currentResult = EDSDK.EdsGetPropertyData(camera, EDSDK.PropID_ImageQuality, 0, out uint current);
    if (currentResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"Current ImageQuality unavailable: 0x{currentResult:X8}");
    var descResult = EDSDK.EdsGetPropertyDesc(camera, EDSDK.PropID_ImageQuality, out var desc);
    if (descResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"ImageQuality descriptor unavailable: 0x{descResult:X8}");

    var supported = desc.PropDesc.Take(Math.Min(desc.NumElements, desc.PropDesc.Length)).ToArray();
    var selected = supported
        .Where(IsRawOnly)
        .OrderBy(value => ((value >> 16) & 0xFF) == 0x64 ? 0 : 1)
        .ThenBy(value => (value >> 24) & 0xFF)
        .ThenBy(value => value)
        .Cast<int?>()
        .FirstOrDefault();
    if (selected is null) throw new InvalidOperationException("Camera descriptor exposes no RAW-only ImageQuality value");

    var setResult = EDSDK.EdsSetPropertyData(camera, EDSDK.PropID_ImageQuality, 0, sizeof(uint), unchecked((uint)selected.Value));
    if (setResult != EDSDK.EDS_ERR_OK) throw new InvalidOperationException($"Set ImageQuality failed: 0x{setResult:X8}");
    Thread.Sleep(500);
    var verifyResult = EDSDK.EdsGetPropertyData(camera, EDSDK.PropID_ImageQuality, 0, out uint after);
    if (verifyResult != EDSDK.EDS_ERR_OK || after != unchecked((uint)selected.Value))
        throw new InvalidOperationException($"ImageQuality verification failed: result=0x{verifyResult:X8}, after=0x{after:X8}");

    Console.WriteLine(JsonSerializer.Serialize(new
    {
        schemaVersion = "hv17-camera-raw-setup/v1",
        observedAt = DateTimeOffset.Now,
        cameraCount = count,
        before = Hex(unchecked((int)current)),
        selected = Hex(selected.Value),
        after = Hex(unchecked((int)after)),
        supported = supported.Select(Hex).ToArray(),
        verified = true
    }, new JsonSerializerOptions { WriteIndented = true }));
}
finally
{
    if (sessionOpen) EDSDK.EdsCloseSession(camera);
    if (camera != IntPtr.Zero) EDSDK.EdsRelease(camera);
    if (cameraList != IntPtr.Zero) EDSDK.EdsRelease(cameraList);
    if (initialized) EDSDK.EdsTerminateSDK();
}
