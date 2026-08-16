# HV-17 camera preflight

한 대의 Canon 카메라에서 image quality, Av, Tv, ISO의 현재값과 descriptor를 읽어 JSON으로
남긴다. `--probe-only`는 설정을 바꾸지 않는다. 적용 모드에서는 descriptor가 보고한 RAW-only를
고르고, 요청한 노출 코드가 descriptor의 **정확한 원소**일 때만 적용한 뒤 readback한다.

```powershell
$env:BOOTHY_CANON_SDK_ROOT = 'C:\Code\cannon_sdk\canon-edsdk'
dotnet run --project .\tools\camera-preflight\camera-preflight.csproj -- `
  --probe-only --out C:\absolute\run-hv17\camera-preflight-probe.json

dotnet run --project .\tools\camera-preflight\camera-preflight.csproj -- `
  --av 0x28 --tv 0x5D --iso 0x60 `
  --out C:\absolute\run-hv17\camera-preflight-applied.json
```

코드 값은 후보일 뿐이며 카메라 descriptor에 없으면 실패한다. 적용 회차는 원래값(`before`)과
요청·readback(`requested`/`after`)을 모두 보존하며 자동 복구하지 않는다.
