import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, join } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const MIN_SIDE_CAR_BYTES = 10 * 1024 * 1024; // 10MB (framework-dependent exes are typically far smaller)

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const appRoot = resolve(scriptDir, '..');
const repoRoot = resolve(appRoot, '..', '..');
const tauriRoot = join(appRoot, 'src-tauri');

const errors = [];
const warnings = [];

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

const configPath = join(tauriRoot, 'tauri.conf.json');
let config;
try {
  config = JSON.parse(readFileSync(configPath, 'utf8'));
} catch (err) {
  errors.push(`Failed to read ${configPath}: ${err instanceof Error ? err.message : String(err)}`);
}

const resources = Array.isArray(config?.bundle?.resources) ? config.bundle.resources : [];
const hasSidecarResource = resources.some((entry) => entry.includes('resources/camera-sidecar'));
const hasNoticesResource = resources.some((entry) => entry.includes('THIRD_PARTY_NOTICES.md'));

if (!hasSidecarResource) {
  errors.push('Missing bundle resource entry for resources/camera-sidecar in tauri.conf.json.');
}
if (!hasNoticesResource) {
  errors.push('Missing bundle resource entry for THIRD_PARTY_NOTICES.md in tauri.conf.json.');
}

const sidecarPath = join(tauriRoot, 'resources', 'camera-sidecar', 'Boothy.CameraSidecar.exe');
if (!existsSync(sidecarPath)) {
  errors.push(`Sidecar binary not found: ${sidecarPath}`);
} else {
  try {
    const size = statSync(sidecarPath).size;
    if (size < MIN_SIDE_CAR_BYTES) {
      const sizeMb = (size / 1024 / 1024).toFixed(2);
      errors.push(
        `Bundled sidecar binary looks too small (${sizeMb}MB). Ensure it is win-x86 self-contained single-file, then run: npm run packaging:prepare`,
      );
    }
  } catch (err) {
    warnings.push(
      `Failed to validate bundled sidecar size: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}
const edsdkPath = join(tauriRoot, 'resources', 'camera-sidecar', 'edsdk', 'EDSDK.dll');
if (!existsSync(edsdkPath)) {
  errors.push(`EDSDK.dll not found next to bundled sidecar resources: ${edsdkPath}`);
} else if (existsSync(sidecarPath)) {
  try {
    const sidecarMachine = readPeMachine(sidecarPath);
    const edsdkMachine = readPeMachine(edsdkPath);
    if (sidecarMachine && edsdkMachine && sidecarMachine !== edsdkMachine) {
      errors.push(
        `Sidecar/EDSDK architecture mismatch: sidecar=${machineLabel(sidecarMachine)} EDSDK=${machineLabel(edsdkMachine)}`,
      );
    }
  } catch (err) {
    warnings.push(
      `Failed to validate sidecar/EDSDK architecture: ${err instanceof Error ? err.message : String(err)}`,
    );
  }
}

const noticesPath = join(repoRoot, 'THIRD_PARTY_NOTICES.md');
if (!existsSync(noticesPath)) {
  errors.push(`THIRD_PARTY_NOTICES.md not found: ${noticesPath}`);
}

const nsisDir = join(tauriRoot, 'target', 'release', 'bundle', 'nsis');
if (!existsSync(nsisDir)) {
  errors.push(`NSIS bundle directory not found. Run npm run tauri build first: ${nsisDir}`);
} else {
  const nsisFiles = readdirSync(nsisDir).filter((file) => file.toLowerCase().endsWith('.exe'));
  if (nsisFiles.length === 0) {
    errors.push(`No NSIS installer found in ${nsisDir}`);
  }
}

const bundledResourcesDir = join(
  tauriRoot,
  'target',
  'release',
  'bundle',
  'resources',
  'camera-sidecar',
);
const verifyBundledResources = process.env.PACKAGING_SMOKE_VERIFY_BUNDLE === '1';
if (existsSync(bundledResourcesDir)) {
  const bundledSidecar = join(bundledResourcesDir, 'Boothy.CameraSidecar.exe');
  if (!existsSync(bundledSidecar)) {
    warnings.push(`Bundled resources missing sidecar binary: ${bundledSidecar}`);
  }
} else if (verifyBundledResources) {
  warnings.push(
    'Bundle resources directory not found; extract installer and set PACKAGING_SMOKE_VERIFY_BUNDLE=1 to verify embedding.',
  );
}

if (warnings.length > 0) {
  console.warn('Packaging smoke check warnings:');
  warnings.forEach((warning) => console.warn(`- ${warning}`));
}

if (errors.length > 0) {
  console.error('Packaging smoke check failed:');
  errors.forEach((error) => console.error(`- ${error}`));
  process.exit(1);
}

console.log('Packaging smoke check passed.');
