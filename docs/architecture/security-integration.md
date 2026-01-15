# Security Integration

## Existing Security Measures

**Authentication:** Boothy MVP는 **계정 로그인 없이** 동작하는 오프라인 앱입니다(NFR7). 참고로 RapidRAW 레퍼런스에는 계정/온라인 기능이 포함될 수 있으나, Boothy 제품 빌드에서는 제거/비활성화하여 “네트워크 없이 기본 기능이 완전 동작”하도록 고정합니다.

**Authorization:** 기존 앱은 역할 기반 권한 모델이 핵심 개념은 아니며, UI 노출/기능 접근 제어는 앱 내부 설정/상태로 결정됩니다.

**Data Protection:** 사진/프리셋/설정은 로컬 파일시스템에 저장됩니다(이미지 파일 + `.rrdata` 사이드카 + AppData의 `settings.json`). 기본적으로 “암호화 저장”은 전제되지 않습니다.

**Security Tools:** Tauri의 capability 기반 권한 모델(`src-tauri/capabilities/*`)과 앱 샌드박스(로컬 번들) 구조를 사용합니다. 다만 RapidRAW 기본 capability에는 `shell`/`process` 권한이 포함되어 있어, Boothy에서는 “최소 권한(least privilege)” 관점에서 재평가가 필요합니다.

## Enhancement Security Requirements

**New Security Measures:**
- **Admin 모드 인증:** admin 비밀번호는 **argon2id**(salt 포함)로 해시 저장하고, 평문 저장/로그를 금지합니다(NFR6).  
- **접근 제한 목적(UX 중심):** customer/admin “숨김”은 “일반 사용자(비전문가)가 실수로 고급 기능을 사용하지 않도록” 하는 목적입니다. MVP 범위에서는 **UI 숨김 중심**으로 구현하고, 악의적/고급 사용자의 우회 호출(개발자 도구/IPC 직접 호출 등) 방지는 별도 보안 하드닝 범위로 둡니다.
- **IPC 접근 제어:** Camera Sidecar의 Named Pipe는 **현재 사용자 세션만 접근** 가능하도록 ACL을 제한합니다(권장). 또한 메시지에 `protocolVersion`/`requestId`/`correlationId`를 포함해 오작동/조사 가능성을 높입니다(NFR8).
- **경로/입력 검증:** `sessionName`/세션 폴더명, `Raw/`/`Jpg/` 경로는 backend에서 검증하며, 파일 삭제/이동은 **활성 세션 루트 하위**로 강제하여 path traversal/오작동을 방지합니다(FR13, NFR5).
- **Tauri Capability 최소화:** Boothy에서 필요 없는 `shell`/`process` 권한을 제거/축소하고, 파일/OS 접근도 필요한 범위만 허용합니다(least privilege).
- **공급망/배포 신뢰:** Windows 배포(NSIS)는 가능하면 **코드 서명**을 적용하고, 번들에 포함되는 sidecar/SDK DLL의 출처/버전을 릴리즈 노트와 해시로 추적합니다(운영 안정성).

**Integration Points:**
- `settings.json`(AppData)에 `boothy.adminPassword`(argon2 파라미터+salt+hash) 저장
- UI 모드 토글 → backend `boothy_admin_login`/`boothy_set_mode` → **UI 노출(visibility) 제어**에 반영
- Tauri backend ↔ sidecar IPC: pipe ACL + 버전드 프로토콜 + 에러 코드 표준화
- 파일시스템 세션 계약: `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}` 경로 검증/강제

**Compliance Requirements:**
- **Offline-first:** 핵심 기능은 오프라인에서 완전 동작(NFR7)
- **Windows-only:** 플랫폼 제약 준수(NFR1)
- **License/Distribution:** RapidRAW(AGPL-3.0) 의무를 수용하고(고지+소스 제공), Canon EDSDK는 내부 매장 배포에 번들링합니다. 외부/공개 배포는 별도 승인 전까지 하지 않습니다.

## Security Testing

**Existing Security Tests:** 현재 리포에서 자동화된 보안 테스트 체계는 명확히 확인되지 않습니다.

**New Security Test Requirements (MVP):**
- **Password storage:** `settings.json`에 평문 비밀번호가 저장/로그되지 않는지 검증(NFR6)
- **UI hiding:** customer 모드에서 admin 전용 UI/컨트롤이 “비활성화”가 아니라 “숨김”으로 적용되는지 검증(FR17)
- **IPC hardening:** 다른 Windows 사용자/프로세스가 pipe에 접근 가능한지(ACL) 점검, 프로토콜 버전 mismatch 처리 확인
- **Path safety:** 삭제/이동/Export 경로가 세션 루트 밖으로 나갈 수 없는지(경로 정규화/canonicalize) 검증
- **Crash resilience:** sidecar 크래시/연결 끊김 시 앱이 크래시 없이 오류를 표면화하고 계속 탐색/Export 가능한지(FR20)

**Penetration Testing:** MVP의 보안 목표는 “악의적 공격 방어”가 아니라 “일반 사용자 오사용 방지(UX)”이므로, 전통적 펜테스트는 MVP 범위 밖으로 둡니다. 대신 경로 안전성/크래시 내성/로그 품질을 우선 검증합니다.

