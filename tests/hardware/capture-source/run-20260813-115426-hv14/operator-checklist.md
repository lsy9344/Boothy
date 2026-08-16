# HV-14 Operator Checklist

## 시작 전

- [ ] 카메라는 현재 상시 전원과 PC 직접 USB 연결을 그대로 사용한다.
- [ ] EOS Utility 등 카메라를 점유할 수 있는 다른 앱을 종료한다.
- [ ] 메모리카드 없이 PC 저장으로 운용한다.
- [ ] 렌즈 줌, 구도, 화이트밸런스를 회차 중 변경하지 않는다.
- [ ] 각 장면 블록 안에서는 노출과 초점을 고정한다.
- [ ] `scene-plan.md`의 네 장면을 준비한다.
- [ ] 이번 회차에서는 기존 RAW → 필터 → JPEG 제품 흐름을 변경하지 않는다.

## 실행

물리 준비를 모두 확인한 뒤에만 다음 명령으로 앱을 시작한다.

```powershell
.\start-measurement.ps1 -OperatorReady
```

새 세션 하나에서 `scene-plan.md` 순서대로 셔터를 **정확히 35회** 누른다.

- 1~5회: warm-up, 일반 실내 장면
- 6~13회: 밝고 균일한 장면 8회
- 14~21회: 어두운 장면 8회
- 22~28회: 사람/피부톤 장면 7회
- 29~35회: 고대비 장면 7회

실패한 촬영도 횟수에 포함한다. 카메라 또는 USB 문제로 회차가 중단되면 추가 촬영으로 이어 붙이지 말고, 해당 세션을 보존한 뒤 새 세션에서 35회를 처음부터 다시 수행한다.

## 종료 후

- [ ] 앱을 정상 종료한다.
- [ ] 카메라 Image Quality가 측정 전 RAW 설정으로 복원됐는지 확인한다.
- [ ] `environment.md`의 `Image Quality after`를 실제 확인값으로 바꾼다.
- [ ] 이번 회차에서 새로 생성된 `session_<id>`를 확인한다.

```powershell
.\collect-evidence.ps1 -SessionId 'session_<id>'
```

수집기가 source 및 display telemetry 완결성 검사를 모두 통과해야 한다. 그 후 capability, correlation, quality, aggregate, decision 증거를 채우고 최종 패키지 게이트를 실행한다.

각 증거 디렉터리의 `*.template.json`을 실제 이름(`summary.json` 또는 `manifest.json`)으로 복사해
실측값과 실제 파일 경로를 채운다. `decision.template.md`는 `decision.md`로 복사한다.
