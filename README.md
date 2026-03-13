# Boothy

Boothy is a Tauri 2 + React 19 desktop booth shell baseline.

## Local Runtime Baseline

- Node.js: `20.19+` or `22.12+`
- pnpm: enabled through Corepack
- Rust: `1.77.2+`

## First-Time Setup

```bash
corepack enable
pnpm install
```

## Local Development

Frontend only:

```bash
pnpm dev
```

Desktop shell:

```bash
pnpm tauri dev
```

Quality checks:

```bash
pnpm test:run
pnpm lint
pnpm build
```

## Booth Shell Verification

When you launch `pnpm tauri dev`, verify the baseline shell against this checklist:

1. The app opens as a full-screen Tauri window without browser chrome.
2. The first screen shows one primary sentence, one supporting sentence, and one primary action only.
3. The primary action is visibly focusable and remains at least `56x56px`.
4. The layout stays readable at `1920x1080`, `1366x768`, and `1280x800`.
5. Reduced motion remains safe when the OS motion setting is lowered.
6. Branch config loads without surfacing diagnostics to the customer screen.
