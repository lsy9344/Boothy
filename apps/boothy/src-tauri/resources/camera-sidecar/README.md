# Camera Sidecar Resources

This directory contains runtime resources bundled with the Boothy installer.

## Contents (When Built)

### Camera Sidecar Binary

- `camera-sidecar.exe` - Headless camera service for Canon EDSDK integration
- Copied from `apps/camera-sidecar/bin/Release/` during build

### Canon EDSDK DLLs (Internal Deployments Only)

⚠️ **EDSDK DLLs are NOT included in source control and require entitlement**

For internal deployments, place Canon EDSDK DLLs in `edsdk/` subdirectory:

```
edsdk/
├── EDSDK.dll
├── EdsImage.dll
└── README.txt (version information)
```

**Bitness Requirements** (MVP):
- x86 (32-bit) DLLs required
- Must match sidecar binary bitness
- Tested on Windows 11 x64 (WoW64 compatibility layer)

## Build-Time Setup

### For Development (Without EDSDK)

1. Build camera sidecar
2. Copy sidecar binary to this directory
3. Boothy will detect EDSDK absence and show appropriate error message

### For Internal Deployment (With EDSDK)

1. **Verify entitlement record exists**:
   ```bash
   cat ../../../docs/compliance/canon-edsdk-entitlement.md
   ```

2. **Copy sidecar binary**:
   ```bash
   cp ../../camera-sidecar/bin/Release/camera-sidecar.exe ./
   ```

3. **Copy EDSDK DLLs** (from Canon SDK download):
   ```bash
   mkdir -p edsdk
   cp /path/to/canon-sdk/Dll/* ./edsdk/
   ```

4. **Verify bitness**:
   ```bash
   # Check DLL architecture (should show "x86" for MVP)
   dumpbin /headers edsdk/EDSDK.dll | findstr "machine"
   ```

## Runtime Behavior

### Bundled Location

After installation, this directory is extracted to:
```
C:\Program Files\Boothy\resources\camera-sidecar\
```

### Sidecar Startup

The Tauri backend (`src-tauri/src/camera/ipc_client.rs`) will:
1. Locate sidecar binary in resources
2. Start sidecar process with Named Pipe IPC
3. Monitor sidecar health via IPC heartbeat
4. Restart sidecar if it crashes

### Error Handling

If EDSDK DLLs are missing:
- Sidecar will fail to start
- Error logged to `%APPDATA%\Boothy\logs\`
- Customer-safe message shown in UI: "Camera not detected. Please check connection."
- Admin diagnostic: "EDSDK DLLs not found or camera not connected"

## Development Notes

- This directory is bundled during `tauri build`
- Contents are embedded in the installer
- Sidecar can be updated without rebuilding main Boothy app (just rebuild installer)
- EDSDK DLLs are dynamically loaded by sidecar at runtime

## References

- Packaging Guide: `../../../docs/packaging-guide.md`
- Canon EDSDK Entitlement: `../../../docs/compliance/canon-edsdk-entitlement.md`
- ADR-003: `../../../docs/decisions/adr-003-canon-edsdk-bundling.md`
