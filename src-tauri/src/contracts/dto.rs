use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartInputDto {
  pub name: String,
  pub phone_last_four: String,
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
  pub field_errors: Option<HostFieldErrors>,
}

impl HostErrorEnvelope {
  pub fn validation(field_errors: HostFieldErrors) -> Self {
    Self {
      code: "validation-error".into(),
      message: "입력한 내용을 다시 확인해 주세요.".into(),
      field_errors: Some(field_errors),
    }
  }

  pub fn persistence(message: impl Into<String>) -> Self {
    Self {
      code: "session-persistence-failed".into(),
      message: message.into(),
      field_errors: None,
    }
  }
}
