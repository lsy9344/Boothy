use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

pub mod embedded_jpeg;
pub mod helper_supervisor;
pub mod ingest_pipeline;
pub mod normalized_state;
pub mod sidecar_client;
pub mod source_probe;
pub mod source_telemetry;

pub(crate) static CAPTURE_PIPELINE_LOCK: Mutex<()> = Mutex::new(());
pub(crate) static IN_FLIGHT_CAPTURE_SESSIONS: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
