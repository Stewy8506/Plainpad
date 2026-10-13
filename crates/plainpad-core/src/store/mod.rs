pub mod file;
pub mod schema;

pub use file::{content_hash, FileStore, FileStoreMeta, FileStoreNote};
pub use schema::{get_user_version, run_migrations, set_user_version, CURRENT_SCHEMA_VERSION};

use rusqlite::Connection;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Core storage error variants.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Migration(String),
    NoteLocked,
    NoteInVoid,
    NotFound(String),
    InvalidOperation(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "SQLite error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Migration(e) => write!(f, "Migration error: {e}"),
            Self::NoteLocked => write!(f, "Note is locked and cannot be modified or deleted"),
            Self::NoteInVoid => write!(f, "Note is in The Void and cannot be edited"),
            Self::NotFound(id) => write!(f, "Entity not found: {id}"),
            Self::InvalidOperation(msg) => write!(f, "Invalid store operation: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// SQLite persistent store wrapping a synchronized rusqlite connection.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Opens an on-disk SQLite store, creates parent directories if needed,
    /// configures PRAGMA journal_mode = WAL and PRAGMA foreign_keys = ON,
    /// and runs sequential migrations up to Model 9.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Opens an in-memory SQLite store, enables PRAGMA foreign_keys = ON,
    /// and runs sequential migrations up to Model 9.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Returns a reference to the shared Connection mutex.
    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    /// Runs a closure with an immutable reference to the connection.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, StoreError>
    where
        F: FnOnce(&Connection) -> Result<T, StoreError>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::InvalidOperation(format!("Lock poisoned: {e}")))?;
        f(&conn)
    }

    /// Runs a closure with a mutable reference to the connection.
    pub fn with_conn_mut<F, T>(&self, f: F) -> Result<T, StoreError>
    where
        F: FnOnce(&mut Connection) -> Result<T, StoreError>,
    {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::InvalidOperation(format!("Lock poisoned: {e}")))?;
        f(&mut conn)
    }
}
