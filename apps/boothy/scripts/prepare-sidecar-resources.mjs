import { cpSync, existsSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const appRoot = resolve(scriptDir, '..');
const repoRoot = resolve(appRoot, '..', '..');

const destDir = join(appRoot, 'src-tauri', 'resources', 'camera-sidecar');
const destExe = join(destDir, 'Boothy.CameraSidecar.exe');

const candidates = [
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
  console.error('  dotnet build -c Release');
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });

const sourceDir = dirname(selected.exe);

let shouldCopy = true;
if (existsSync(destExe)) {
  try {
    shouldCopy = statSync(selected.exe).mtimeMs > statSync(destExe).mtimeMs;
  } catch {
    shouldCopy = true;
  }
}

if (shouldCopy) {
  cpSync(sourceDir, destDir, { recursive: true, force: true });
  console.log(`Copied sidecar resources from ${sourceDir} to ${destDir}`);
} else {
  console.log('Sidecar resources already up to date.');
}

if (selected.kind === 'debug') {
  console.warn('Using Debug sidecar build; run dotnet build -c Release for packaging.');
}
