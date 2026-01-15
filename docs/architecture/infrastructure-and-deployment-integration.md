# Infrastructure and Deployment Integration

## Existing Infrastructure

**Current Deployment:** 현재 Boothy 리포 루트에는 제품용 CI/CD가 없고, RapidRAW 쪽에만 GitHub Actions 워크플로우가 포함되어 있습니다(단, `reference/uxui_presetfunction/.github/workflows/*`는 리포 루트가 아니라서 Boothy 리포의 CI로는 동작하지 않음).

**Infrastructure Tools:** (참고) RapidRAW는 GitHub Actions + `tauri-apps/tauri-action` 기반으로 멀티플랫폼 번들링(NSIS/dmg/AppImage 등)을 수행합니다.

**Environments:** MVP는 Windows-only이며, 네트워크 의존 없이 로컬 설치/실행이 핵심입니다(NFR1/NFR7).

## Enhancement Deployment Strategy

**Deployment Approach:**
- Boothy는 **Tauri 번들링(NSIS installer)**로 배포하며, 설치 대상은 “프로그램 파일(앱)” + “사용자 세션 폴더(사진 저장)” + “AppData 설정/로그”로 구분합니다.
- 카메라 기능은 별도 프로세스인 **Camera Sidecar Service**를 함께 설치/번들하고, Boothy 앱이 런타임에 sidecar를 자동 실행/감시합니다.

**Infrastructure Changes:**
- 리포 루트에 Boothy 전용 CI 워크플로우를 추가(Windows build + NSIS output)하고, 카메라 sidecar 빌드 산출물을 Boothy 번들 리소스에 포함합니다.
- sidecar에는 Canon EDSDK 등 네이티브 DLL이 필요하므로 x86/x64 정합성을 엄격히 관리합니다. 내부 매장 배포용 인스톨러는 sidecar와 함께 **필요 DLL을 번들링**합니다.
  - MVP 기준: digiCamControl 레퍼런스에 포함된 EDSDK DLL이 x86이므로, camera sidecar는 **x86 타겟**으로 빌드합니다(Windows 11 x64에서 정상 실행).

**Pipeline Integration (권장 구성):**
1. **Build boothy app** (`apps/boothy`)
   - Node(예: 22)로 프론트 빌드 → Rust로 Tauri build → NSIS 산출물 생성
2. **Build camera sidecar** (`apps/camera-sidecar`)
   - .NET 빌드(타겟 프레임워크/런타임은 sidecar 설계에 따름)
   - 산출물(Exe + DLL)을 Boothy 번들 리소스 경로로 복사
3. **Bundle**
   - Tauri `bundle.resources`에 sidecar 산출물을 포함
   - Boothy 런타임에서 sidecar를 “설치 디렉터리/리소스”에서 실행

**Runtime Layout (제안):**
- 설치 경로(예): `C:\\Program Files\\Boothy\\`  
  - `Boothy.exe` (Tauri)
  - `resources\\camera-sidecar\\Boothy.CameraSidecar.exe`
  - `resources\\camera-sidecar\\*.dll` (Canon EDSDK 등, 내부 매장 배포는 번들)
- 사용자 데이터(예): `%APPDATA%\\Boothy\\`
  - `settings.json` (RapidRAW + boothy 확장)
  - `logs\\boothy.log`, `logs\\camera-sidecar.log`
- 세션 데이터: `%USERPROFILE%\\Pictures\\dabi_shoot\\<session>\\{Raw,Jpg}`

**Operational Concerns (필수):**
- **버전 동기화:** Boothy 앱과 sidecar는 같은 릴리즈로 배포하고, sidecar는 `protocolVersion`/`appVersion` handshake로 불일치 시 명시적으로 실패(FR20, NFR8).
- **자가복구:** Boothy backend가 sidecar 프로세스를 감시하고 크래시 시 자동 재시작/재연결합니다.
- **오프라인:** 업데이트/다운로드 없이 동작해야 하며, 필드 진단은 로컬 로그로 해결합니다(NFR7/NFR8).

## Rollback Strategy

**Rollback Method:**
- NSIS 인스톨러 단위로 “이전 버전 재설치”를 공식 롤백으로 정의합니다(네트워크 없는 현장에서도 가능).
- 설정/스키마(`boothy.schemaVersion`)는 **하위호환(append-only)** 중심으로 설계하여, 롤백 시에도 기본 기능이 동작하도록 합니다(필요 시 “새 설정을 무시/리셋” 옵션 제공).

**Risk Mitigation:**
- sidecar IPC는 `protocolVersion`으로 강제하며, mismatch는 “정상적으로 촬영이 안 되는 불명확 상태”가 아니라 “업데이트 필요/호환 불가”로 명확히 표면화합니다.
- 세션 폴더/원본 데이터는 롤백과 독립적으로 유지되어야 합니다(원본/출력 손실 금지, NFR5).

**Monitoring:**
- 원격 모니터링은 MVP 범위 밖(오프라인)으로 두고, 로컬 로그/진단 파일의 품질을 품질게이트로 설정합니다(NFR8).

