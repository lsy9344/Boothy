use std::{fs, path::{Path, PathBuf}};

use rusqlite::{Connection, OpenFlags};
use tauri::{AppHandle, Manager, Runtime};

use crate::{db::migrations::apply_pending_migrations, diagnostics::error::OperationalLogError};

const OPERATIONAL_LOG_FILENAME: &str = "operational-log.sqlite3";

#[derive(Clone, Debug)]
pub struct OperationalLogState {
    db_path: PathBuf,
}

impl OperationalLogState {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

pub fn resolve_operational_log_db_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join(OPERATIONAL_LOG_FILENAME)
}

pub fn open_operational_log_connection(path: &Path) -> Result<Connection, OperationalLogError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?;

    configure_connection(&connection)?;

    Ok(connection)
}

pub fn initialize_operational_log<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<OperationalLogState, OperationalLogError> {
    let app_local_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|error| OperationalLogError::initialization(format!("failed to resolve app-local data path: {error}")))?;
    let db_path = resolve_operational_log_db_path(&app_local_data_dir);
    let mut connection = open_operational_log_connection(&db_path)?;

    apply_pending_migrations(&mut connection)?;

    Ok(OperationalLogState::new(db_path))
}

fn configure_connection(connection: &Connection) -> Result<(), OperationalLogError> {
    let journal_mode = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get::<_, String>(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(OperationalLogError::initialization(format!(
            "failed to enable WAL journal mode, received {journal_mode}"
        )));
    }

    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "busy_timeout", 5_000_i64)?;

    Ok(())
}
