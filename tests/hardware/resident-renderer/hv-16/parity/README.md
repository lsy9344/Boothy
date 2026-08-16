# HV-16 parity 원자료

## 무엇을 비교했는가

| 파일 | 후보 | 참조 | 쌍 |
| --- | --- | --- | ---: |
| `wic-vs-darktable-neutral.json` | Windows WIC direct CR2 디코드 | pinned darktable 5.4.1 중립 렌더 | 6 |
| `wic-vs-darktable-preset.json` | Windows WIC direct CR2 디코드 | pinned darktable 5.4.1 preset 렌더 (3종) | 18 |

두 회차 모두 `inputProvenance: real-capture-direct`다 — HV-14 실촬영 CR2를 그대로 읽었다.
**WebGL2 후보의 parity는 여기에 없다.** WebGL2는 CR2를 읽지 못해 비교할 산출물 자체를 만들 수 없고,
그 사실이 `decision.md`의 `Technology No-Go` 근거다.

## 결과 요약

| 참조 | n | SSIM (median) | ΔE00 median | ΔE00 p95 | clipping Δ |
| --- | ---: | ---: | ---: | ---: | ---: |
| 중립 렌더 | 6 | 0.021 | 26.20 | 33.86 | −37.66 %p |
| `preset-daylight` | 6 | 0.022 | 26.07 | 33.79 | −41.14 %p |
| `preset-mono-pop` | 6 | 0.013 | 26.56 | 34.27 | −56.33 %p |
| `preset-soft-glow` | 6 | 0.159 | 22.96 | 32.62 | −2.38 %p |

기준선은 SSIM ≥ 0.95, ΔE00 median ≤ 3, p95 ≤ 8, clipping 증가 ≤ 2 %p다.
clipping은 증가하지 않았지만 SSIM과 ΔE00이 큰 차이로 미달해 **모든 비교군이 전체 판정에서 실패한다.**

## 이 숫자를 읽을 때 반드시 함께 보아야 할 것

1. **WIC 산출물에는 preset이 적용되어 있지 않다.** 따라서 이 값은 "디코더만의 오차"가 아니라
   상주 WIC 파이프라인이 메워야 할 **전체 격차**다.
2. **corpus가 심한 저노출이다.** HV-15에서 평균 luma 12/255로 기록된 그 corpus이며,
   darktable 중립 렌더의 평균 luma는 0.006(정규화)에 불과하다.
   any-channel 기준에서 darktable 참조는 WIC 후보보다 경계 채널이 많아 모든 비교군의 clipping 차이가 음수다.
   특히 `preset-mono-pop` 참조는 화면의 상당 부분이 검게 눌려 −56.33 %p로 나온다.
3. 패키지에 보존된 재현 원자료는 **90° 회차**인 `wic-vs-darktable-neutral.json`이다.
   270° spot-check에서 SSIM 0.0219가 관측됐다는 실행 메모는 있지만 원자료가 패키지에 없으므로
   재현 가능한 방향성 증거로 사용하지 않는다. 이 값은 입력 축 No-Go 종료 근거에도 포함하지 않는다.

따라서 이 표는 "WIC가 나쁘다"가 아니라 **"두 파이프라인은 drop-in 교체 대상이 아니다"**의 증거다.

## 측정하지 않은 것

- **MTF50** — 도구(`mtf50FromSlantedEdge`)는 구현·검증했으나 HV-14 corpus에
  적합한 slanted-edge 대상이 없다. 자연 사진에서 경계를 자동으로 찾으면 조용히
  엉뚱한 영역을 재고 그 숫자가 evidence에 남는다. **측정하지 않음으로 기록한다.**
- **skin ROI ΔE00** — ROI는 호출자가 지정해야 한다. 이 회차의 매니페스트는 `skinRoi: null`이다.
- **5명 × 30 transition blind review** — 패널 미확정. **AC 3 통과로 기록하지 않으며**, 입력 축 `Technology No-Go` 종료에는 비차단이고 후보 재활성화 시 필수다.

## 지표 구현

`src/quality-metrics/parity-metrics.ts` 하나뿐이다. 실행기가 자기만의 지표를 따로 갖지 않는다 —
두 벌이 되면 evidence의 숫자와 회귀 테스트의 숫자가 갈린다.

- CIEDE2000: Sharma·Wu·Dalal(2005) 공개 검증 표 **17쌍**으로 단위 테스트
- SSIM: 가장자리 부분 블록을 포함한 8×8 블록 SSIM의 픽셀 가중 평균, Rec.709 luma, K1=0.01 / K2=0.03 / L=1
- clipping: sRGB 채널 중 하나라도 ≤ 1/255 또는 ≥ 254/255인 픽셀 비율
- 크기가 다른 쌍은 SSIM·ΔE00을 계산하지 않는다. 억지로 맞추면 숫자가 거짓말을 한다

## 재실행

```bash
BOOTHY_HV16_PARITY_MANIFEST=<manifest.json> \
BOOTHY_HV16_PARITY_OUTPUT=<result.json> \
npx vitest run tests/hardware/resident-renderer/hv-16/tools/hv16-parity.test.ts
```

매니페스트 없이 실행하면 이 실행기는 **skip**된다. 일반 테스트 회차를 오염시키지 않는다.
