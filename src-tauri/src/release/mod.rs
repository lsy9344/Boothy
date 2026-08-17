//! Story 7.7. 설치본이 자기가 담고 있다고 주장한 것을 실제로 담고 있는지 확인한다.
//!
//! **도메인 로직은 여기 있고 `commands/`에는 없다.** self-check는 Tauri 빌더를 세우기
//! 전에 도는 진입점이라 command가 아니다.
//!
//! - `inventory`: 인벤토리 매니페스트 읽기와 실제 파일 대조
//! - `self_check`: `--self-check` 진입점과 `install-self-check/v1` 보고서

pub mod inventory;
pub mod self_check;

/// 번들 설정 원문. 앱 버전과 identifier의 **단일 진실**이다.
/// 여기서 다시 상수로 베껴 적으면 설정과 코드가 갈라진다.
const TAURI_CONF: &str = include_str!("../../tauri.conf.json");

/// 설치본이 스스로를 어떻게 부르는가. 보고서와 진단 기록이 이 값을 쓴다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleIdentity {
    pub app_version: String,
    pub identifier: String,
    pub product_name: String,
}

pub fn bundle_identity() -> BundleIdentity {
    parse_bundle_identity(TAURI_CONF)
}

pub fn parse_bundle_identity(config_source: &str) -> BundleIdentity {
    let parsed: serde_json::Value = serde_json::from_str(config_source).unwrap_or_default();

    BundleIdentity {
        app_version: parsed
            .get("version")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        identifier: parsed
            .get("identifier")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        product_name: parsed
            .get("productName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Boothy")
            .to_string(),
    }
}
