# Coding Standards

## Existing Standards Compliance

**Code Style:**
- **TypeScript/React:** `reference/uxui_presetfunction` 기준으로 `singleQuote=true`, `semi=true`, `printWidth=120`(Prettier) 및 React 함수형 컴포넌트 + hooks 패턴을 유지합니다.
- **Rust(Tauri backend):** 모듈 분리(예: `file_management.rs`, `image_processing.rs` 등) 패턴을 유지하고, `tauri::command` 기반의 명시적 API 경계를 유지합니다.
- **C# sidecar:** UI 없는 headless 프로세스 기준으로, 카메라 제어/IPC/로깅을 명확히 분리합니다(제품 UI는 금지).

**Linting Rules:**
- **Frontend:** ESLint + Prettier를 사용하며(`plugin:prettier/recommended`), `quotes: 'single'`, `semi: always`, `no-unused-vars(argsIgnorePattern: '^_')` 규칙을 준수합니다.
- **Rust:** `cargo fmt`/`cargo clippy`를 기본 품질게이트로 사용합니다(추가 설정이 없다면 rustfmt 기본 규칙).
- **C#:** 최소한 `.editorconfig` + 경고 수준 고정(가능하면 treat warnings as errors)으로 일관성을 확보합니다.

**Testing Patterns:**
- RapidRAW 코드베이스에서 명확한 자동화 테스트(프론트 단위 테스트/백엔드 `#[test]`) 패턴은 현재 확인되지 않습니다. 따라서 Boothy는 신규 테스트 프레임워크를 “즉시 도입”하기보다, 먼저 **통합/회귀 시나리오**를 문서화하고(Testing Strategy에서 정의) 위험도가 높은 영역부터 점진적으로 테스트를 추가합니다.

**Documentation Style:**
- 문서는 Markdown(`docs/*.md`)으로 유지하고, “결정/근거/검증 포인트”를 함께 기록합니다(현장 운영/AI 개발 핸드오프 목적).

## Enhancement-Specific Standards

- **Boothy 네임스페이스:** 신규 Tauri command는 `boothy_*`, 이벤트는 `boothy-*`, sidecar RPC는 `camera.*`로 네임스페이스를 고정합니다.
- **Append-only 저장 규칙:** `.rrdata`(`ImageMetadata.adjustments`) 확장은 기존 키를 파괴적으로 변경하지 않고 **추가(append)**만 허용합니다(호환성).
- **Boothy 메타데이터 키:** RapidRAW 조정 키와 충돌을 피하기 위해 Boothy 전용 메타데이터는 `adjustments.boothy`(object) 하위에 저장합니다(예: `presetId`, `presetName`, `appliedAt`). 실제 “프리셋 적용 결과”는 상위 조정 키(exposure 등)로 스냅샷 저장합니다.
- **Background-first:** import/프리셋 적용/썸네일/Export는 UI 스레드를 block하지 않습니다. CPU 집약 작업은 `spawn_blocking`(또는 기존 Rayon 패턴)으로 분리합니다(NFR4).
- **File 안정화:** watcher는 “파일 생성 이벤트”만으로 import 확정하지 않고, 락/사이즈 안정화/최소 시간 등 안정화 체크 후 확정합니다(NFR5).
- **에러는 코드화:** sidecar/IPC/파일 감지/프리셋 처리 에러는 문자열만이 아니라 **에러 코드 + 메시지 + 컨텍스트**로 표준화하고, UI에는 “행동 가능한” 상태로 노출합니다(FR20).

## Critical Integration Rules

- **Existing API Compatibility:** RapidRAW 프리셋 포맷(`presets.json`의 `PresetItem`)과 렌더/Export 파이프라인을 변경하지 않습니다. Boothy는 “프리셋 선택/스냅샷 저장/세션 폴더 제약”만 추가합니다(CR1).
- **Database Integration:** MVP는 DB를 사용하지 않습니다. DB 도입 시 `schemaVersion` + 마이그레이션을 제공하고, 세션 폴더/원본 파일을 DB에 넣지 않습니다(CR2, NFR5).
- **Error Handling:** 카메라 연결/촬영/전송 실패는 앱 크래시로 이어지면 안 되며, 기존 세션 사진의 탐색/Export는 계속 가능해야 합니다(FR20).
- **Logging Consistency:** capture→transfer→import→preset→export의 상관관계(`correlationId`)를 로그로 연결합니다(NFR8). 비밀번호/민감정보(세션 이름 등)는 기본 로그에 평문으로 남기지 않으며, 필요 시 진단 모드에서만 제한적으로 기록합니다(NFR6).
