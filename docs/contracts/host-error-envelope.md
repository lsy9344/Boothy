# Host Error Envelope 계약

## 목적

이 문서는 frontend가 host boundary에서 받는 customer-safe error envelope baseline을 고정한다.
Story 1.14의 목표는 raw helper stderr나 ad hoc object return을 늘리는 대신, 허용된 오류 shape를 문서와 shared schema로 닫는 것이다.

## Authoritative Source / 소비 경계

- 문서 기준: 이 문서
- TypeScript 기준: `src/shared-contracts/errors/index.ts`
- Rust 기준: `src-tauri/src/contracts/dto.rs`
- 대표 fixture: `tests/fixtures/contracts/host-error-envelope-capture-not-ready.json`

## Envelope Shape

```json
{
  "code": "capture-not-ready",
  "message": "사진이 아직 준비되지 않았어요.",
  "readiness": {
    "schemaVersion": "capture-readiness/v1",
    "sessionId": "session_01hs6n1r8b8zc5v4ey2x7b9g1m",
    "surfaceState": "previewWaiting",
    "customerState": "Preview Waiting",
    "canCapture": false,
    "primaryAction": "wait",
    "customerMessage": "사진이 안전하게 저장되었어요.",
    "supportMessage": "확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.",
    "reasonCode": "preview-waiting",
    "latestCapture": null
  }
}
```

## 최소 필드

- `code`: machine-readable host error code
- `message`: customer-safe 또는 operator-safe 요약 메시지
- `readiness`: 선택 필드. capture / booth state를 안전하게 다시 안내해야 할 때만 포함
- `fieldErrors`: validation error일 때만 포함되는 field-level safe guidance

## 허용 범위

- 허용되는 `code`는 shared schema와 Rust DTO가 함께 소유한다.
- `message`는 customer/operator-safe copy여야 하며 아래를 직접 노출하면 안 된다.
  - raw helper stderr
  - Canon SDK 내부 코드 원문
  - filesystem absolute path 디버그 덤프
  - implementation stack trace
- `fieldErrors`는 현재 `name`, `phoneLastFour`의 bounded validation copy만 허용한다.
- `readiness`가 포함되면 반드시 `capture-readiness/v1`을 따라야 한다.

## 범위 경계

- Story 1.14가 닫는 범위:
  - envelope shape
  - safe message / readiness / fieldErrors baseline
- Story 1.15 이후가 닫는 범위:
  - helper-specific deeper diagnostic enrichment이 필요할 때도 이 envelope를 깨지 않는 확장 방식
