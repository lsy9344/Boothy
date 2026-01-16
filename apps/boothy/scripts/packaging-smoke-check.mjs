import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const appRoot = resolve(scriptDir, '..');
const repoRoot = resolve(appRoot, '..', '..');
const tauriRoot = join(appRoot, 'src-tauri');

const errors = [];
const warnings = [];

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
