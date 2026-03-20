use serde::{Deserialize, Serialize};

use crate::session::session_manifest::{
    ActivePresetBinding, SessionCaptureRecord, SessionManifest,
};

const SESSION_ID_PREFIX: &str = "session_";
const PRESET_ID_PREFIX: &str = "preset_";

pub fn is_valid_session_id(session_id: &str) -> bool {
    let suffix = match session_id.strip_prefix(SESSION_ID_PREFIX) {
        Some(suffix) => suffix,
        None => return false,
    };

    suffix.len() == 26 && suffix.chars().all(|char| char.is_ascii_alphanumeric())
}

pub fn is_valid_preset_id(preset_id: &str) -> bool {
    let suffix = match preset_id.strip_prefix(PRESET_ID_PREFIX) {
        Some(suffix) => suffix,
        None => return false,
    };

    !suffix.is_empty()
        && suffix
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-')
}

pub fn is_valid_published_version(published_version: &str) -> bool {
    if published_version.len() != 10 {
        return false;
    }

    published_version
        .chars()
        .enumerate()
        .all(|(index, char)| match index {
            4 | 7 => char == '.',
            _ => char.is_ascii_digit(),
        })
}

pub fn is_non_blank(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn validate_session_id(session_id: &str) -> Result<(), HostErrorEnvelope> {
    if is_valid_session_id(session_id) {
        Ok(())
    } else {
        Err(HostErrorEnvelope::validation_message(
            "세션 정보를 다시 확인해 주세요.",
        ))
    }
}

pub fn validate_preset_selection_input(
    preset_id: &str,
    published_version: &str,
) -> Result<(), HostErrorEnvelope> {
    if !is_valid_preset_id(preset_id) || !is_valid_published_version(published_version) {
        return Err(HostErrorEnvelope::validation_message(
            "프리셋 정보를 다시 확인해 주세요.",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartInputDto {
    pub name: String,
    pub phone_last_four: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPresetCatalogInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPreviewAssetDto {
    pub kind: String,
    pub asset_path: String,
    pub alt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPresetSummaryDto {
    pub preset_id: String,
    pub display_name: String,
    pub published_version: String,
    pub booth_status: String,
    pub preview: PresetPreviewAssetDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCatalogResultDto {
    pub session_id: String,
    pub state: String,
    pub presets: Vec<PublishedPresetSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSelectionInputDto {
    pub session_id: String,
    pub preset_id: String,
    pub published_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSelectionResultDto {
    pub session_id: String,
    pub active_preset: ActivePresetBinding,
    pub manifest: SessionManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequestInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessDto {
    pub schema_version: String,
    pub session_id: String,
    pub surface_state: String,
    pub customer_state: String,
    pub can_capture: bool,
    pub primary_action: String,
    pub customer_message: String,
    pub support_message: String,
    pub reason_code: String,
    pub latest_capture: Option<SessionCaptureRecord>,
}

impl CaptureReadinessDto {
    fn build(
        session_id: impl Into<String>,
        surface_state: impl Into<String>,
        customer_state: impl Into<String>,
        can_capture: bool,
        primary_action: impl Into<String>,
        customer_message: impl Into<String>,
        support_message: impl Into<String>,
        reason_code: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        Self {
            schema_version: "capture-readiness/v1".into(),
            session_id: session_id.into(),
            surface_state: surface_state.into(),
            customer_state: customer_state.into(),
            can_capture,
            primary_action: primary_action.into(),
            customer_message: customer_message.into(),
            support_message: support_message.into(),
            reason_code: reason_code.into(),
            latest_capture,
        }
    }

    pub fn preset_missing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "choose-preset",
            "촬영 전에 룩을 먼저 골라 주세요.",
            "선택이 끝나면 바로 찍을 수 있어요.",
            "preset-missing",
            None,
        )
    }

    pub fn camera_preparing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "촬영 준비 중이에요.",
            "잠시만 기다려 주세요.",
            "camera-preparing",
            None,
        )
    }

    pub fn helper_preparing(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "촬영 준비 중이에요.",
            "잠시만 기다려 주세요.",
            "helper-preparing",
            None,
        )
    }

    pub fn preview_waiting(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "previewWaiting",
            "Preview Waiting",
            false,
            "wait",
            "사진이 안전하게 저장되었어요.",
            "확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.",
            "preview-waiting",
            latest_capture,
        )
    }

    pub fn export_waiting(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "previewWaiting",
            "Preparing",
            false,
            "wait",
            "마무리 준비 중이에요.",
            "다음 안내가 나올 때까지 기다려 주세요.",
            "export-waiting",
            latest_capture,
        )
    }

    pub fn completed(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Preparing",
            false,
            "wait",
            "이 촬영은 마무리되었어요.",
            "다음 안내를 확인해 주세요.",
            "completed",
            latest_capture,
        )
    }

    pub fn phone_required(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "blocked",
            "Phone Required",
            false,
            "call-support",
            "지금은 도움이 필요해요.",
            "가까운 직원에게 알려 주세요.",
            "phone-required",
            None,
        )
    }

    pub fn warning(
        session_id: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "captureReady",
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "남은 시간 안에 계속 찍을 수 있어요.",
            "warning",
            latest_capture,
        )
    }

    pub fn ready(
        session_id: impl Into<String>,
        surface_state: impl Into<String>,
        latest_capture: Option<SessionCaptureRecord>,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            surface_state,
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "버튼을 누르면 바로 시작돼요.",
            "ready",
            latest_capture,
        )
    }

    pub fn capture_saved(
        session_id: impl Into<String>,
        latest_capture: SessionCaptureRecord,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "captureSaved",
            "Preview Waiting",
            false,
            "wait",
            "사진이 안전하게 저장되었어요.",
            "확인용 사진을 준비하고 있어요. 잠시만 기다려 주세요.",
            "preview-waiting",
            Some(latest_capture),
        )
    }

    pub fn preview_ready(
        session_id: impl Into<String>,
        latest_capture: SessionCaptureRecord,
    ) -> Self {
        let session_id = session_id.into();

        Self::build(
            session_id,
            "previewReady",
            "Ready",
            true,
            "capture",
            "지금 촬영할 수 있어요.",
            "방금 찍은 사진을 아래에서 바로 확인할 수 있어요.",
            "ready",
            Some(latest_capture),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReadinessUpdateDto {
    pub schema_version: String,
    pub session_id: String,
    pub readiness: CaptureReadinessDto,
}

impl CaptureReadinessUpdateDto {
    pub fn new(session_id: impl Into<String>, readiness: CaptureReadinessDto) -> Self {
        Self {
            schema_version: "capture-readiness-update/v1".into(),
            session_id: session_id.into(),
            readiness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequestResultDto {
    pub schema_version: String,
    pub session_id: String,
    pub status: String,
    pub capture: SessionCaptureRecord,
    pub readiness: CaptureReadinessDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFieldErrors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_last_four: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostErrorEnvelope {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness: Option<CaptureReadinessDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<HostFieldErrors>,
}

impl HostErrorEnvelope {
    pub fn validation_message(message: impl Into<String>) -> Self {
        Self {
            code: "validation-error".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn validation(field_errors: HostFieldErrors) -> Self {
        Self {
            code: "validation-error".into(),
            message: "입력한 내용을 다시 확인해 주세요.".into(),
            readiness: None,
            field_errors: Some(field_errors),
        }
    }

    pub fn persistence(message: impl Into<String>) -> Self {
        Self {
            code: "session-persistence-failed".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn session_not_found(message: impl Into<String>) -> Self {
        Self {
            code: "session-not-found".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn preset_catalog_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "preset-catalog-unavailable".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn preset_not_available(message: impl Into<String>) -> Self {
        Self {
            code: "preset-not-available".into(),
            message: message.into(),
            readiness: None,
            field_errors: None,
        }
    }

    pub fn capture_not_ready(message: impl Into<String>, readiness: CaptureReadinessDto) -> Self {
        Self {
            code: "capture-not-ready".into(),
            message: message.into(),
            readiness: Some(readiness),
            field_errors: None,
        }
    }
}
