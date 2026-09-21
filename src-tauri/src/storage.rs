use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Row};

use crate::providers::usage::{
    ProviderUsage, UsagePeriod, UsageSource, UsageStatus, UsageUnit, UsageWindow,
};
use crate::providers::ProviderId;

const SCHEMA_VERSION: i64 = 1;

/// Local, credential-free persistence for usage snapshots.
///
/// The database stores only normalized quota data and opaque account
/// fingerprints. Provider credential files are never copied into it.
pub struct Storage {
    connection: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let connection = Connection::open(path)?;
        configure_connection(&connection)?;
        let storage = Self {
            connection: Mutex::new(connection),
        };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn in_memory() -> SqlResult<Self> {
        let connection = Connection::open_in_memory()?;
        configure_connection(&connection)?;
        let storage = Self {
            connection: Mutex::new(connection),
        };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> SqlResult<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        connection.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS usage_snapshots (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 installation_id TEXT NOT NULL,
                 provider TEXT NOT NULL,
                 runtime_id TEXT NOT NULL,
                 account_key TEXT,
                 status TEXT NOT NULL,
                 percentage REAL,
                 used REAL,
                 remaining REAL,
                 quota_limit REAL,
                 unit TEXT,
                 period TEXT,
                 period_description TEXT,
                 reset_at TEXT,
                 weekly_percentage REAL,
                 weekly_period TEXT,
                 weekly_period_description TEXT,
                 weekly_reset_at TEXT,
                 updated_at TEXT NOT NULL,
                 source TEXT NOT NULL,
                 error TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_usage_snapshots_installation_updated
                 ON usage_snapshots (installation_id, updated_at DESC);
             CREATE TABLE IF NOT EXISTS last_valid_usage (
                 installation_id TEXT PRIMARY KEY,
                 snapshot_id INTEGER NOT NULL REFERENCES usage_snapshots(id),
                 updated_at TEXT NOT NULL
             );
             COMMIT;",
        )?;
        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations(version, applied_at)
             VALUES (?1, datetime('now'))",
            params![SCHEMA_VERSION],
        )?;

        Ok(())
    }

    pub fn save_valid_usage(&self, usage: &ProviderUsage) -> SqlResult<i64> {
        if usage.error.is_some()
            || matches!(usage.status, UsageStatus::Error | UsageStatus::Unavailable)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "only valid usage can be persisted".into(),
            ));
        }

        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.unchecked_transaction()?;
        let weekly = usage.weekly.as_ref();

        transaction.execute(
            "INSERT INTO usage_snapshots (
                 installation_id, provider, runtime_id, account_key, status,
                 percentage, used, remaining, quota_limit, unit, period,
                 period_description, reset_at, weekly_percentage, weekly_period,
                 weekly_period_description, weekly_reset_at, updated_at, source, error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                       ?14, ?15, ?16, ?17, ?18, ?19, NULL)",
            params![
                usage.installation_id,
                provider_name(usage.provider),
                usage.runtime_id,
                usage.account_key,
                status_name(usage.status),
                usage.percentage,
                usage.used,
                usage.remaining,
                usage.limit,
                usage.unit.map(unit_name),
                usage.period.map(period_name),
                usage.period_description,
                usage.reset_at,
                weekly.map(|window| window.percentage),
                weekly.map(|window| period_name(window.period)),
                weekly.map(|window| window.period_description.clone()),
                weekly.and_then(|window| window.reset_at.clone()),
                usage.updated_at,
                source_name(usage.source),
            ],
        )?;

        let snapshot_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO last_valid_usage(installation_id, snapshot_id, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(installation_id) DO UPDATE SET
                 snapshot_id = excluded.snapshot_id,
                 updated_at = excluded.updated_at",
            params![usage.installation_id, snapshot_id, usage.updated_at],
        )?;
        transaction.commit()?;
        Ok(snapshot_id)
    }

    pub fn last_valid_usage(&self, installation_id: &str) -> SqlResult<Option<ProviderUsage>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection
            .query_row(
                "SELECT s.provider, s.installation_id, s.runtime_id, s.account_key,
                        s.status, s.percentage, s.used, s.remaining, s.quota_limit,
                        s.unit, s.period, s.period_description, s.reset_at,
                        s.weekly_percentage, s.weekly_period, s.weekly_period_description,
                        s.weekly_reset_at, s.updated_at, s.source, s.error
                 FROM last_valid_usage l
                 JOIN usage_snapshots s ON s.id = l.snapshot_id
                 WHERE l.installation_id = ?1",
                params![installation_id],
                usage_from_row,
            )
            .optional()
    }

    #[cfg(test)]
    fn scalar_i64(&self, sql: &str) -> SqlResult<i64> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.query_row(sql, [], |row| row.get(0))
    }
}

fn configure_connection(connection: &Connection) -> SqlResult<()> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA journal_mode = WAL;",
    )?;
    Ok(())
}

fn usage_from_row(row: &Row<'_>) -> SqlResult<ProviderUsage> {
    let provider: String = row.get(0)?;
    let status: String = row.get(4)?;
    let unit: Option<String> = row.get(9)?;
    let period: Option<String> = row.get(10)?;
    let weekly_percentage: Option<f64> = row.get(13)?;
    let weekly_period: Option<String> = row.get(14)?;

    let weekly = match (weekly_percentage, weekly_period) {
        (Some(percentage), Some(period)) => Some(UsageWindow {
            percentage,
            period: parse_period(&period)?,
            period_description: row
                .get::<_, Option<String>>(15)?
                .unwrap_or_else(|| "current window".into()),
            reset_at: row.get(16)?,
        }),
        _ => None,
    };

    Ok(ProviderUsage {
        provider: parse_provider(&provider)?,
        installation_id: row.get(1)?,
        runtime_id: row.get(2)?,
        account_key: row.get(3)?,
        status: parse_status(&status)?,
        percentage: row.get(5)?,
        used: row.get(6)?,
        remaining: row.get(7)?,
        limit: row.get(8)?,
        unit: unit.as_deref().map(parse_unit).transpose()?,
        period: period.as_deref().map(parse_period).transpose()?,
        period_description: row.get(11)?,
        reset_at: row.get(12)?,
        weekly,
        updated_at: row.get(17)?,
        source: parse_source(&row.get::<_, String>(18)?)?,
        error: row.get(19)?,
    })
}

fn invalid_value(value: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(format!("unsupported stored value: {value}"))
}

fn provider_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Claude => "claude",
        ProviderId::Codex => "codex",
        ProviderId::Gemini => "gemini",
        ProviderId::Grok => "grok",
    }
}

fn parse_provider(value: &str) -> SqlResult<ProviderId> {
    match value {
        "claude" => Ok(ProviderId::Claude),
        "codex" => Ok(ProviderId::Codex),
        "gemini" => Ok(ProviderId::Gemini),
        "grok" => Ok(ProviderId::Grok),
        _ => Err(invalid_value(value)),
    }
}

fn status_name(status: UsageStatus) -> &'static str {
    match status {
        UsageStatus::Ok => "ok",
        UsageStatus::Warning => "warning",
        UsageStatus::Critical => "critical",
        UsageStatus::Unknown => "unknown",
        UsageStatus::Unavailable => "unavailable",
        UsageStatus::Error => "error",
    }
}

fn parse_status(value: &str) -> SqlResult<UsageStatus> {
    match value {
        "ok" => Ok(UsageStatus::Ok),
        "warning" => Ok(UsageStatus::Warning),
        "critical" => Ok(UsageStatus::Critical),
        "unknown" => Ok(UsageStatus::Unknown),
        "unavailable" => Ok(UsageStatus::Unavailable),
        "error" => Ok(UsageStatus::Error),
        _ => Err(invalid_value(value)),
    }
}

fn unit_name(unit: UsageUnit) -> &'static str {
    match unit {
        UsageUnit::Percentage => "percentage",
        UsageUnit::Tokens => "tokens",
        UsageUnit::Requests => "requests",
        UsageUnit::Credits => "credits",
        UsageUnit::Messages => "messages",
        UsageUnit::Unknown => "unknown",
    }
}

fn parse_unit(value: &str) -> SqlResult<UsageUnit> {
    match value {
        "percentage" => Ok(UsageUnit::Percentage),
        "tokens" => Ok(UsageUnit::Tokens),
        "requests" => Ok(UsageUnit::Requests),
        "credits" => Ok(UsageUnit::Credits),
        "messages" => Ok(UsageUnit::Messages),
        "unknown" => Ok(UsageUnit::Unknown),
        _ => Err(invalid_value(value)),
    }
}

fn period_name(period: UsagePeriod) -> &'static str {
    match period {
        UsagePeriod::Hourly => "hourly",
        UsagePeriod::Rolling => "rolling",
        UsagePeriod::Daily => "daily",
        UsagePeriod::Weekly => "weekly",
        UsagePeriod::Monthly => "monthly",
        UsagePeriod::Unknown => "unknown",
    }
}

fn parse_period(value: &str) -> SqlResult<UsagePeriod> {
    match value {
        "hourly" => Ok(UsagePeriod::Hourly),
        "rolling" => Ok(UsagePeriod::Rolling),
        "daily" => Ok(UsagePeriod::Daily),
        "weekly" => Ok(UsagePeriod::Weekly),
        "monthly" => Ok(UsagePeriod::Monthly),
        "unknown" => Ok(UsagePeriod::Unknown),
        _ => Err(invalid_value(value)),
    }
}

fn source_name(source: UsageSource) -> &'static str {
    match source {
        UsageSource::Mock => "mock",
        UsageSource::OfficialApi => "official_api",
        UsageSource::Cli => "cli",
        UsageSource::LocalState => "local_state",
        UsageSource::InternalEndpoint => "internal_endpoint",
    }
}

fn parse_source(value: &str) -> SqlResult<UsageSource> {
    match value {
        "mock" => Ok(UsageSource::Mock),
        "official_api" => Ok(UsageSource::OfficialApi),
        "cli" => Ok(UsageSource::Cli),
        "local_state" => Ok(UsageSource::LocalState),
        "internal_endpoint" => Ok(UsageSource::InternalEndpoint),
        _ => Err(invalid_value(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage() -> ProviderUsage {
        ProviderUsage {
            provider: ProviderId::Codex,
            installation_id: "codex:linux".into(),
            runtime_id: "linux".into(),
            account_key: Some("opaque-account-fingerprint".into()),
            status: UsageStatus::Ok,
            percentage: Some(25.0),
            used: Some(25.0),
            remaining: Some(75.0),
            limit: Some(100.0),
            unit: Some(UsageUnit::Percentage),
            period: Some(UsagePeriod::Weekly),
            period_description: Some("weekly".into()),
            reset_at: Some("2026-09-22T00:00:00Z".into()),
            weekly: Some(UsageWindow {
                percentage: 10.0,
                period: UsagePeriod::Monthly,
                period_description: "monthly".into(),
                reset_at: None,
            }),
            updated_at: "2026-09-21T00:00:00Z".into(),
            source: UsageSource::Cli,
            error: None,
        }
    }

    #[test]
    fn applies_versioned_schema_once() {
        let storage = Storage::in_memory().unwrap();
        assert_eq!(
            storage.scalar_i64("SELECT MAX(version) FROM schema_migrations"),
            Ok(SCHEMA_VERSION)
        );
        assert_eq!(
            storage.scalar_i64("SELECT COUNT(*) FROM usage_snapshots"),
            Ok(0)
        );
        assert_eq!(
            storage.scalar_i64("SELECT COUNT(*) FROM last_valid_usage"),
            Ok(0)
        );
    }

    #[test]
    fn saves_and_reads_last_valid_usage_without_credentials() {
        let storage = Storage::in_memory().unwrap();
        let input = usage();
        assert_eq!(storage.save_valid_usage(&input).unwrap(), 1);
        let output = storage.last_valid_usage("codex:linux").unwrap().unwrap();

        assert_eq!(output.installation_id, input.installation_id);
        assert_eq!(output.account_key, input.account_key);
        assert_eq!(output.weekly.unwrap().percentage, 10.0);
        assert_eq!(
            storage.scalar_i64("SELECT COUNT(*) FROM usage_snapshots"),
            Ok(1)
        );

        let connection = storage.connection.lock().unwrap();
        let columns: Vec<String> = connection
            .prepare("PRAGMA table_info(usage_snapshots)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.iter().any(|column| {
            column.contains("token") || column.contains("secret") || column.contains("credential")
        }));
    }

    #[test]
    fn newer_valid_snapshot_updates_pointer_and_keeps_history() {
        let storage = Storage::in_memory().unwrap();
        storage.save_valid_usage(&usage()).unwrap();
        let mut second = usage();
        second.percentage = Some(50.0);
        storage.save_valid_usage(&second).unwrap();

        assert_eq!(
            storage.scalar_i64("SELECT COUNT(*) FROM usage_snapshots"),
            Ok(2)
        );
        assert_eq!(
            storage
                .last_valid_usage("codex:linux")
                .unwrap()
                .unwrap()
                .percentage,
            Some(50.0)
        );
    }
}
