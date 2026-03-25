use std::sync::Mutex;

pub mod ingest_pipeline;
pub mod normalized_state;

pub(crate) static CAPTURE_PIPELINE_LOCK: Mutex<()> = Mutex::new(());
