using System.Globalization;
using System.Text.Json;
using CanonHelper.Runtime;
using EDSDKLib;

static uint ParseCode(string raw)
{
    var value = raw.Trim();
    return value.StartsWith("0x", StringComparison.OrdinalIgnoreCase)
        ? uint.Parse(value[2..], NumberStyles.HexNumber, CultureInfo.InvariantCulture)
        : uint.Parse(value, CultureInfo.InvariantCulture);
}

static string Hex(uint value) => $"0x{value:X8}";

static Dictionary<string, string?> ParseArgs(string[] rawArgs)
{
    var allowed = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
    {
        "--probe-only", "--out", "--av", "--tv", "--iso",
    };
    var parsed = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
    for (var index = 0; index < rawArgs.Length; index++)
    {
        var key = rawArgs[index];
        if (!allowed.Contains(key))
            throw new ArgumentException($"Unexpected argument: {key}");
        if (parsed.ContainsKey(key))
            throw new ArgumentException($"Duplicate argument: {key}");
        if (key.Equals("--probe-only", StringComparison.OrdinalIgnoreCase))
        {
            parsed[key] = null;
            continue;
        }
        if (index + 1 >= rawArgs.Length)
            throw new ArgumentException($"Missing value for {key}");
        parsed[key] = rawArgs[++index];
    }
    return parsed;
}

static PropertyProbe Probe(IntPtr camera, string name, uint propertyId)
{
    var get = EDSDK.EdsGetPropertyData(camera, propertyId, 0, out uint current);
    if (get != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"{name} read failed: 0x{get:X8}");
    var descriptor = EDSDK.EdsGetPropertyDesc(camera, propertyId, out var desc);
    if (descriptor != EDSDK.EDS_ERR_OK || desc.NumElements <= 0)
        throw new InvalidOperationException($"{name} descriptor unavailable: 0x{descriptor:X8}");
    var supported = desc.PropDesc
        .Take(Math.Min(desc.NumElements, desc.PropDesc?.Length ?? 0))
        .Select(value => unchecked((uint)value))
        .ToArray();
    return new PropertyProbe(name, propertyId, current, supported);
}

static void SetAndVerify(IntPtr camera, PropertyProbe probe, uint requested)
{
    if (!probe.Supported.Contains(requested))
        throw new InvalidOperationException(
            $"{probe.Name} requested value {Hex(requested)} is not an exact descriptor member"
        );
    var set = EDSDK.EdsSetPropertyData(camera, probe.PropertyId, 0, sizeof(uint), requested);
    if (set != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"{probe.Name} set failed: 0x{set:X8}");
    Thread.Sleep(250);
    var readback = EDSDK.EdsGetPropertyData(camera, probe.PropertyId, 0, out uint after);
    if (readback != EDSDK.EDS_ERR_OK || after != requested)
        throw new InvalidOperationException(
            $"{probe.Name} readback mismatch: result=0x{readback:X8}, requested={Hex(requested)}, after={Hex(after)}"
        );
    probe.After = after;
    probe.Requested = requested;
}

var arguments = ParseArgs(args);
if (!arguments.TryGetValue("--out", out var outputRaw) || string.IsNullOrWhiteSpace(outputRaw))
    throw new ArgumentException("--out <absolute-json-path> is required");
if (!Path.IsPathFullyQualified(outputRaw))
    throw new ArgumentException("--out <absolute-json-path> is required");
var outputPath = Path.GetFullPath(outputRaw);
if (!outputPath
    .Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
    .Any(segment => segment.Contains("hv17", StringComparison.OrdinalIgnoreCase)))
    throw new ArgumentException("--out must be inside an explicitly named HV-17 evidence path");
var probeOnly = arguments.ContainsKey("--probe-only");
var requestedArgs = new[] { "--av", "--tv", "--iso" }
    .Where(arguments.ContainsKey)
    .ToArray();
if (probeOnly && requestedArgs.Length > 0)
    throw new ArgumentException("--probe-only cannot be combined with exposure changes");

var initialized = false;
var cameraList = IntPtr.Zero;
var camera = IntPtr.Zero;
var sessionOpen = false;
Dictionary<string, PropertyProbe>? properties = null;
var evidence = new Dictionary<string, object?>
{
    ["schemaVersion"] = "hv17-camera-preflight/v1",
    ["observedAt"] = DateTimeOffset.Now,
    ["probeOnly"] = probeOnly,
    ["success"] = false,
};

try
{
    var init = EDSDK.EdsInitializeSDK();
    if (init != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"EdsInitializeSDK failed: 0x{init:X8}");
    initialized = true;
    var listResult = EDSDK.EdsGetCameraList(out cameraList);
    if (listResult != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"EdsGetCameraList failed: 0x{listResult:X8}");
    var countResult = EDSDK.EdsGetChildCount(cameraList, out var count);
    evidence["cameraCount"] = count;
    if (countResult != EDSDK.EDS_ERR_OK || count != 1)
        throw new InvalidOperationException($"Expected exactly one camera, count={count}, result=0x{countResult:X8}");
    var childResult = EDSDK.EdsGetChildAtIndex(cameraList, 0, out camera);
    if (childResult != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"EdsGetChildAtIndex failed: 0x{childResult:X8}");
    var openResult = EDSDK.EdsOpenSession(camera);
    if (openResult != EDSDK.EDS_ERR_OK)
        throw new InvalidOperationException($"EdsOpenSession failed: 0x{openResult:X8}");
    sessionOpen = true;

    properties = new Dictionary<string, PropertyProbe>
    {
        ["imageQuality"] = Probe(camera, "imageQuality", EDSDK.PropID_ImageQuality),
        ["av"] = Probe(camera, "av", EDSDK.PropID_Av),
        ["tv"] = Probe(camera, "tv", EDSDK.PropID_Tv),
        ["iso"] = Probe(camera, "iso", EDSDK.PropID_ISOSpeed),
    };

    if (!probeOnly)
    {
        var rawOnly = ImageQualityValue.SelectRawOnlyCandidate(
            properties["imageQuality"].Supported.Select(value => unchecked((int)value)).ToArray()
        );
        if (rawOnly is null)
            throw new InvalidOperationException("Camera descriptor exposes no RAW-only ImageQuality value");

        var requested = new Dictionary<string, uint>
        {
            ["imageQuality"] = unchecked((uint)rawOnly.Value),
        };
        if (arguments.TryGetValue("--av", out var av)) requested["av"] = ParseCode(av!);
        if (arguments.TryGetValue("--tv", out var tv)) requested["tv"] = ParseCode(tv!);
        if (arguments.TryGetValue("--iso", out var iso)) requested["iso"] = ParseCode(iso!);

        // 모든 요청을 먼저 검증한다. 중간 설정만 바뀐 채 실패하는 것을 막는다.
        foreach (var (name, value) in requested)
            if (!properties[name].Supported.Contains(value))
                throw new InvalidOperationException($"{name} requested value {Hex(value)} is not an exact descriptor member");
        foreach (var (name, value) in requested)
            SetAndVerify(camera, properties[name], value);
    }

    evidence["success"] = true;
}
catch (Exception error)
{
    evidence["error"] = error.Message;
    Console.Error.WriteLine(error.Message);
    Environment.ExitCode = 1;
}
finally
{
    if (sessionOpen) EDSDK.EdsCloseSession(camera);
    if (camera != IntPtr.Zero) EDSDK.EdsRelease(camera);
    if (cameraList != IntPtr.Zero) EDSDK.EdsRelease(cameraList);
    if (initialized) EDSDK.EdsTerminateSDK();
    if (properties is not null)
        evidence["properties"] = properties.ToDictionary(
            pair => pair.Key,
            pair => pair.Value.ToEvidence()
        );
    Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
    File.WriteAllText(outputPath, JsonSerializer.Serialize(evidence, new JsonSerializerOptions { WriteIndented = true }));
}

sealed class PropertyProbe(string name, uint propertyId, uint before, uint[] supported)
{
    public string Name { get; } = name;
    public uint PropertyId { get; } = propertyId;
    public uint Before { get; } = before;
    public uint[] Supported { get; } = supported;
    public uint? Requested { get; set; }
    public uint? After { get; set; }

    public object ToEvidence() => new
    {
        propertyId = Hex(PropertyId),
        before = Hex(Before),
        requested = Requested is uint requested ? Hex(requested) : null,
        after = After is uint after ? Hex(after) : Hex(Before),
        supported = Supported.Select(Hex).ToArray(),
        changed = After is uint changed && changed != Before,
        verified = Requested is null || Requested == After,
    };

    private static string Hex(uint value) => $"0x{value:X8}";
}
