# 프리셋 번들 계약

## 목적

이 문서는 현재 booth runtime이 읽는 `published-preset-bundle/v1`의 닫힌 기준을 기록한다.
Story 4.3의 publish flow도 이 published bundle을 그대로 canonical output으로 사용한다.

## Published Preset Bundle v1

- schemaVersion: `published-preset-bundle/v1`
- presetId: `preset_*` 형식의 stable identifier
- displayName: booth 고객에게 보이는 이름
- publishedVersion: `YYYY.MM.DD`
- lifecycleStatus: `published`
- boothStatus: `booth-safe`
- runtime render metadata:
  - darktableVersion: pinned runtime version, 현재 기준 `5.4.1`
  - xmpTemplatePath: bundle root 내부 XMP sidecar template
  - previewProfile:
    - profileId
    - displayName
    - outputColorSpace
  - finalProfile:
    - profileId
    - displayName
    - outputColorSpace
- preview:
  - kind: `preview-tile` 또는 `sample-cut`
  - assetPath: bundle root 내부 파일
  - altText: 고객에게 보여 줄 설명
- proxyPublication (선택, Story 7.4):
  - **선택 metadata지만 proxy lane 진입에는 필수다.** 블록이 없으면 `proxyCompatible=false`이며,
    RAW 원본+참조 렌더러 조합도 display proxy로 게시하지 않는다.
  - 현재 내장 preset 3종은 제품 책임자의 수동 승인을 명시적으로 기록한다. 측정하지 않은 품질
    수치나 corpus 경로는 만들지 않으며, 이후 fast source나 상주 렌더러로 경로를 바꾸면 별도
    승인 근거를 갱신해야 한다.
  - proxyCompatible: `true`일 때만 아래 필드가 의미를 갖는다
  - supportedOperations: 이 recipe가 재현할 수 있다고 승인된 operation allowlist (비어 있으면 강등)
  - proxyRecipeVersion: 승인된 recipe 버전
  - proxyRecipePath: bundle root 안의 경로. **게시 host가 기록한다.**
    참조 렌더러가 darktable인 동안 recipe의 실체는 번들이 이미 싣고 있는 XMP template이다.
    별도의 compiled recipe 파일은 Story 7.5가 상주 렌더러를 도입할 때 생긴다
  - referenceRenderer / referenceRendererVersion: `darktable` + pinned `5.4.1`.
    다르면 승인된 화질 증거가 지금 도는 렌더러의 것이 아니므로 강등한다
  - outputProfile: `colorSpace`(현재 `sRGB`), `jpegQuality`(1~100), `iccIntent`(기본 `perceptual`)
  - visualApproval: `approvedAt`, `approvedBy`, 선택 `corpusPath`
- 확장 metadata:
  - publish host는 `sourceDraftVersion`, `publishedAt`, `publishedBy`,
    `sampleCut`, `darktableProjectPath` 같은 추가 필드를 기록할 수 있다.
  - 이 추가 필드는 booth loader의 필수 판정 기준이 아니며, `published-preset-bundle/v1`
    schemaVersion을 바꾸지 않는다.

## 런타임 로더 규칙

- booth catalog loader는 `preset-catalog/published/**/bundle.json`만 읽는다.
- `lifecycleStatus != published` 또는 `boothStatus != booth-safe`면 무시한다.
- preview 자산이 bundle root 밖으로 벗어나면 무시한다.
- runtime render loader는 catalog summary와 별도로 `darktableVersion`, `xmpTemplatePath`,
  `previewProfile`, `finalProfile`까지 모두 읽을 수 있어야 한다.
- runtime render loader는 위 필드 중 하나라도 비어 있거나 bundle root 밖을 가리키면 실패해야 한다.
- draft 또는 validated artifact는 이 로더 경계에 들어오면 안 된다.
- **proxy 자격 판정은 부가 판정이다.** `proxyPublication`이 없거나 불완전해도 preview/final 로딩은
  성공해야 한다. 강등은 항상 고유 사유와 함께 기록한다 — 조용한 통과는 없다.
  - `proxy-not-declared` / `proxy-not-compatible` / `proxy-incomplete-metadata`
  - `proxy-recipe-unresolvable` / `proxy-reference-renderer-mismatch` / `proxy-visual-approval-missing`
- publish host는 기존 `presetId/publishedVersion` 디렉터리를 in-place 수정하면 안 된다.
  같은 version이 이미 존재하면 새 bundle을 만들지 않고 거절해야 한다.

## Catalog State v1

- live future-session catalog truth는 immutable bundle 디렉터리와 별도의
  `preset-catalog/catalog-state.json`이 소유한다.
- `catalog-state.json`은 preset identity별 현재 live published version과 `catalogRevision`을 기록한다.
- publish와 rollback은 bundle directory를 지우거나 수정하지 않고 이 live pointer만 갱신한다.
- active session은 `session.json.catalogSnapshot`에 고정된 version만 사용하고, 새로 시작한 session만
  최신 `catalog-state.json`을 본다.
- catalog audit는 `preset-catalog/catalog-audit/{presetId}.json`에 `published`/`rollback` action,
  actor, timestamp, from/to version을 남긴다.

## Story 4.3과의 관계

- Story 4.2는 draft artifact를 host에서 검증해 `validated` 내부 상태를 만든다.
- Story 4.3은 그 validated draft를 `approved -> published` publish flow로 승격하지만,
  output format은 여전히 `published-preset-bundle/v1`이다.
- publish host는 bundle을 live catalog에 노출한 뒤 audit/draft 저장이 실패하면, 그 bundle을
  다시 제거해 반쯤 완료된 published artifact를 남기지 않아야 한다.
- publish 성공은 future session catalog에만 반영되고, active session manifest나 existing
  capture binding을 직접 바꾸면 안 된다.

## Story 4.4와의 관계

- Story 4.4는 same preset identity의 이전 승인 버전으로 rollback할 수 있게 하되, immutable bundle
  자체는 그대로 유지한다.
- rollback target은 이미 승인된 published booth-safe bundle이어야 한다.
- rollback 성공은 live catalog pointer와 catalog audit만 바꾸고, 진행 중인 세션의 `activePreset`이나
  기존 capture binding은 바꾸지 않는다.

## 화질 승인 기준 (Story 7.4)

`visualApproval`은 아래 게이트를 통과한 뒤에만 기록한다. 하나라도 미달이면 그 preset은
proxy lane에 들어가지 않고 정확한 darktable RAW 경로만 쓴다.

| 지표 | 기준 |
| --- | --- |
| SSIM | ≥ 0.95 |
| median ΔE00 | ≤ 3 |
| p95 ΔE00 | ≤ 8 |
| skin ROI median ΔE00 | ≤ 3 |
| proxy MTF50 | RAW 정밀본의 80% 이상 |
| clipping 증가 | ≤ 2%p |
| upscale | 0 |
| blind review | 5명 × 30 transition에서 "달라 보이거나 거슬린다" < 5% |

**속도를 얻기 위해 이 게이트를 완화하지 않는다.** 과거 XMP trimming이 2.8초를 만들었지만
look-affecting module을 제거한 것이어서 비교 증거로 강등된 이력이 이 원칙의 근거다.
