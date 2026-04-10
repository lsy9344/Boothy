# darktable Reference

## 목적

이 디렉터리는 darktable 전체 소스를 vendoring하기 위한 폴더가 아니다.  
Boothy가 `preset authoring truth source`와 `headless preset apply engine`으로 darktable를 채택했을 때, 에이전트와 개발자가 같은 전제를 공유하도록 만드는 참조 노트다.

현재 원칙:

- darktable 전체 소스는 지금 repo에 복사하지 않는다
- upstream은 별도 clone 또는 submodule로 관리한다
- 이 디렉터리에는 pinned upstream 정보, 운영 전제, 핵심 명령어만 둔다

## Upstream

- GitHub repo: `https://github.com/darktable-org/darktable`
- Official docs: `https://docs.darktable.org/`
- Pinned release tag: `release-5.4.1`
- Pinned release commit: `c3f96ca`
- Pin date: `2026-03-15`

Pinning rules:

- Boothy runtime apply 결과는 pinned tag 기준으로 검증한다
- preset fidelity가 깨질 수 있으므로 darktable version은 함부로 올리지 않는다
- version을 올릴 때는 preset revalidation, preview SLA, final export 결과를 다시 확인한다

## 왜 참고하는가

Boothy는 darktable를 아래 목적에 한정해 참고한다.

- 운영자용 preset authoring workflow
- darktable sidecar/XMP 기반 preset 저장 개념
- `darktable-cli` 기반 headless apply path
- OpenCL/GPU 가속 가능한 Windows RAW processing path
- 노이즈 억제와 tone/color pipeline의 기준 동작

Boothy는 darktable를 아래 용도로 사용하지 않는다.

- 고객용 runtime UI
- full photo library manager
- Lightroom preset 호환 엔진
- 제품 전체 베이스 앱

## 채택 / 모방 / 제외 범위

이 섹션은 운영용 축약본이다. 최종 제품 경계의 authoritative source는 `_bmad-output/planning-artifacts/architecture.md`다.

### 채택

- `XMP sidecar + history stack`
  - runtime preset truth의 1차 artifact다
  - session manifest는 `raw`, `preview`, `final`, `render status`를 분리하는 방향을 따른다

- `darktable-cli`
  - preview/final render의 headless apply 경로다
  - render worker가 queue, retry, validation을 맡는다

- 핵심 look/correction module 결과
  - `input color profile`
  - `exposure`
  - `filmic rgb`
  - `color balance rgb`
  - `diffuse or sharpen`
  - denoise 계열
  - `lens correction`
  - `orientation`
  - `crop`
  - `rotate and perspective`
  - Boothy는 이 모듈의 UI를 그대로 쓰는 것이 아니라, approved XMP artifact에 bake된 결과를 채택한다

- OpenCL/GPU capability와 preview/final quality profile 분리
  - preview는 latency 우선
  - final export는 quality 우선

### 모방

- preset catalog / publish / rollback workflow
  - darktable의 style/library UX를 그대로 노출하지 않고 Boothy preset catalog UX로 재구성한다

- internal preset authoring surface
  - 필요하면 darktable GUI나 darktable 기반 rich editing flow를 감싸서 쓸 수 있다
  - 하지만 제품 계약은 approved artifact publication에 있다

### 제외

- `.dtstyle`를 runtime truth로 사용하는 것
- darktable를 camera tethering/control truth source로 사용하는 것
- customer/operator surface에 darktable module 명이나 raw editing controls를 직접 노출하는 것
- watermark/export adornment를 MVP preset truth에 섞는 것
- darktable library/config state를 Boothy business truth로 취급하는 것

## Boothy에서의 권장 artifact 전략

Boothy는 runtime apply truth를 가능하면 `darktable XMP sidecar template`로 고정한다.

권장 우선순위:

1. `XMP sidecar template`를 runtime apply의 1차 artifact로 사용
2. `.dtpreset`나 `.dtstyle`는 있으면 authoring reference로만 보관
3. preset catalog는 `preset manifest + xmp template path + darktable version pin`을 함께 저장

이렇게 두는 이유:

- darktable의 history stack과 processing state는 XMP sidecar에 직결된다
- `darktable-cli`는 XMP를 직접 받아 export할 수 있다
- `--style` 기반 apply는 `data.db`와 `configdir` 의존성이 더 강하다
- runtime에서 반복 재현성과 병렬 운용을 맞추려면 style보다 XMP template가 더 안전하다
- `.dtpreset`는 제품 런타임 필수 입력이 아니라 authoring 흔적/참조 metadata에 가깝다

## 핵심 운영 전제

- capture 성공과 render 성공은 분리한다
- camera service는 RAW transfer까지를 책임지고, darktable apply는 render worker가 책임진다
- preview render와 final export render는 다른 profile로 나눌 수 있다
- 가장 무거운 noise pipeline을 항상 preview 경로에 강제하지 않는다
- darktable 인스턴스는 같은 library/config를 공유하며 무작정 병렬 실행하지 않는다

## 핵심 명령어

### 1. 버전 확인

```powershell
darktable --version
```

### 2. OpenCL/GPU 환경 점검

```powershell
darktable-cltest
```

### 3. 운영자용 GUI authoring

```powershell
darktable "C:\path\to\sample.CR3"
```

운영 원칙:

- preset authoring은 별도 작업용 library/config에서 수행하는 편이 안전하다
- 제품 runtime과 authoring 작업 환경을 같은 state로 섞지 않는다

### 4. XMP sidecar 기반 preview export

```powershell
darktable-cli `
  "C:\path\to\capture.CR3" `
  "C:\path\to\preset-template.xmp" `
  "C:\path\to\preview.jpg" `
  --hq false `
  --core `
  --configdir "C:\boothy\darktable\preview-config" `
  --library "C:\boothy\darktable\preview-library.db"
```

### 5. XMP sidecar 기반 final export

```powershell
darktable-cli `
  "C:\path\to\capture.CR3" `
  "C:\path\to\preset-template.xmp" `
  "C:\path\to\final.jpg" `
  --hq true `
  --core `
  --configdir "C:\boothy\darktable\final-config" `
  --library "C:\boothy\darktable\final-library.db"
```

### 6. style 기반 export 예시

```powershell
darktable-cli `
  "C:\path\to\capture.CR3" `
  "C:\path\to\final.jpg" `
  --style "Boothy Warm" `
  --style-overwrite `
  --core `
  --configdir "C:\boothy\darktable\style-config" `
  --library "C:\boothy\darktable\style-library.db"
```

주의:

- style apply는 가능하지만 runtime 주 경로로는 권장하지 않는다
- style은 `data.db` 의존성이 커서 병렬/격리 전략이 더 복잡하다

## 에이전트가 알아야 할 핵심 포인트

- runtime의 preset truth는 `name`이 아니라 `artifact`다
- artifact에는 최소 `preset id`, `display name`, `xmp template path`, `darktable version`, `preview profile`, `final profile`이 포함되어야 한다
- `darktable-cli`를 호출하는 worker는 camera helper와 별개로 취급한다
- manifest는 `raw asset`, `preview asset`, `final asset`, `render status`를 분리해 저장하는 방향으로 진화해야 한다
- 현재처럼 capture 단계에서 곧바로 processed file 하나를 확정하는 구조는 darktable 경로와 맞지 않는다

## 지금 repo에 source dump를 넣지 않는 이유

- repo 검색 노이즈가 커진다
- third-party drift 관리가 어려워진다
- 실제로 필요한 건 source 전체보다 `pinned version + artifact contract + CLI apply path`다
- 직접 patch가 필요한 시점이 오면 그때 fork 또는 submodule 전략으로 전환하는 편이 낫다

## 참고 링크

- Release 5.4.1: `https://github.com/darktable-org/darktable/releases/tag/release-5.4.1`
- CLI manual: `https://docs.darktable.org/usermanual/4.8/en/special-topics/program-invocation/darktable-cli/`
- Sidecar overview: `https://docs.darktable.org/usermanual/4.0/en/overview/sidecar-files/`
- Sidecar import: `https://docs.darktable.org/usermanual/4.0/en/overview/sidecar-files/sidecar-import/`
- Styles module: `https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/lighttable/styles/`
- OpenCL activation: `https://docs.darktable.org/usermanual/development/en/special-topics/opencl/activate-opencl/`
