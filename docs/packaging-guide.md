# Boothy Packaging & Deployment Guide

## Overview

This document describes the packaging and deployment process for Boothy, including NSIS installer creation, sidecar bundling, and EDSDK DLL integration for internal deployments.

## Prerequisites

### Development Environment

- **Windows 11** (x64) with development toolchain
- **Node.js** 18+ and npm
- **Rust** 1.92+ with cargo
- **Tauri CLI** 2.9+
- **NSIS** (Nullsoft Scriptable Install System) - installed automatically by Tauri

### Required Files

- Boothy application source (`apps/boothy`)
- Camera sidecar binary (`apps/camera-sidecar`)
- Canon EDSDK DLLs (for internal deployments only - see Canon EDSDK section)
- Third-party notices (`THIRD_PARTY_NOTICES.md`)

## Build Process

### 1. Frontend Build

```bash
cd apps/boothy
npm install
npm run build
```

This produces the frontend bundle in `apps/boothy/dist/`.

### 2. Backend Build

```bash
cd apps/boothy/src-tauri
cargo build --release
```

This produces the Rust backend binary in `apps/boothy/src-tauri/target/release/boothy.exe`.

### 3. Sidecar Build

```bash
cd apps/camera-sidecar
# For .NET sidecar
dotnet build -c Release
```

This produces the sidecar binary that will be bundled with Boothy.

### 4. Installer Creation

```bash
cd apps/boothy
npm run tauri build
```

This creates the NSIS installer in `apps/boothy/src-tauri/target/release/bundle/nsis/`.

## Installer Contents

The NSIS installer includes:

1. **Boothy.exe** - Main application binary
2. **Frontend assets** - React UI bundle
3. **Camera sidecar** - Bundled in `resources/camera-sidecar/`
4. **Required DLLs** - Runtime dependencies
5. **THIRD_PARTY_NOTICES.md** - License notices
6. **Uninstaller** - For clean removal

## Installation Paths

### Program Files

- **Installation Root**: `C:\Program Files\Boothy\`
- **Sidecar Location**: `C:\Program Files\Boothy\resources\camera-sidecar\`
- **EDSDK DLLs** (internal only): `C:\Program Files\Boothy\resources\camera-sidecar\edsdk\`

### User Data (Rollback-Safe)

- **Session Photos**: `%USERPROFILE%\Pictures\dabi_shoot\<session-name>\`
  - `Raw/` - Original RAW files from camera
  - `Jpg/` - Exported JPEGs
  - `.rrdata` files - Image metadata and adjustments
- **Settings**: `%APPDATA%\Boothy\settings\`
- **Logs**: `%APPDATA%\Boothy\logs\`

**CRITICAL**: Session photos are stored separately from program files to ensure no data loss during upgrades or rollbacks.

## Canon EDSDK Bundling (Internal Deployments Only)

### Legal Requirements

⚠️ **IMPORTANT**: Canon EDSDK DLLs can only be bundled for **internal deployments** and require:
1. Valid Canon Developer Program membership
2. Signed EDSDK SDK Agreement
3. Completed entitlement record in `docs/compliance/canon-edsdk-entitlement.md`

### Pre-Packaging Checklist

Before creating any installer that includes EDSDK DLLs, verify:

- [ ] `docs/compliance/canon-edsdk-entitlement.md` exists and is complete
- [ ] Owner confirmation section includes name, date, and Canon account details
- [ ] Technical details section specifies DLL versions and bitness
- [ ] Deployment scope is clearly "internal only"

### DLL Packaging Steps

1. **Verify Entitlement**
   ```bash
   # Check that entitlement file exists
   test -f docs/compliance/canon-edsdk-entitlement.md && echo "OK" || echo "MISSING"
   ```

2. **Copy EDSDK DLLs** (x86 for MVP)
   ```
   apps/boothy/src-tauri/resources/camera-sidecar/edsdk/
   ├── EDSDK.dll
   ├── EdsImage.dll
   └── README.txt (version info)
   ```

3. **Bitness Compatibility**
   - MVP: x86 sidecar + x86 DLLs on Win11 x64
   - Future: Consider x64 sidecar for performance

### Distribution Restrictions

- ❌ **DO NOT** distribute EDSDK DLLs in public releases
- ❌ **DO NOT** include EDSDK DLLs in source repositories
- ✅ **DO** bundle for internal/employee deployments only
- ✅ **DO** maintain entitlement records

## AGPL Compliance

Boothy is licensed under **AGPL-3.0**. Each distributed build must include:

1. **Binary**: The compiled Boothy installer
2. **Notices**: `THIRD_PARTY_NOTICES.md` with all dependencies
3. **Source Access**: Link to corresponding source code

### Source Code Packaging

```bash
# Create source archive matching binary release
git archive --format=zip --prefix=boothy-<version>/ HEAD > boothy-<version>-source.zip
```

### Distribution Checklist

- [ ] Installer includes `THIRD_PARTY_NOTICES.md`
- [ ] Source archive created and published
- [ ] Release notes include link to source code
- [ ] All third-party licenses documented

## Rollback Strategy

### Rollback-Safe Design

Boothy is designed to allow rollback to previous versions without data loss:

1. **Session photos** are in `%USERPROFILE%\Pictures\` (separate from program files)
2. **Settings schema** is append-only (old versions ignore new fields)
3. **Metadata format** (`.rrdata`) is backward-compatible

### Rollback Procedure

1. Uninstall current version via Control Panel
2. Install previous version from archived installer
3. Verify session folders remain intact
4. Confirm existing photos are browsable and exportable

### Testing Rollback

Before releasing version N:
1. Install version N-1
2. Create session and capture photos
3. Install version N
4. Verify photos are still accessible
5. Uninstall N, reinstall N-1
6. Verify photos are still accessible

## Build Automation

### CI/CD Integration

```yaml
# Example GitHub Actions workflow
name: Build Installer
on: [push, pull_request]
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v2
      - name: Setup Node
        uses: actions/setup-node@v2
        with:
          node-version: '18'
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.92
      - name: Install dependencies
        run: cd apps/boothy && npm install
      - name: Build installer
        run: cd apps/boothy && npm run tauri build
      - name: Upload installer
        uses: actions/upload-artifact@v2
        with:
          name: boothy-installer
          path: apps/boothy/src-tauri/target/release/bundle/nsis/*.exe
```

## Troubleshooting

### Build Issues

**Error**: `resource path 'resources' doesn't exist`
- **Solution**: Create `apps/boothy/src-tauri/resources/` directory

**Error**: `EDSDK.dll not found`
- **Solution**: Verify EDSDK DLLs are in `resources/camera-sidecar/edsdk/`
- **Check**: Ensure bitness matches (x86 sidecar needs x86 DLLs)

### Installer Issues

**Error**: Installer fails to start sidecar
- **Solution**: Check that sidecar binary is bundled in resources
- **Check**: Verify sidecar has correct permissions

## References

- ADR-002: AGPL Compliance (`docs/decisions/adr-002-agpl-compliance.md`)
- ADR-003: Canon EDSDK Bundling (`docs/decisions/adr-003-canon-edsdk-bundling.md`)
- Canon EDSDK Entitlement (`docs/compliance/canon-edsdk-entitlement.md`)
