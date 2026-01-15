# Rollback Safety & Data Handling

## Overview

Boothy is designed to support **rollback to previous versions without data loss**. This document describes the architecture decisions and implementation patterns that ensure session photos remain intact across version upgrades and downgrades.

## Core Principle: Data Separation

**Session photos and user data are stored separately from program files.**

### File System Layout

```
C:\Program Files\Boothy\              # Program files (replaced on upgrade)
├── Boothy.exe
├── resources/
│   └── camera-sidecar/
└── uninstall.exe

%USERPROFILE%\Pictures\dabi_shoot\    # User data (persistent across versions)
├── session-2026-01-14-morning/
│   ├── Raw/                          # Original RAW files
│   │   ├── IMG_0001.CR2
│   │   └── IMG_0001.CR2.rrdata       # Metadata + adjustments
│   └── Jpg/                          # Exported JPEGs
│       └── IMG_0001.jpg
└── session-2026-01-14-afternoon/
    ├── Raw/
    └── Jpg/

%APPDATA%\Boothy\                     # Settings (persistent, versioned)
├── settings/
│   └── config.json                   # App settings (schema versioned)
└── logs/
    └── boothy-20260114.log           # Diagnostic logs
```

### Why This Matters

When you:
- **Upgrade** from version N to N+1: Session photos remain in Pictures, new version can access them
- **Rollback** from version N+1 to N: Session photos remain in Pictures, old version can still access them
- **Uninstall**: Program files are removed, but session photos remain until manually deleted

## Schema Versioning Strategy

### Append-Only Metadata

All metadata stored in `.rrdata` files follows an **append-only** pattern:
- **New keys can be added** in newer versions
- **Existing keys cannot be removed or renamed**
- **Old versions ignore unknown keys** (forward compatibility)

### Example: `.rrdata` Schema Evolution

**Version 1.0** (MVP):
```json
{
  "version": "1.0",
  "adjustments": {
    "exposure": 0.5,
    "contrast": 0.2,
    "boothy": {
      "presetId": "preset-001",
      "presetName": "Bright",
      "appliedAt": "2026-01-14T10:30:00Z"
    }
  }
}
```

**Version 1.1** (hypothetical future):
```json
{
  "version": "1.1",
  "adjustments": {
    "exposure": 0.5,
    "contrast": 0.2,
    "boothy": {
      "presetId": "preset-001",
      "presetName": "Bright",
      "appliedAt": "2026-01-14T10:30:00Z",
      "captureCorrelationId": "corr-12345"  // NEW KEY (v1.1)
    }
  },
  "ai": {                                     // NEW SECTION (v1.1)
    "autoEnhance": true
  }
}
```

**Rollback Behavior**:
- Version 1.0 opens file, ignores `captureCorrelationId` and `ai` section
- Core functionality (preset application, export) continues working
- User can still browse, preview, and export images

### Settings Schema

`%APPDATA%\Boothy\settings\config.json` follows the same pattern:

```json
{
  "schemaVersion": 1,
  "session": {
    "lastSessionName": "session-2026-01-14-morning"
  },
  "ui": {
    "theme": "dark"
  }
}
```

**Rules**:
- `schemaVersion` field indicates settings format
- New fields can be added with safe defaults
- Old versions use defaults for unknown fields
- **Never change the meaning of existing fields**

## Rollback Testing Procedure

Before releasing version N, perform rollback testing:

### Test Steps

1. **Install Version N-1**
   ```bash
   # Install previous version
   boothy-setup-v1.0.0.exe
   ```

2. **Create Session and Capture Photos**
   - Launch Boothy
   - Create session "rollback-test"
   - Capture 3+ photos (or use test RAW files)
   - Apply preset to at least one photo
   - Export at least one JPEG

3. **Verify Session Folder Structure**
   ```bash
   # Check session folder exists with photos
   ls "%USERPROFILE%\Pictures\dabi_shoot\rollback-test"
   # Should show: Raw/ and Jpg/ directories
   ```

4. **Install Version N (New Version)**
   ```bash
   # Installer should offer upgrade
   boothy-setup-v1.1.0.exe
   ```

5. **Verify Upgrade Compatibility**
   - Launch new version
   - Confirm session "rollback-test" is still accessible
   - Verify photos are visible in gallery
   - Confirm presets still applied correctly
   - Export another JPEG

6. **Rollback to Version N-1**
   ```bash
   # Uninstall new version
   control appwiz.cpl  # Uninstall "Boothy"

   # Reinstall old version
   boothy-setup-v1.0.0.exe
   ```

7. **Verify Rollback Safety** ✅
   - Launch old version
   - **CRITICAL**: Session "rollback-test" must still be accessible
   - All photos (including those captured on N) must be visible
   - Photos must be browsable and exportable
   - Existing exports must remain intact

### Success Criteria

✅ **Rollback is successful if**:
- All session folders remain intact
- All RAW files are present in `Raw/` directories
- All exported JPEGs are present in `Jpg/` directories
- Metadata (`.rrdata`) files can be read (with unknown keys ignored)
- Core workflow (browse, preview, export) functions correctly

❌ **Rollback fails if**:
- Session folders are missing
- Photos are inaccessible or corrupted
- Application crashes on startup or session load
- Export functionality breaks

## Developer Guidelines

### When Adding New Features

1. **User Data Changes**:
   - Add new keys to `.rrdata` adjustments (don't modify existing)
   - Increment schema version if structure changes
   - Provide sensible defaults for missing keys

2. **Settings Changes**:
   - Add new fields to `config.json` with defaults
   - Increment `schemaVersion` if necessary
   - Test that old versions ignore new fields gracefully

3. **Database Changes** (Future):
   - Use migrations (never destructive)
   - Schema version table
   - Downgrade migrations for common scenarios

### Code Review Checklist

When reviewing PRs that touch data handling:

- [ ] Are session photos still stored in `%USERPROFILE%\Pictures\dabi_shoot\`?
- [ ] Are new metadata fields added (not replacing existing ones)?
- [ ] Is schema version incremented if structure changes?
- [ ] Are defaults provided for new settings?
- [ ] Has rollback testing been performed?

## Known Limitations

### Schema Breaking Changes

If a **breaking change** is unavoidable (e.g., security fix), document:
1. Minimum rollback-safe version (e.g., "can rollback to 1.5+, not earlier")
2. Migration guide for users
3. Data export tool before upgrade

### Database Introduction (Future)

When introducing database (beyond MVP):
- Session photos remain file-based (never store RAW files in DB)
- DB stores only metadata, session lists, presets
- DB schema version tracked separately
- Migrations provided for upgrades
- **Critical**: Rollback must not corrupt DB; prefer read-only mode on unknown schema

## References

- Session Manager: `apps/boothy/src-tauri/src/session/manager.rs`
- Packaging Guide: `docs/packaging-guide.md`
- Architecture: `docs/architecture/infrastructure-and-deployment-integration.md`
