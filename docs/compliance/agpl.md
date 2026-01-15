# AGPL Compliance (Boothy)

Boothy is derived from RapidRAW (AGPL-3.0). We accept AGPL obligations per `docs/decisions/adr-002-agpl-compliance.md`.

## What every Boothy build must include

1. **License and notices**
   - Include RapidRAW AGPL license text (source: `reference/uxui_presetfunction/LICENSE`)
   - Include third-party notices: `THIRD_PARTY_NOTICES.md`

2. **Corresponding Source**
   - Provide the exact corresponding source for each distributed installer/build:
     - git tag in this repo, and/or
     - a source zip archive attached to the release

3. **User-facing access**
   - Provide an in-app “Licenses / Source” page or an equivalent offline-accessible file that states:
     - the license (AGPL-3.0),
     - where to obtain the matching source (URL and/or local file path),
     - a brief summary of modifications.

## Release checklist (minimum)

- Create release tag (e.g., `boothy-vX.Y.Z`)
- Produce installer artifact(s)
- Produce matching source zip for the same tag/commit
- Ship installer + notices + source location together

