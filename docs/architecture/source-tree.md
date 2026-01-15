# Source Tree

## Existing Project Structure

```plaintext
Boothy/
├── .bmad-core/                      # BMAD 문서/체크리스트 자동화 도구(리포에 포함/버전관리)
├── docs/
│   ├── prd.md
│   ├── brownfield-architecture.md
│   ├── design_concept.md
│   └── architecture.md              # (this document)
│   └── architecture/                # (sharded) tech-stack / source-tree / coding-standards
└── reference/
    ├── camerafunction/
    │   └── digiCamControl-2.0.0/     # C#/.NET Framework 4.0 + WPF (레퍼런스)
    └── uxui_presetfunction/          # RapidRAW (React/Vite + Tauri/Rust, 레퍼런스)
```

## New File Organization

```plaintext
Boothy/
├── apps/
│   ├── boothy/                      # ✅ 제품 코드: RapidRAW 기반 Boothy 앱
│   │   ├── src/                     # React UI (+ Boothy UI Extensions)
│   │   ├── src-tauri/               # Rust backend (+ Boothy services)
│   │   └── ...                      # vite/tauri config, assets, packaging
│   └── camera-sidecar/              # ✅ 제품 코드: Headless 카메라 서비스(.NET)
│       ├── Boothy.CameraSidecar.sln
│       ├── src/
│       └── ...                      # IPC, logging, EDSDK integration wrapper
├── docs/
│   └── ...                          # 기존 유지(아키텍처/스토리/QA 등)
└── reference/
    └── ...                          # 레퍼런스 스택은 가능한 “읽기 전용”으로 유지
```

## Integration Guidelines

- **File Naming:** RapidRAW의 기존 네이밍/레이아웃을 유지하고, Boothy 신규 커맨드는 `boothy_*`(Tauri command) / 이벤트는 `boothy-*`(Tauri event)로 통일합니다. sidecar RPC는 `camera.*` 메서드 네임스페이스를 사용합니다.
- **Folder Organization:** **제품 코드와 레퍼런스를 분리**하기 위해, RapidRAW(현재 `reference/uxui_presetfunction`)는 초기 단계에서 **`apps/boothy`로 승격(migrate)**하여 그 위치에서 리브랜딩/기능 통합을 진행합니다. `reference/`는 카메라 스택 및 업스트림 비교 용도의 “읽기 전용” 영역으로 유지합니다(리포 내 “레퍼런스 vs 제품” 경계 명확화).
- **Import/Export Patterns:** “세션 폴더 계약”을 최우선으로 하고, 앱 내부 통신은 event-driven(새 사진 도착 이벤트 → UI refresh/선택)으로 구성합니다. 저장 포맷은 기존 `.rrdata`를 확장(append-only)합니다.

**Decision (Efficiency)**
RapidRAW가 제품 베이스로 확정된 상태에서는, `reference/` 아래에서 계속 개발하는 것보다 **초기에 `apps/boothy`로 승격**해 “제품 코드 경계”를 명확히 하는 편이 개발/빌드/문서화/온보딩에서 더 효율적입니다. 업스트림 비교는 `UPSTREAM.md` 기록 + git tag/branch(또는 별도 스냅샷 디렉터리)로 대체합니다.
