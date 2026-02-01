import { cpSync, existsSync, mkdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const MIN_SIDE_CAR_BYTES = 10 * 1024 * 1024; // 10MB (framework-dependent exes are typically far smaller)

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const appRoot = resolve(scriptDir, '..');
const repoRoot = resolve(appRoot, '..', '..');

const destDir = join(appRoot, 'src-tauri', 'resources', 'camera-sidecar');
const destExe = join(destDir, 'Boothy.CameraSidecar.exe');

const MACHINE_X86 = 0x014c;
const MACHINE_X64 = 0x8664;

function readPeMachine(filePath) {
  const buffer = readFileSync(filePath);
  if (buffer.length < 0x40) {
    return null;
  }
  const peOffset = buffer.readUInt32LE(0x3c);
  if (peOffset + 6 >= buffer.length) {
    return null;
  }
  const signature = buffer.readUInt32LE(peOffset);
  // 'PE\0\0' little-endian
  if (signature !== 0x00004550) {
    return null;
  }
  return buffer.readUInt16LE(peOffset + 4);
}

function machineLabel(machine) {
  if (machine === MACHINE_X86) return 'x86';
  if (machine === MACHINE_X64) return 'x64';
  if (typeof machine === 'number') return `0x${machine.toString(16)}`;
  return 'unknown';
}

const candidates = [
  {
    label: 'Release publish (win-x86)',
    kind: 'release',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Release',
      'net8.0',
      'win-x86',
      'publish',
      'Boothy.CameraSidecar.exe',
    ),
  },
  {
    label: 'Release',
    kind: 'release',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Release',
      'net8.0',
      'Boothy.CameraSidecar.exe',
    ),
  },
  {
    label: 'Release (flat)',
    kind: 'release',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Release',
      'Boothy.CameraSidecar.exe',
    ),
  },
  {
    label: 'Debug publish (win-x86)',
    kind: 'debug',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Debug',
      'net8.0',
      'win-x86',
      'publish',
      'Boothy.CameraSidecar.exe',
    ),
  },
  {
    label: 'Debug',
    kind: 'debug',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Debug',
      'net8.0',
      'Boothy.CameraSidecar.exe',
    ),
  },
  {
    label: 'Debug (flat)',
    kind: 'debug',
    exe: join(
      repoRoot,
      'apps',
      'camera-sidecar',
      'bin',
      'Debug',
      'Boothy.CameraSidecar.exe',
    ),
  },
];

const selected = candidates.find((candidate) => existsSync(candidate.exe));

if (!selected) {
  console.error('Camera sidecar build output not found. Build it first:');
  console.error('  cd apps/camera-sidecar');
  console.error('  dotnet publish -c Release -r win-x86 --self-contained true');
  process.exit(1);
}

const sidecarStat = statSync(selected.exe);
if (sidecarStat.size < MIN_SIDE_CAR_BYTES) {
  const sizeMb = (sidecarStat.size / 1024 / 1024).toFixed(2);
  console.error(`Sidecar binary looks too small (${sizeMb}MB). This is likely framework-dependent and will fail on PCs without the x86 .NET runtime.`);
  console.error(`- Sidecar: ${selected.exe}`);
  console.error('Build the sidecar as self-contained single-file (win-x86), then re-run:');
  console.error('  cd apps/camera-sidecar');
  console.error('  dotnet publish -c Release -r win-x86 --self-contained true /p:PublishSingleFile=true /p:IncludeNativeLibrariesForSelfExtract=true');
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });

const sourceDir = dirname(selected.exe);
const sourceEdsdk = join(sourceDir, 'edsdk', 'EDSDK.dll');

if (!existsSync(sourceEdsdk)) {
  console.error(`EDSDK.dll not found next to sidecar output: ${sourceEdsdk}`);
  console.error('Expected the sidecar build to bundle Canon EDSDK under an edsdk/ folder.');
  console.error('If you are building locally, ensure the repo has:');
  console.error('  apps/boothy/src-tauri/resources/camera-sidecar/edsdk/EDSDK.dll');
  console.error('Then run:');
  console.error('  cd apps/camera-sidecar');
  console.error('  dotnet publish -c Release -r win-x86 --self-contained true');
  process.exit(1);
}

const sidecarMachine = readPeMachine(selected.exe);
const edsdkMachine = readPeMachine(sourceEdsdk);
if (sidecarMachine && edsdkMachine && sidecarMachine !== edsdkMachine) {
  console.error(
    `Sidecar/EDSDK architecture mismatch: sidecar=${machineLabel(sidecarMachine)} EDSDK=${machineLabel(edsdkMachine)}`,
  );
  console.error(`- Sidecar: ${selected.exe}`);
  console.error(`- EDSDK:   ${sourceEdsdk}`);
  console.error('Fix: build the sidecar for the same architecture as the bundled Canon EDSDK (x86).');
  console.error('  cd apps/camera-sidecar');
  console.error('  dotnet publish -c Release -r win-x86 --self-contained true');
  process.exit(1);
}

let shouldCopy = true;
if (existsSync(destExe)) {
  try {
    shouldCopy = statSync(selected.exe).mtimeMs > statSync(destExe).mtimeMs;
  } catch {
    shouldCopy = true;
  }
}

if (shouldCopy) {
  // Copy only the sidecar binary (single-file) and required EDSDK DLLs.
  // Copying the entire publish directory can fail on Windows if old resource files are locked.
  cpSync(selected.exe, destExe, { force: true });

  const sourceEdsdkDir = join(sourceDir, 'edsdk');
  const destEdsdkDir = join(destDir, 'edsdk');
  if (!existsSync(destEdsdkDir)) {
    cpSync(sourceEdsdkDir, destEdsdkDir, { recursive: true, force: true });
  }

  console.log(`Copied sidecar binary from ${selected.exe} to ${destExe}`);
} else {
  console.log('Sidecar resources already up to date.');
}

if (selected.kind === 'debug') {
  console.warn('Using Debug sidecar build; run dotnet build -c Release for packaging.');
}
