# Booth Hardware Validation Checklist

## 목적

이 문서는 실제 카메라, 실제 helper, 실제 RAW 파일, 실제 darktable 적용 경계가 승인된 부스 장비에서 끝까지 동작하는지 확인하기 위한 운영용 실검증 런북이다.

현재 코드베이스의 story `done`은 주로 구현 + 자동 테스트 완료를 의미한다. 이 문서는 제품 관점의 실장비 검증 완료를 별도로 잠그기 위한 마지막 게이트다.

아키텍처 관점의 상세 근거와 연구 메모는 `docs/runbooks/booth-hardware-validation-architecture-research.md`를 함께 본다.
EDSDK helper 구현 기준선은 `docs/contracts/camera-helper-edsdk-profile.md`를 함께 본다.

## 적용 범위

canonical release-gated stories:

- Story 1.4: 준비 상태 안내와 유효 상태에서만 촬영 허용
- Story 1.5: 현재 세션 촬영 저장과 truthful `Preview Waiting` 피드백
- Story 1.6: 실카메라/helper readiness truth 연결과 false-ready 차단
- Story 1.13: guarded cutover와 original-visible-to-preset-applied-visible hardware validation gate
- Story 3.2: `Export Waiting`과 truthful completion 안내
- Story 4.2: 부스 호환성 검증과 승인 준비 상태 전환
- Story 4.3: 승인과 불변 게시 아티팩트 생성

supporting regression / follow-up scope:

- Story 1.7: `HV-04` / `HV-05` capture correlation evidence를 Story 1.5 close review에 공급하는 supporting story
- Story 1.11 / 1.12: preview architecture supporting evidence를 공급하지만 canonical release close owner는 아니다.
- Story 1.19: ETW/WPR/WPA/PIX + parity diff gate를 위한 evidence package와 rerun 판단 기준을 고정하는 supporting governance story
- Story 2.3: `HV-06` 후속 validation note와 preset switch regression 확인 범위
- canonical close record는 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`가 소유한다.

## 이 문서가 막으려는 실패

- 카메라가 실제로 준비되지 않았는데 고객 화면이 `Ready`를 보여주는 false-ready
- RAW 저장은 되었지만 preview 또는 final 준비 전인데 완료처럼 보이는 false-complete
- active session이 의도하지 않은 다른 preset version으로 조용히 바뀌는 preset drift
- darktable runtime이 같은 `configdir` 또는 `library`를 공유해 결과가 흔들리거나 충돌하는 상태 오염
- draft 또는 validated artifact가 booth runtime에 섞이는 게시 경계 오염
- 고객 화면에 darktable, XMP, helper, OpenCL 같은 내부 용어가 노출되는 UX 누수
- 실패가 발생했는데 `session.json`, `timing-events.log`, bundle evidence가 남지 않아 원인 분석이 막히는 상황

## 종료 기준

아래 조건을 모두 만족해야 실장비 검증 통과로 본다.

- 승인된 Windows 부스 장비에서 실제 카메라 연결 상태를 확인했다.
- 실제 촬영으로 생성된 RAW 파일이 세션 루트에 저장되는 것을 확인했다.
- 게시된 preset version이 실제 세션에서 선택되고, preview 또는 final 결과에 반영되는 것을 확인했다.
- `Preview Waiting`과 `Export Waiting`이 false-ready 없이 truthful하게 동작하는 것을 확인했다.
- active session의 `catalogSnapshot`과 capture binding이 게시 중 변경에 흔들리지 않는 것을 확인했다.
- 카메라 재연결 또는 darktable 실패 시에도 세션 truth와 고객 안내가 안전하게 유지되는 것을 확인했다.
- 각 시나리오의 증거를 수집했다.

권장 운영:

- 실장비 검증 전까지 관련 스토리는 `review`로 유지한다.
- canonical close record와 sprint review 표는 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에 함께 남긴다.
- 또는 별도 `hardware-smoke` / `hardware-validation` 스토리를 만들어 해당 스토리가 `done`일 때 제품 관점 완료로 간주한다.

## Go / No-Go 게이트

아래 중 하나라도 위반되면 이번 검증 회차는 `No-Go`다.

- HV-00, HV-01, HV-02, HV-03, HV-04, HV-05, HV-07, HV-09, HV-10, HV-11, HV-12 중 하나라도 실패
- darktable pinned version 불일치
- preview/final/style runtime이 같은 `configdir` 또는 `library`를 공유
- booth runtime이 published bundle이 아닌 draft / validated artifact를 읽음
- active session의 `catalogSnapshot` 또는 capture binding이 조용히 바뀜
- preview 파일이 없는데 `previewReady`로 보이거나, post-end truth 없이 `Completed`가 보임
- 고객 화면에 darktable, XMP, helper, library, OpenCL, SDK 용어가 노출됨
- 증거 패키지에서 `session.json`, `bundle.json`, `timing-events.log`, 화면 캡처 중 핵심 항목이 누락됨
- Story 1.19 evidence package에서 `preview-promotion-evidence.jsonl`, route policy snapshot, parity oracle result, rollback evidence가 빠짐

## 운영 고정값

이 검증은 아래 전제가 잠겨 있어야 의미가 있다.

- 승인된 Windows 부스 PC에서만 수행한다.
- camera helper는 `docs/contracts/camera-helper-edsdk-profile.md`를 따르는 Windows 전용 Canon EDSDK helper exe를 기준으로 본다.
- darktable runtime pin은 `release-5.4.1` / commit `c3f96ca` 기준으로 검증한다.
- booth runtime은 `preset-catalog/published/**/bundle.json`만 읽는다.
- preview architecture rollout 경계는 host-owned `branch-config/preview-renderer-policy.json`로만 제어하고, active session truth는 강제 재해석하지 않는다.
- active session은 `session.json.catalogSnapshot`에 고정된 version만 사용한다.
- publish host는 기존 `presetId/publishedVersion` 디렉터리를 in-place 수정하면 안 된다.
- `camera helper`와 darktable render worker는 별개 프로세스 경계로 취급한다.
- preview, final, style 경로는 서로 다른 `configdir` / `library`를 사용한다.
- authoring 작업 환경과 booth runtime 환경은 같은 darktable state를 공유하지 않는다.

## 사전 준비

- 승인된 Windows 부스 PC 1대
- 실제 카메라, 배터리 또는 전원 어댑터, 데이터 케이블
- 실제 camera helper / sidecar 실행 가능 상태
- `canon-helper.exe`와 Canon EDSDK DLL이 같은 배포 경계에 배치된 상태
- helper version / sdk version / diagnostics path를 확인할 수 있는 상태
- pinned darktable 런타임 사용 가능 상태
- 게시된 preset bundle 최소 2개
- 검증용 draft preset 최소 2개
- 정상 validation / publish 확인용 draft 1개
- 의도적 validation failure 확인용 draft 1개
- 가능하면 look 차이가 명확한 published preset 2개
- 앱 로그 및 session root 확인 가능한 로컬 권한
- 화면 녹화 또는 사진 촬영 도구
- 운영 기준 이상의 디스크 여유 공간

## EDSDK helper 전용 사전 확인

실검증 전에 아래 helper 전용 항목을 먼저 기록하는 편이 좋다.

- helper executable version
- helper가 링크하거나 번들한 sdk package version
- camera model
- helper diagnostics path
- last fresh `camera-status`를 기록할 수 있는지 여부

권장 원칙:

- `helper-ready`는 helper process boot 완료 신호일 뿐 camera `Ready`와 같지 않다.
- helper가 떠 있어도 카메라가 없으면 정상적으로 blocked path가 나와야 한다.
- helper raw detail은 운영 증거로 남길 수 있지만 고객 화면 copy에는 그대로 노출되면 안 된다.

## 앱 실행과 카메라 연결 확인 진입점

실카메라 연결 상태 확인은 브라우저 미리보기(`pnpm dev`)가 아니라 Tauri 앱에서만 수행한다.

이유:

- 브라우저 미리보기는 host 명령 대신 브라우저 fallback readiness를 사용하므로 실제 카메라 연결 truth를 검증할 수 없다.
- 실제 readiness, capture, session root, diagnostics evidence는 Tauri host 기준으로만 확정된다.

권장 실행 순서:

1. PowerShell에서 프로젝트 루트 `C:\Code\Project\Boothy`로 이동한다.
2. booth 화면만 확인할 때는 `pnpm tauri dev --no-watch`로 실행한다.
3. operator 진단 화면까지 함께 볼 때는 아래 환경 변수를 설정한 뒤 같은 명령으로 실행한다.

```powershell
$env:BOOTHY_RUNTIME_PROFILE="operator-enabled"
$env:BOOTHY_ADMIN_AUTHENTICATED="true"
pnpm tauri dev --no-watch
```

실행 결과 기대값:

- 기본 실행은 `booth-window` 하나가 열린다.
- `operator-enabled` 실행은 `booth-window`와 `operator-window`가 함께 열린다.
- 앱 첫 화면은 `/booth` 세션 시작 화면이다.

booth 화면에서 연결 상태 확인 순서:

1. 이름과 휴대전화 뒤 4자리를 입력해 세션을 시작한다.
2. 게시된 preset을 하나 선택한다.
3. capture/readiness 화면으로 진입한다.
4. 카메라가 아직 준비되지 않았으면 고객 상태가 `Ready`가 아니어야 하고, `사진 찍기` 버튼이 비활성화되어야 한다.
5. 카메라와 helper가 준비되면 고객 상태가 `Ready`로 바뀌고, `사진 찍기` 버튼이 활성화되어야 한다.

operator 화면에서 함께 볼 항목:

- 상단 badge가 `Capture 확인 필요`에서 `정상`으로 바뀌는지 본다.
- `Current Session` 문맥에서 lifecycle, preset, updated time이 현재 세션과 일치하는지 본다.
- `Capture Boundary` 카드가 blocked에서 clear로 바뀌는지 본다.

기록 원칙:

- 실행에 사용한 명령과 환경 변수 값을 증거에 함께 남긴다.
- booth 화면 캡처와 operator 화면 캡처를 같은 회차 증거로 묶는다.
- `pnpm dev` 브라우저 화면 캡처는 실장비 검증 증거로 인정하지 않는다.
- Story 1.5 close review와 Story 1.7 supporting evidence에서 인정하는 supported capture trigger는 booth 앱의 `사진 찍기` 버튼이다. 카메라 본체 셔터 직접 입력은 canonical close evidence로 인정하지 않고, 별도 관찰 메모로만 남긴다.

## 선행 차단 체크리스트

실촬영 전에 아래 항목을 먼저 잠근다.

### HV-00 사전 점검과 런타임 격리 확인

목적:
검증 자체가 흔들리지 않도록 장비, 버전, 경로, 격리 조건을 먼저 잠근다.

절차:

1. 검증 대상 부스 PC 이름과 카메라 모델을 기록한다.
2. `darktable --version`으로 pinned version과 일치하는지 확인한다.
3. `darktable-cltest` 결과를 저장해 OpenCL/GPU capability를 확인한다.
4. preview, final, style 경로가 서로 다른 `configdir` / `library`를 쓰는지 확인한다.
5. booth runtime과 authoring 작업이 같은 darktable state를 공유하지 않는지 확인한다.
6. helper 프로세스와 darktable worker를 별도 경계로 보고 각각 실행 가능 상태를 확인한다.
7. `canon-helper.exe`와 Canon EDSDK DLL이 같은 배포 경계에 있는지 확인한다.
8. helper version, sdk version, diagnostics path를 이번 회차 evidence 메모에 기록한다.
9. `preset-catalog/published/**/bundle.json`이 존재하고, 사용할 preset의 `publishedVersion`이 기록돼 있는지 확인한다.
10. `preset-catalog/catalog-state.json`이 live version을 가리키고 있는지 확인한다.
11. 세션 루트에 `session.json`, `captures/originals`, `renders/previews`, `renders/finals`, `handoff`, `diagnostics`를 만들 수 있는 권한이 있는지 확인한다.

통과 기준:

- darktable version pin이 일치한다.
- runtime 간 state 공유가 없다.
- published bundle과 live catalog pointer가 모두 정상이다.
- 세션 루트와 diagnostics 경로에 쓰기 가능하다.

증거:

- `darktable --version` 결과
- `darktable-cltest` 결과
- helper version / sdk version 기록
- helper diagnostics path 기록
- preview/final/style `configdir` / `library` 경로 기록
- published `bundle.json` 캡처 또는 사본
- `catalog-state.json` 캡처 또는 사본

즉시 중단 조건:

- version pin 불일치
- runtime과 authoring이 같은 darktable state 사용
- published bundle 없음
- 세션 루트 또는 diagnostics 쓰기 실패

## 반드시 확인할 경로

검증 증거는 아래 경로와 연결되어야 한다.

- 세션 루트: `Pictures/dabi_shoot/sessions/{sessionId}/`
- 매니페스트: `Pictures/dabi_shoot/sessions/{sessionId}/session.json`
- RAW: `Pictures/dabi_shoot/sessions/{sessionId}/captures/originals/`
- preview: `Pictures/dabi_shoot/sessions/{sessionId}/renders/previews/`
- final: `Pictures/dabi_shoot/sessions/{sessionId}/renders/finals/`
- diagnostics: `Pictures/dabi_shoot/sessions/{sessionId}/diagnostics/`
- timing log: `Pictures/dabi_shoot/sessions/{sessionId}/diagnostics/timing-events.log`
- preview route policy: `Pictures/dabi_shoot/branch-config/preview-renderer-policy.json`
- published bundle: `preset-catalog/published/{presetId}/{publishedVersion}/bundle.json`
- live catalog state: `preset-catalog/catalog-state.json`
- catalog audit: `preset-catalog/catalog-audit/{presetId}.json`

## 수집 증거

각 시나리오마다 아래를 남긴다.

- 실행 일시
- 검증자 이름
- 부스 PC 이름
- 카메라 모델
- darktable 버전
- helper version
- sdk version
- helper 실행 상태 확인 방법
- 사용한 `presetId` / `publishedVersion`
- 생성된 `sessionId`
- 화면 캡처 또는 짧은 녹화
- 생성된 산출물 경로
- RAW
- preview
- final
- `session.json` 사본 또는 핵심 필드 캡처
- `timing-events.log` 마지막 이벤트 캡처
- helper-ready 또는 최근 `camera-status` 캡처
- `bundle.json`과 `catalog-state.json` 근거
- pass / fail 판정
- 실패 시 관찰 메모

### Story 1.19 Evidence Package

- Story 1.19 rerun은 `docs/runbooks/preview-promotion-evidence-package.md`를 기준으로 준비한다.
- trace planning/start는 `scripts/hardware/Start-PreviewPromotionTrace.ps1`를 사용한다.
- trace stop/export planning은 `scripts/hardware/Stop-PreviewPromotionTrace.ps1`를 사용한다.
- booth package assemble은 `scripts/hardware/New-PreviewPromotionEvidenceBundle.ps1`를 사용한다.
- evidence package는 최소 `session.json`, `timing-events.log`, `preview-promotion-evidence.jsonl`, route policy snapshot, published `bundle.json`,
  `catalog-state.json`, booth/operator visual evidence를 같이 보관해야 한다.
- parity diff는 same-capture / same-session / same-preset-version 비교만 허용한다.
- fallback-only evidence, rollback evidence 누락, parity threshold fail은 모두 `No-Go`다.

## 시나리오

### HV-01 게시 프리셋 준비 확인

목적:
실제 부스가 선택 가능한 게시 프리셋만 소비하는지 확인한다.

절차:

1. `/authoring`에서 정상 draft preset을 validation 한다.
2. 같은 preset을 publish 한다.
3. 새 세션을 시작한다.
4. preset 선택 화면에서 방금 publish한 preset이 노출되는지 확인한다.
5. `bundle.json`과 `catalog-state.json`에서 동일 version이 live로 가리켜지는지 확인한다.

통과 기준:

- draft는 `validated`를 거쳐 publish 된다.
- booth catalog에 게시된 preset만 보인다.
- 고객 화면에 darktable, XMP, authoring 용어가 보이지 않는다.
- booth가 live catalog pointer와 published bundle을 일치하게 읽는다.

증거:

- authoring validation 성공 화면
- publish 성공 화면
- booth preset 선택 화면 캡처
- `bundle.json` 캡처
- `catalog-state.json` 캡처

### HV-02 카메라 미연결 차단 확인

목적:
실제 장비 미연결 시 부스가 안전하게 막히는지 확인한다.

절차:

1. 카메라를 분리한 상태로 세션을 시작한다.
2. preset을 선택한다.
3. capture 화면 또는 readiness 화면으로 이동한다.
4. 촬영 버튼을 눌러 본다.
5. 고객 안내가 기술 진단어 없이 대기 또는 직원 호출 의미만 주는지 확인한다.

통과 기준:

- 상태가 `Ready`가 아니어야 한다.
- 촬영이 실제로 차단되어야 한다.
- 고객 화면에 기술 진단어가 아니라 wait / call guidance만 보여야 한다.
- `session.json`과 현재 화면 의미가 모순되지 않는다.

증거:

- readiness 상태 화면 캡처
- 차단된 촬영 시도 화면 캡처
- `session.json` 캡처

### HV-03 카메라 연결 후 Ready 진입 확인

목적:
실제 카메라 연결 시 host가 `Ready`로 정규화하는지 확인한다.

절차:

1. 카메라를 실제로 연결한다.
2. helper / sidecar가 정상 동작하는지, `helper-ready` 이후 fresh `camera-status`가 들어오는지 확인한다.
3. 같은 세션 또는 새 세션에서 preset을 선택한다.
4. readiness가 `Ready`로 바뀌는지 확인한다.
5. 고객 화면이 내부 용어 없이 촬영 가능 상태만 안내하는지 확인한다.

통과 기준:

- `camera/helper ready`가 되면 고객 화면이 `Ready`를 보여준다.
- 촬영 버튼이 활성화된다.
- helper 준비 전에는 false-ready가 나오지 않는다.

증거:

- `Ready` 화면 캡처
- helper 정상 동작 로그 또는 상태 캡처
- 최근 `camera-status` 또는 동등한 helper freshness 근거
- `session.json` 캡처

### HV-04 실제 촬영과 RAW 저장 확인

목적:
실제 촬영 요청이 source RAW 저장까지 완료되는지 확인한다.

절차:

1. `Ready` 상태에서 booth 앱의 `사진 찍기` 버튼으로 실제 촬영을 실행한다.
2. `session.json`과 세션 루트 파일 변화를 확인한다.
3. RAW 파일이 `captures/originals/` 아래 생성되었는지 확인한다.
4. capture record가 active preset version을 바인딩하고 있는지 확인한다.
5. 같은 회차에서 카메라 본체 셔터 직접 입력을 별도로 눌렀다면, 그 결과는 supported success evidence와 분리해 관찰 메모로 기록하고 active session 성공 또는 `Preview Waiting` 시작으로 이어지면 즉시 `Fail`로 판정한다.

통과 기준:

- 촬영 성공 전에 실제 RAW 파일이 세션 루트에 생긴다.
- 새 capture record가 session truth에 기록된다.
- capture record가 active preset version을 참조한다.
- RAW 저장 실패를 preview 성공으로 오인하지 않는다.
- helper correlation 근거에서 동일한 `requestId`/`captureId`가 `request-capture -> capture-accepted -> file-arrived` 순서로 닫힌다.
- 카메라 본체 셔터 직접 입력만으로 생긴 결과는 HV-04 pass 근거로 사용하지 않으며, 그 입력이 active session 성공이나 `Preview Waiting`을 열면 HV-04는 `Fail`이다.

증거:

- session root 파일 목록
- `camera-helper-requests.jsonl`의 해당 `requestId` 기록
- `camera-helper-events.jsonl`의 `capture-accepted -> file-arrived` correlation 기록
- 생성된 RAW 파일 경로
- `session.json`의 capture record 캡처
- capture 직후 화면 캡처

### HV-05 Preview Waiting -> Preview Ready 확인

목적:
저장 성공과 preview 준비 완료가 truthful하게 분리되는지 확인한다.

절차:

1. HV-04에서 booth 앱의 `사진 찍기` 버튼으로 시작한 supported capture 직후 화면을 계속 관찰한다.
2. 먼저 `Preview Waiting`이 나오는지 확인한다.
3. 이후 preview rail 또는 최신 사진 확인 UI에 preview가 나타나는지 확인한다.
4. `renders/previews/`에 실제 파일이 생겼는지 확인한다.
5. 가능하면 capture acknowledgement 시각과 preview 표시 시각을 기록한다.
6. 같은 회차에서 카메라 본체 셔터 직접 입력이 별도로 관찰됐다면, 그 결과가 active session 성공, `Preview Waiting`, `Preview Ready`로 승격되지 않았는지 확인한다.

통과 기준:

- 첫 문장은 저장 완료 사실을 보여준다.
- preview가 준비되기 전에는 false-ready 또는 false-complete가 나오지 않는다.
- 실제 preview 파일이 생긴 뒤에만 최신 사진이 노출된다.
- `timing-events.log`와 화면 전환의 순서가 모순되지 않는다.
- helper correlation과 같은 `captureId`의 preview만 현재 세션 최신 사진으로 노출된다.
- 카메라 본체 셔터 직접 입력으로 우연히 파일이 생기더라도, 그것만으로 HV-05 pass를 판정하지 않으며 그 입력이 `Preview Waiting` 또는 `Preview Ready`를 열면 HV-05는 `Fail`이다.

증거:

- `Preview Waiting` 화면 캡처
- preview 표시 화면 캡처
- preview 파일 경로
- helper correlation을 보여주는 `requestId`/`captureId` 근거
- `timing-events.log` 캡처
- 시간 기록

### HV-06 프리셋 변경 후 이후 촬영만 반영 확인

목적:
preset 변경이 이후 촬영부터만 적용되고 이전 촬영을 다시 쓰지 않는지 확인한다.

절차:

1. preset A로 실제 촬영 1장을 만든다.
2. 세션 중 preset B로 바꾼다.
3. 다시 실제 촬영 1장을 만든다.
4. 두 capture의 preset binding과 결과물을 비교한다.

통과 기준:

- 첫 capture는 preset A binding을 유지한다.
- 두 번째 capture는 preset B binding을 사용한다.
- 첫 capture asset이 다시 렌더되거나 덮어써지지 않는다.

증거:

- 세션별 capture record
- 두 결과 화면 캡처
- 필요 시 preview 또는 final 파일 비교 메모

### HV-07 실제 darktable 적용 확인

목적:
게시된 preset artifact가 실제 RAW에 적용되는지 확인한다.

절차:

1. 눈에 띄게 다른 look의 게시 preset 2개를 준비한다.
2. 동일한 조건에서 preset A와 preset B로 각각 실제 촬영을 수행한다.
3. preview 또는 final 결과를 비교한다.
4. 결과 파일이 각 capture의 preset version과 맞는지 확인한다.
5. 가능하면 `bundle.json`의 `xmpTemplatePath`, `darktableVersion` 등 게시 artifact 메타데이터도 함께 기록한다.

통과 기준:

- preset A와 preset B 결과가 육안으로 구분된다.
- 결과는 각 capture의 preset binding과 일치한다.
- darktable 내부 편집 UI 없이 booth-safe 결과만 노출된다.
- published artifact와 실제 적용 결과가 서로 다른 version을 가리키지 않는다.

증거:

- 두 결과 이미지
- 각 capture의 `presetId` / `publishedVersion`
- `bundle.json` 캡처
- 비교 메모

### HV-08 종료 후 Export Waiting / Completed 확인

목적:
실제 종료 후 결과 준비 상태가 truthful하게 표현되는지 확인한다.

절차:

1. 실제 세션을 종료 시각까지 진행한다.
2. 종료 직후 상태를 관찰한다.
3. `Export Waiting` 또는 `Completed`로 전환되는지 확인한다.
4. 촬영이 계속 차단되는지 확인한다.
5. `postEnd`와 lifecycle stage가 세션 manifest에 기록되는지 확인한다.

통과 기준:

- 종료 후 모호한 상태에 오래 머무르지 않는다.
- `Export Waiting`이면 결과 준비 중임을 보여주고 촬영은 비활성화된다.
- `Completed`는 booth-side work 완료 뒤에만 나온다.
- `postEnd` truth 없이 완료를 주장하지 않는다.

증거:

- 종료 직후 화면 캡처
- `Export Waiting` 또는 `Completed` 화면 캡처
- `session.json`의 `postEnd` 캡처
- 종료 시각 대비 전환 시각 메모

### HV-09 검증 실패 preset 차단 확인

목적:
실패한 draft preset이 booth runtime에 섞이지 않는지 확인한다.

절차:

1. 일부 필수 조건을 깨는 draft preset을 준비한다.
2. validation 을 실행한다.
3. failure finding을 확인한다.
4. booth 새 세션을 시작해 catalog를 확인한다.

통과 기준:

- validation 이 실패한다.
- 실패한 preset은 booth catalog에 노출되지 않는다.
- active session이나 현재 session asset은 바뀌지 않는다.

증거:

- validation 실패 화면
- finding 목록 캡처
- booth catalog 캡처

### HV-10 카메라 분리 후 재연결 복구 확인

목적:
운영 중 USB 분리 또는 카메라 재연결이 생겨도 false-ready 없이 복구되는지 확인한다.

절차:

1. `Ready` 상태까지 진입한다.
2. 카메라를 물리적으로 분리한다.
3. 고객 화면이 즉시 `Ready`에서 내려오고 촬영이 차단되는지 확인한다.
4. 카메라를 다시 연결한다.
5. helper가 `recovering`에서 fresh status를 다시 보낸 뒤 같은 세션 또는 새 세션에서 다시 `Ready`로 돌아오는지 확인한다.
6. 복구 후 실제 촬영 1회를 추가로 수행한다.

통과 기준:

- 분리 직후 false-ready가 유지되지 않는다.
- 복구 전 촬영이 차단된다.
- 재연결 후 `Ready` 복귀가 가능하다.
- 기존 capture 기록과 session truth가 깨지지 않는다.

증거:

- 분리 직후 화면 캡처
- 재연결 후 `Ready` 화면 캡처
- recovery-status 또는 최근 `camera-status` sequence 근거
- 복구 후 추가 capture 증거
- `session.json` 전후 비교

### HV-11 darktable 렌더 실패 격리 확인

목적:
preview 또는 final 렌더가 실패해도 booth가 성공으로 오인하지 않고 안전하게 격리하는지 확인한다.

절차:

1. 실험용 검증 환경에서 darktable apply failure를 유도할 수 있는 조건을 준비한다.
2. 실제 촬영을 1회 수행한다.
3. preview 또는 final 렌더가 실패한 경우 booth가 `previewReady` 또는 `Completed`를 잘못 보여주지 않는지 확인한다.
4. RAW 파일과 session manifest, diagnostics evidence가 그대로 남는지 확인한다.
5. 고객 화면이 안전한 직원 호출 또는 보호 안내로 내려가는지 확인한다.

통과 기준:

- 실패한 렌더를 성공처럼 보이지 않는다.
- RAW와 capture record는 남고, 실패 원인 분석용 evidence가 보존된다.
- stage / reason / 화면 안내가 모순되지 않는다.
- 재촬영 또는 직원 개입 전까지 위험 상태를 숨기지 않는다.

증거:

- 실패 직후 화면 캡처
- `session.json` 캡처
- `timing-events.log` 캡처
- RAW 경로
- 실패 유도 조건 메모

참고:

- 이 시나리오는 출시 후보 전 최소 1회는 반드시 수행한다.
- 운영 현장 production 장비에서 의도적 실패 유도가 위험하면 staging 또는 승인된 검증 장비에서 먼저 수행한다.

### HV-12 active session catalogSnapshot 고정 확인

목적:
세션 진행 중 publish 또는 rollback이 일어나도 현재 세션의 preset truth가 조용히 바뀌지 않는지 확인한다.

절차:

1. published preset version V1이 live인 상태에서 새 세션을 시작하고 preset을 선택한다.
2. `session.json.catalogSnapshot`과 active preset binding을 기록한다.
3. 다른 화면 또는 작업자에서 같은 preset의 새 published version을 만들거나 rollback을 수행한다.
4. 기존 세션으로 돌아와 추가 촬영을 수행한다.
5. 기존 세션의 `catalogSnapshot`과 capture binding이 처음 기록한 version을 유지하는지 확인한다.
6. 별도의 새 세션을 시작해 새 live version이 그쪽에만 반영되는지 확인한다.

통과 기준:

- active session의 `catalogSnapshot`이 바뀌지 않는다.
- 기존 세션 capture binding이 조용히 다른 version으로 바뀌지 않는다.
- 새 세션만 새 live version을 본다.

증거:

- 기존 세션 `session.json` 전후 비교
- 새 세션 `session.json` 비교
- `catalog-state.json` 캡처
- 관련 `bundle.json` 캡처

## 실패 처리 규칙

- HV-00 실패: 검증 중지. 환경 잠금부터 다시 잡는다.
- HV-02 또는 HV-03 실패: 카메라 연결 경계 재점검 후 관련 스토리를 `review`로 되돌린다.
- HV-04 또는 HV-05 실패: 먼저 booth 앱의 `사진 찍기` 버튼 경로에서 실패했는지 확인한다. supported 버튼 경로 실패면 Story 1.5를 `review`로 되돌린다. Story 1.7 supporting evidence row도 함께 재점검한다. 카메라 본체 셔터 직접 입력만 관찰된 회차도 supported success evidence가 아니므로 `Fail`로 기록하고, 현장 혼선 메모와 함께 supported 버튼 경로로 재검증한다.
- HV-06, HV-07, HV-12 실패: Story 2.3 / 4.3 경계를 우선 재점검한다.
- HV-08 실패: Story 3.2 범위를 `review`로 되돌린다.
- HV-09 실패: Story 4.2 경계를 우선 재점검한다.
- HV-10 실패: helper 복구 흐름과 장비 연결 안정성을 release blocker로 취급한다.
- HV-11 실패: darktable apply path를 release blocker로 취급한다.

## 현장 1차 복구 순서

실패 시 바로 랜덤 재시도하지 말고 아래 순서로 확인한다.

1. 고객 흐름을 일시 중지하고 추가 촬영을 막는다.
2. 현재 `sessionId`, `presetId`, `publishedVersion`, 실패 시각을 먼저 기록한다.
3. `session.json`, `timing-events.log`, RAW / preview / final 경로를 확보한다.
4. 카메라 이슈면 전원, 케이블, 포트, helper 상태를 순서대로 확인한다.
5. darktable 이슈면 version pin, `configdir` / `library` 격리, bundle metadata, worker 상태를 순서대로 확인한다.
6. active session evidence를 확보하기 전에는 `session.json`과 published bundle을 수동 수정하지 않는다.
7. draft 또는 임시 artifact를 runtime catalog에 섞지 않는다.

## 절대 금지

- published bundle 디렉터리 in-place 수정
- runtime session 중 `session.json` 수동 편집
- booth runtime에서 draft / validated artifact 직접 사용
- authoring과 runtime이 같은 darktable state 사용
- 동일 `configdir` / `library`를 여러 runtime mode에서 병렬 공유
- evidence 확보 전 임의 파일 삭제
- 고객 화면에서 내부 기술 용어로 문제 설명

## Canonical Ledger

이 runbook의 실행 메모와 별도로, sprint close 판단에 쓰는 canonical close record는 `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에 남긴다.

- runbook 표는 현장 quick scratch sheet다.
- release close, `done` 복귀, blocker/owner 추적은 ledger row가 최종 기준이다.

## 기록 양식

아래 표는 현장 quick check용이다. canonical close row는 ledger의 확장 필드를 반드시 채운다.

| ID | 결과 | 실행 일시 | sessionId | preset/version | 증거 위치 | 메모 |
| --- | --- | --- | --- | --- | --- | --- |
| HV-00 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-01 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-02 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-03 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-04 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-05 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-06 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-07 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-08 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-09 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-10 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-11 | ☐ Pass / ☐ Fail |  |  |  |  |  |
| HV-12 | ☐ Pass / ☐ Fail |  |  |  |  |  |

## 최종 서명

- 검증 일자:
- 검증 환경:
- 검증자:
- darktable pin:
- 카메라 모델:
- helper 버전 또는 식별:
- 최종 판정: `Go` / `No-Go`
- release blocker:
- follow-up owner:

## BMAD 운영 메모

현재 sprint 상태 정의는 `done`을 일반적인 완료 상태로만 두고 있고, 실장비 검증 게이트를 기본으로 포함하지 않는다.

권장 보완안:

1. 관련 story를 실장비 검증 전까지 `review`에 유지한다.
2. `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`에서 story별 `automated pass`, `hardware pass`, `Go / No-Go`, `blocker`, `owner`, `evidence path`를 함께 본다.
3. 이 문서를 기준으로 별도 `hardware-smoke` 스토리를 만들 수 있지만, canonical close record는 ledger에 남긴다.
4. Story 1.13은 preview architecture close owner이므로 ledger row가 `Go`가 되기 전까지 `review`와 `release hold`를 유지한다.
5. 모든 HV 항목 통과 후에만 제품 관점 완료로 간주한다.
