//! Pre-Restore Safety Snapshot manager with sync isolation.

use crate::model::now_iso;
use crate::store::{SqliteStore, StoreError};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadata for a database snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMeta {
    pub path: PathBuf,
    pub timestamp: String,
    pub reason: String,
    pub size_bytes: u64,
}

impl SqliteStore {
    /// Returns the directory where snapshots are stored.
    pub fn snapshots_dir(&self) -> PathBuf {
        if let Some(ref db_path) = self.path() {
            db_path.parent().unwrap_or(Path::new(".")).join("snapshots")
        } else {
            std::env::temp_dir().join("plainpad_snapshots")
        }
    }

    /// Creates a pre-restore safety snapshot of the active SQLite database.
    ///
    /// Generates `snapshots/snapshot_<timestamp_ms>_<sanitized_reason>.db`
    /// using SQLite's online backup engine to ensure transactional consistency
    /// without WAL log corruption.
    pub fn create_pre_restore_snapshot(&self, reason: &str) -> Result<PathBuf, StoreError> {
        let snapshots_dir = self.snapshots_dir();
        std::fs::create_dir_all(&snapshots_dir)?;

        let sanitized_reason: String = reason
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();

        let timestamp_ms = chrono::Utc::now().timestamp_millis();
        let filename = format!("snapshot_{timestamp_ms}_{sanitized_reason}.db");
        let snapshot_path = snapshots_dir.join(filename);

        self.with_conn(|conn| {
            let mut dst_conn = Connection::open(&snapshot_path)?;
            let backup = rusqlite::backup::Backup::new(conn, &mut dst_conn)?;
            backup.run_to_completion(100, std::time::Duration::from_millis(10), None)?;
            Ok(())
        })?;

        Ok(snapshot_path)
    }

    /// Restores database state from a snapshot file.
    ///
    /// Lifecycle guarantees:
    /// 1. Automatically creates a safety snapshot (`pre_restore_safety`) of the current state
    ///    before performing any overwrite (pendingBackupRestoreAnchorPath pattern).
    /// 2. Restores snapshot pages into active database using the SQLite backup engine.
    /// 3. Enables sync isolation (`sync_isolated = true` in `app_state`) to decouple
    ///    cloud sync during restore evaluation.
    pub fn restore_from_snapshot(&self, snapshot_path: &Path) -> Result<(), StoreError> {
        if !snapshot_path.exists() {
            return Err(StoreError::NotFound(snapshot_path.display().to_string()));
        }

        // 1. Automatic safety backup of current state
        self.create_pre_restore_snapshot("pre_restore_safety")?;

        // 2. Restore pages from snapshot into active connection
        let src_conn = Connection::open(snapshot_path)?;

        self.with_conn_mut(|conn| {
            {
                let backup = rusqlite::backup::Backup::new(&src_conn, conn)?;
                backup.run_to_completion(100, std::time::Duration::from_millis(10), None)?;
            }

            // Re-assert foreign keys and run any necessary migrations
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            crate::store::schema::run_migrations(conn)?;

            // 3. Sync isolation: set sync_isolated = true
            conn.execute(
                "INSERT INTO app_state (key, value) VALUES ('sync_isolated', 'true')
                 ON CONFLICT(key) DO UPDATE SET value = 'true'",
                [],
            )?;

            Ok(())
        })?;

        Ok(())
    }

    /// Lists all snapshots in `<root>/snapshots/`, sorted newest first.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotMeta>, StoreError> {
        let dir = self.snapshots_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("db") {
                let size_bytes = entry.metadata()?.len();
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let (timestamp, reason) = if let Some(rest) = file_name.strip_prefix("snapshot_") {
                    if let Some((ts_str, r)) = rest.split_once('_') {
                        if let Ok(ts_ms) = ts_str.parse::<i64>() {
                            let dt = chrono::DateTime::from_timestamp_millis(ts_ms)
                                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
                                .unwrap_or_else(now_iso);
                            (dt, r.to_string())
                        } else {
                            (now_iso(), r.to_string())
                        }
                    } else {
                        (now_iso(), rest.to_string())
                    }
                } else {
                    (now_iso(), "snapshot".to_string())
                };

                snapshots.push(SnapshotMeta {
                    path,
                    timestamp,
                    reason,
                    size_bytes,
                });
            }
        }

        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(snapshots)
    }

    /// Returns whether sync isolation is currently enabled (`sync_isolated == true`).
    pub fn is_sync_isolated(&self) -> bool {
        self.get_setting("sync_isolated")
            .ok()
            .flatten()
            .and_then(|v| {
                if let Some(b) = v.as_bool() {
                    Some(b)
                } else if let Some(s) = v.as_str() {
                    Some(s == "true")
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
}
