# HV-16 T0 — 실제 입력 전제와 후보 결정표

작성: 2026-08-16 · Story 7.5 (상주 renderer 검증 spike) · 측정 PC에서 실제로 실행한 결과다.

## 0. 입력 조건 (다시 열지 않는다)

- Story 7.3 / **HV-14**가 `embedded-jpeg`, `camera-paired-jpeg`, `windows-shell-thumbnail`
  세 fast source를 전부 `Technology No-Go`로 닫았다. 105개 route row에서 accepted fast source는 **0건**이었다.
- 그 뒤 승인된 대체 route는 없다. 즉 **현재 승인된 빠른 raster 입력은 존재하지 않는다.**
- Story 7.4 / **HV-15**가 `raw-original + pinned darktable 5.4.1`을 production route로 승인했다.
- 이 문서의 predecoded/embedded raster 사용은 2026-08-16 승인에 따라 **engine feasibility 자료 전용**이며,
  HV-14 후보의 route 재승인이 아니다.

## 1. 실측 1 — pinned darktable one-shot display-fit 렌더

- 대상: HV-14 실촬영 corpus 35장 (Canon EOS 700D CR2)
- 명령: 제품과 동일한 display-fit invocation (`--width 1620 --height 1080 --upscale false --hq false
  --apply-custom-presets false --icc-type SRGB --icc-intent PERCEPTUAL`)
- 표본: 35 capture × 4 recipe × 3 repeat = **420회**. 느린 표본을 제외하지 않았다.
- 원자료: `baseline/darktable-oneshot-latency.csv`

| recipe | n | min | median | p95 | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| `preset-daylight` | 105 | 3395 ms | 3705 ms | 6553 ms | 10565 ms |
| `preset-mono-pop` | 105 | 3418 ms | 3710 ms | 4168 ms | 4616 ms |
| `preset-soft-glow` | 105 | 3454 ms | 3748 ms | 6627 ms | 11730 ms |
| **`default-render-template` (preset 연산 없음)** | 105 | 3373 ms | **3669 ms** | 4287 ms | 10382 ms |
| 전체 | 420 | 3373 ms | 3705 ms | 4995 ms | 11730 ms |

### 이 표의 결론

**preset 연산을 전부 빼도 렌더 시간이 거의 그대로다.** 세 preset의 median 3716 ms와
연산 없는 중립 렌더의 median 3669 ms의 차이는 **47 ms, 렌더 시간의 1.26%**다.

즉 darktable 한 번 실행의 **98.7%는 RAW 디코드와 process startup**이고,
temperature·exposure·sigmoid·bloom·sharpen·monochrome은 사실상 공짜다.

이것이 이 Story의 1순위 후보(WebGL2)에 대한 **결정적 관측**이다. WebGL2는 CR2를 직접 읽을 수 없으므로
누군가 먼저 raster를 만들어 줘야 하는데, 그 raster를 만드는 비용이 **preset까지 적용한 완성본을 만드는
비용과 같다.** 따라서 WebGL2가 preset 연산만 대신하면 절약되는 시간은 최대 47 ms다.

## 2. 실측 2 — Windows WIC 상주 direct CR2 디코드

측정 중 예상하지 못한 사실을 확인했다: **이 PC의 Windows는 CR2를 직접 디코드할 수 있다.**

- decoder: `Microsoft Raw Image Decoder`
- 제공: `Microsoft.RawImageExtension` **2.5.24.0** (Microsoft Store appx)
- 지원 확장자: `.CR2`, `.CR3`, `.NEF`, `.ARW`, `.DNG` 등 36종
- 결과: 5208×3476 `Rgb24`, in-process, 별도 process 없음
- 표본: 12 capture × 2 mode × 3 repeat = 72회. 원자료 `baseline/wic-cr2-decode.csv`

| mode | n | min | median | p95 | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| full (5208×3476) | 36 | 1208 ms | **1284 ms** | 2139 ms | 2264 ms |
| fit1620 (축소 디코드 요청) | 36 | 1240 ms | 1311 ms | 2200 ms | 2286 ms |

축소 요청이 **빨라지지 않는다.** WIC RAW decoder는 전체 demosaic을 먼저 끝내고 축소하므로,
목표 크기를 줄여서 얻는 이득이 없다.

## 3. 후보 결정표

| 항목 | WebGL2 (1순위) | WIC / Direct2D / D3D11 (조건부) | 현행 pinned darktable |
| --- | --- | --- | --- |
| 입력 형식 | RGBA raster only | CR2 직접 (Store appx 필요) | CR2 직접 |
| **CR2 직접 처리** | **불가** — WebView2/Chromium에 RAW decoder 없음 | **가능** (실측 확인) | 가능 |
| 별도 process 필요 | 입력 raster 생산자 필요 (= darktable 재실행) | 불필요 (in-process) | 필요 (매 촬영 1회) |
| hot-path 비용 | 상주 후 GPU pass만 (context/program 사전 준비) | 디코드 median 1284 ms + 파이프라인 | median 3705 ms |
| 색 관리 | shader 안에서 직접 구현. sRGB/perceptual만 | WIC/D2D 색 변환. darktable과 다른 파이프라인 | 승인된 참조 |
| 미지원 연산 | mask, blend 미지원 값, allowlist 밖 전부 | 미평가 (Direct2D effect 이식 필요) | 없음 (참조 자신) |
| fallback | 전부 정확 darktable 경로 | 전부 정확 darktable 경로 | — |
| **채택 판정** | **Technology No-Go (입력 부재)** | **평가 완료 / production No-Go (미승인 dependency + parity 미달)** | 유지 |

## 4. 남은 지연이 어디에 있는가 (승인 결정 #2가 요구한 지목)

Story 7.4 HV-15 실장비 median 종단 지연은 **8284 ms**다. 여기서:

| 시나리오 | 계산 | 남는 지연 | NFR-003 warm p50 3000 ms |
| --- | --- | ---: | --- |
| display 렌더가 **완전히 0**이 된다면 | 8284 − 3716 | **4568 ms** | 여전히 **미달** |
| 상주 direct decoder로 디코드만 남긴다면 | 8284 − (3716 − 1284) | **5855 ms** | 여전히 **미달** |
| preset 연산만 GPU로 옮긴다면 | 8284 − 47 | **8237 ms** | 여전히 **미달** |

**렌더러를 아무리 빠르게 만들어도 NFR-003의 warm p50 3초에 닿지 않는다.**
남은 4.5초 이상은 렌더 구간 밖에 있으며, 구간과 소유 Story는 다음과 같다.

| 남은 구간 | 성격 | 소유 Story |
| --- | --- | --- |
| 카메라 → PC RAW 전송 (USB, 19 MB/장) | 촬영 직후 helper 다운로드. HV-15에서 timeout 2회 관측 | **Story 7.8** (100-shot 성능·복구) |
| 렌더 큐 대기 (`queue_wait`) | 동시 in-flight 2 제한, 우선순위 없음 | **Story 7.6** (P0/P1/P2 scheduler, stale 취소) |
| 게시 (stage → probe → pointer commit → journal) | 파일 완결성 보장 구간 | Story 7.2 계약 유지, 성능은 **Story 7.6** |
| present (viewer decode → swap → actual-present) | HV-13B에서 계측됨 | **Story 7.8** (물리 프레임은 HV-18B) |

이 지목이 없으면 "렌더러가 빨라졌다"만 남고 Story가 닫히지 않는다는 조건(승인 결정 #2)을 충족한다.

## 5. predecoded corpus 라벨링

`baseline/`과 `parity/`의 모든 산출물은 **engine feasibility 자료**다.
`RESIDENT_APPROVED_DIRECT_DECODERS`가 비어 있는 한, 어떤 상주 후보도 `productionEligible`을
주장할 수 없으며 이는 host와 TS 계약 양쪽에서 기계적으로 차단된다
(`validate_resident_producer_provenance`, `residentProducerProvenanceSchema`).

## 6. 새 RAW decoder 도입 범위 (별도 승인 필요, 이 Story에서 도입하지 않음)

WIC direct decode를 production route로 올리려면 아래를 적어 **별도 승인**을 받아야 한다.

| 항목 | 현재 확인된 사실 | 미확인 |
| --- | --- | --- |
| dependency | `Microsoft.RawImageExtension` 2.5.24.0 (Store appx) | 오프라인 부스에서의 설치 경로 |
| installer | Store 배포. 일반 offline installer에 동봉 불가 | 기업 배포(오프라인 라이선스) 가능 여부 → **Story 7.7** |
| 라이선스 | Microsoft Store 약관 | 재배포 조건 |
| 성능 | median 1284 ms (실측) | 부스 PC 전원/열 조건에서의 p95 |
| parity | SSIM 0.013–0.159, ΔE00 median 22.96–26.56 (아래 §7) | 파이프라인 이식 후 재측정 |
| 카메라 호환 | EOS 700D CR2 확인 | 향후 기종 |

## 7. parity 실측 (bounded WIC 평가)

- 후보: WIC direct CR2 디코드 (preset 미적용)
- 참조: pinned darktable 5.4.1 display-fit 렌더
- 표본: 6 capture × 3 preset = 18쌍 + 중립 6쌍. 원자료 `parity/*.json`
- 지표 구현: `src/quality-metrics/parity-metrics.ts` (CIEDE2000은 Sharma 공개 검증 표 17쌍으로 단위 테스트)

| 참조 | n | SSIM (median) | ΔE00 median | ΔE00 p95 | clipping Δ |
| --- | ---: | ---: | ---: | ---: | ---: |
| darktable 중립 렌더 | 6 | 0.021 | 26.20 | 33.86 | −37.66 %p |
| `preset-daylight` | 6 | 0.022 | 26.07 | 33.79 | −41.14 %p |
| `preset-mono-pop` | 6 | 0.013 | 26.56 | 34.27 | −56.33 %p |
| `preset-soft-glow` | 6 | 0.159 | 22.96 | 32.62 | −2.38 %p |

기준선은 SSIM ≥ 0.95, ΔE00 median ≤ 3, p95 ≤ 8이다. **모든 표본이 큰 차이로 미달한다.**

**이 숫자를 과장하지 않기 위한 단서:**

1. WIC 산출물에는 preset이 적용되어 있지 않다. 따라서 이 값은 "디코더만의 오차"가 아니라
   **상주 WIC 파이프라인이 메워야 할 전체 격차**다.
2. corpus는 HV-15에서 심한 저노출로 기록된 그 corpus다 (평균 luma 12/255 수준).
   any-channel 기준에서 darktable 참조는 WIC 후보보다 경계 채널이 많으며,
   `preset-mono-pop`의 clipping 차이는 −56.33 %p다.
3. 따라서 이 표는 "WIC가 나쁘다"의 증거가 아니라 **"두 파이프라인은 서로 drop-in이 아니다"**의 증거이며,
   채택하려면 별도의 색 파이프라인 이식과 재측정이 필요하다는 뜻이다.

## 8. T0 판정

- 실제 촬영 입력을 **직접** 처리할 수 있는 후보는 WIC/Direct2D 계열뿐이며, 그 dependency는 미승인이다.
- 1순위 WebGL2는 입력 축에서 막힌다. 엔진을 아무리 최적화해도 절약 상한이 47 ms다.
- 따라서 승인 결정 #1에 따라 **구현을 억지로 확장하지 않고**
  `Technology No-Go + darktable exact fallback`으로 닫을 수 있다.
- 상세 판정은 `decision.md`.
