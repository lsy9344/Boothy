use rusqlite::{params, Connection, Transaction};

use crate::diagnostics::error::OperationalLogError;

type MigrationSql = &'static str;

struct Migration {
    version: i64,
    name: &'static str,
    sql: MigrationSql,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "0001_init.sql",
    sql: include_str!("../../migrations/0001_init.sql"),
}];

#[derive(Debug, PartialEq, Eq)]
struct AppliedMigration {
    version: i64,
    name: String,
}

pub fn apply_pending_migrations(connection: &mut Connection) -> Result<(), OperationalLogError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
    )?;

    let applied = load_applied_migrations(connection)?;
    validate_migration_state(&applied)?;

    let last_applied_version = applied.last().map(|migration| migration.version).unwrap_or_default();
    let pending = MIGRATIONS
        .iter()
        .filter(|migration| migration.version > last_applied_version)
        .collect::<Vec<_>>();

    if pending.is_empty() {
        return Ok(());
    }

    let transaction = connection.transaction()?;
    for migration in pending {
        apply_migration(&transaction, migration)?;
    }
    transaction.commit()?;

    Ok(())
}

fn load_applied_migrations(connection: &Connection) -> Result<Vec<AppliedMigration>, OperationalLogError> {
    let mut statement =
        connection.prepare("SELECT version, name FROM schema_migrations ORDER BY version ASC")?;
    let rows = statement.query_map([], |row| {
        Ok(AppliedMigration {
            version: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let mut applied = Vec::new();
    for row in rows {
        applied.push(row?);
    }

    Ok(applied)
}

fn validate_migration_state(applied: &[AppliedMigration]) -> Result<(), OperationalLogError> {
    for (index, migration) in applied.iter().enumerate() {
        let expected = (index as i64) + 1;
        if migration.version != expected {
            return Err(OperationalLogError::migration_invalid_state(format!(
                "invalid migration state: expected version {expected}, found {}",
                migration.version
            )));
        }

        let known = MIGRATIONS
            .iter()
            .find(|candidate| candidate.version == migration.version)
            .ok_or_else(|| {
                OperationalLogError::migration_invalid_state(format!(
                    "invalid migration state: unknown migration version {}",
                    migration.version
                ))
            })?;

        if known.name != migration.name {
            return Err(OperationalLogError::migration_invalid_state(format!(
                "invalid migration state: expected migration {} for version {}, found {}",
                known.name, known.version, migration.name
            )));
        }
    }

    Ok(())
}

fn apply_migration(transaction: &Transaction<'_>, migration: &Migration) -> Result<(), OperationalLogError> {
    transaction.execute_batch(migration.sql)?;
    transaction.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
        params![migration.version, migration.name],
    )?;
    Ok(())
}
