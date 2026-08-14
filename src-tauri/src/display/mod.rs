//! Story 7.2: immutable display generation과 atomic display pointer.
//!
//! 이 모듈은 **어떤 자산이 지금 고객 화면의 진실인가**만 소유한다.
//! 실제 preset 렌더러(Story 7.4), RAW 정밀본 tier와 deadline scheduler(Story 7.6)는
//! 여기에 들어오지 않는다. Story 7.2는 renderer를 교체하지 않고 표시 종단점을 증명한다.

pub mod display_artifact;
pub mod generation_repository;
pub mod image_probe;
pub mod sample_publisher;

use std::sync::Mutex;

pub use display_artifact::DisplayState;

/// Tauri `manage`로 등록되는 프로세스 전역 display pointer truth.
///
/// pointer 갱신은 이 Mutex 안에서만 일어난다 (single writer). 게시가 직렬화되므로
/// generation seq 예약과 승격 판정 사이에 경합이 생기지 않는다.
#[derive(Default)]
pub struct DisplayStateHandle(pub Mutex<DisplayState>);
