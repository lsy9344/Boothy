# `Microsoft.RawImageExtension` 오프라인 배포 판정

작성: 2026-08-17 · Story 7.7 「핵심 설계 결정 7」 · HV-16 이관 항목

## 이 문서가 답하는 질문과 답하지 않는 질문

HV-16 이 Story 7.7 로 넘긴 것은 **채택이 아니라 질문**이다.

> 오프라인 부스 installer 에 `Microsoft.RawImageExtension` 을 동봉할 수 있는가?

- **답한다:** 오프라인 배포가 가능한가.
- **답하지 않는다:** 채택할 것인가. 채택은 별도 승인 안건이고, 아래 「결론」이 그 이유를 적는다.

## 배경

HV-16 이 발견한 사실은 새로웠다. Windows 가 `Microsoft.RawImageExtension` 2.5.24.0 을 통해
CR2 를 **in-process 로, 1284 ms median 에** 디코드할 수 있다. darktable 한 번 돌리는 것보다
훨씬 빠르다.

그러나 같은 회차가 두 가지를 함께 기록했다.

| 사실 | 값 | 출처 |
| --- | --- | --- |
| in-process CR2 디코드 속도 | 1284 ms median | `tests/hardware/resident-renderer/hv-16/baseline/summary.json` |
| darktable 대비 parity (SSIM) | **0.013 ~ 0.159** (기준 ≥ 0.95) | `hv-16/parity/wic-vs-darktable-*.json` |
| darktable 대비 색차 (ΔE00 median) | **22.96 ~ 26.56** (기준 ≤ 3) | 같음 |
| 패키지 형태 | Microsoft Store appx (x64) | `hv-16/environment.md` |

## 세 문항

판정 규칙: **세 개 중 하나라도 「아니오」면 `offline-distribution: not-possible` 로 닫는다.**

### 문항 1 — Store 외 오프라인 설치 수단이 존재하는가

**판정: 아니오.**

- Raw Image Extension 은 Microsoft Store 를 통해서만 배포되는 appx 패키지이고, Microsoft 가
  제공하는 **독립 실행 설치본이 없다.**
- Store 앱을 오프라인 라이선스로 받아 사내 배포하던 경로(Microsoft Store for Business /
  Education 의 offline licensing)는 **은퇴했다.** 그 경로를 전제로 한 절차는 더 이상 재현할 수 없다.
- 부스 PC 는 인터넷이 없다. Store 클라이언트가 동작할 수 없으므로 설치 시점 획득도 불가능하다.

**증거 강도:** 이 판정은 **문서 근거에 의한 판단**이고, clean offline VM 에서의 실측이 아니다.
실측하려면 오프라인 appx 획득 수단이 먼저 있어야 하는데, 그 수단이 없다는 것이 곧 판정이다.

### 문항 2 — 재배포 라이선스상 허용되는가

**판정: 아니오 (확인되지 않음).**

- Boothy 가 Microsoft 의 appx 를 제3자 installer 에 담아 배포할 권리를 부여하는 조항을
  **확인하지 못했다.** WebView2 Runtime 처럼 명시적 재배포 조건이 공개된 구성요소와 다르다.
- 확인되지 않은 재배포 권리는 「없음」과 같게 취급한다. Story 7.3 이 LGPL-2.1/CDDL 때문에 LibRaw 를
  뺀 것과 같은 규칙이다.

### 문항 3 — 버전 고정이 가능한가

**판정: 아니오.**

- Store 배포 패키지는 Store 갱신 정책을 따른다. 설치본이 특정 버전을 고정할 수단이 없다.
- 이것은 WebView2 와 같은 구조적 한계다. WebView2 는 그래서 **회차마다 실측 버전을 기록**하는
  방식으로 다룬다. 그러나 WebView2 는 화면을 그리는 셸이고, RAW 디코더는 **고객 사진의 픽셀을
  직접 만든다.** 픽셀을 만드는 구성요소를 버전 고정 없이 배포하면 지점마다 다른 결과가 나온다.

## 결론

```
offline-distribution: not-possible
adoption: not-adopted
RESIDENT_APPROVED_DIRECT_DECODERS: 비어 있는 채로 유지
```

**세 문항이 모두 「아니오」다.** 하나만 아니어도 닫히는 규칙이므로 판정은 명확하다.

그리고 **이 판정이 긍정이었더라도 채택은 아니었다.** HV-16 이 이미 parity 미달을 기록했다
(SSIM 0.013~0.159, ΔE00 22.96~26.56). 승인된 production route 는
`raw-original + pinned darktable 5.4.1` 이고, 이 Story 는 그 판정을 뒤집지 않는다.

배포 가능성과 화질 적합성은 **독립된 두 관문**이다. 이 문서는 첫 번째 관문만 닫았고,
두 번째 관문은 HV-16 이 이미 닫았다.

## 이 판정을 다시 열어야 하는 조건

다음이 **모두** 성립하면 다시 묻는다. 하나라도 빠지면 다시 묻지 않는다.

1. Microsoft 가 오프라인 설치 가능한 독립 배포본을 제공하고,
2. 그 배포본의 재배포 조건이 문서로 확인되고,
3. 버전 고정 수단이 생기고,
4. **parity 가 SSIM ≥ 0.95 / ΔE00 ≤ 3 을 실측으로 만족한다.**

4 번이 없으면 1~3 번은 의미가 없다. 빠른 디코더가 다른 그림을 만들면 그것은 다른 제품이다.

## 역참조

- HV-16 후속 항목: `tests/hardware/resident-renderer/hv-16/decision.md`
- ledger Story 7.7 행: `_bmad-output/implementation-artifacts/hardware-validation-ledger.md`

**양쪽에 모두 적는다.** 한쪽만 적으면 다음 회차에 또 묻는다.
