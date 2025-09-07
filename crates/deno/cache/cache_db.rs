use deno_core::error::AnyError;
use deno_core::parking_lot::Mutex;
use deno_core::parking_lot::MutexGuard;
use deno_core::unsync::spawn_blocking;
use deno_lib::util::hash::FastInsecureHasher;
use deno_runtime::deno_webstorage::rusqlite;
use deno_runtime::deno_webstorage::rusqlite::Connection;
use deno_runtime::deno_webstorage::rusqlite::OptionalExtension;
use deno_runtime::deno_webstorage::rusqlite::Params;
use once_cell::sync::OnceCell;
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CacheDBHash(u64);
impl CacheDBHash {
  pub fn new(hash: u64) -> Self {
    Self(hash)
  }
  pub fn from_hashable(hashable: impl std::hash::Hash) -> Self {
    Self::new(
      FastInsecureHasher::new_deno_versioned()
        .write_hashable(hashable)
        .finish(),
    )
  }
  pub fn inner(&self) -> u64 {
    self.0
  }
}
impl rusqlite::types::ToSql for CacheDBHash {
  fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
    Ok(rusqlite::types::ToSqlOutput::Owned(
      rusqlite::types::Value::Integer(self.0 as i64),
    ))
  }
}
impl rusqlite::types::FromSql for CacheDBHash {
  fn column_result(
    value: rusqlite::types::ValueRef,
  ) -> rusqlite::types::FromSqlResult<Self> {
    match value {
      rusqlite::types::ValueRef::Integer(i) => Ok(Self::new(i as u64)),
      _ => Err(rusqlite::types::FromSqlError::InvalidType),
    }
  }
}
/// What should the cache should do on failure?
#[derive(Debug, Default)]
pub enum CacheFailure {
  /// Return errors if failure mode otherwise unspecified.
  #[default]
  Error,
  /// Create an in-memory cache that is not persistent.
  InMemory,
  /// Create a blackhole cache that ignores writes and returns empty reads.
  Blackhole,
}
/// Configuration SQL and other parameters for a [`CacheDB`].
#[derive(Debug)]
pub struct CacheDBConfiguration {
  /// SQL to run for a new database.
  pub table_initializer: &'static str,
  /// SQL to run when the version from [`crate::version::deno()`] changes.
  pub on_version_change: &'static str,
  /// Prepared statements to pre-heat while initializing the database.
  pub preheat_queries: &'static [&'static str],
  /// What the cache should do on failure.
  pub on_failure: CacheFailure,
}
impl CacheDBConfiguration {
  fn create_combined_sql(&self) -> String {
    format!(
      concat!(
        "PRAGMA journal_mode=WAL;",
        "PRAGMA synchronous=NORMAL;",
        "PRAGMA temp_store=memory;",
        "PRAGMA page_size=4096;",
        "PRAGMA mmap_size=6000000;",
        "PRAGMA optimize;",
        "CREATE TABLE IF NOT EXISTS info (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        "{}",
      ),
      self.table_initializer
    )
  }
}
#[derive(Debug)]
enum ConnectionState {
  Connected(Connection),
  Blackhole,
  Error(Arc<AnyError>),
}
/// A cache database that eagerly initializes itself off-thread, preventing initialization operations
/// from blocking the main thread.
#[derive(Debug, Clone)]
pub struct CacheDB {
  conn: Arc<Mutex<OnceCell<ConnectionState>>>,
  path: Option<PathBuf>,
  config: &'static CacheDBConfiguration,
  version: &'static str,
}
impl Drop for CacheDB {
  fn drop(&mut self) {
    let path = match self.path.take() {
      Some(path) => path,
      _ => return,
    };
    if tokio::runtime::Handle::try_current().is_err() {
      return;
    }
    let arc = std::mem::take(&mut self.conn);
    if let Ok(inner) = Arc::try_unwrap(arc) {
      let inner = inner.into_inner().into_inner();
      if let Some(conn) = inner {
        spawn_blocking(move || {
          drop(conn);
          log::trace!(
            "Cleaned up SQLite connection at {}",
            path.to_string_lossy()
          );
        });
      }
    }
  }
}
impl CacheDB {
  pub fn in_memory(
    config: &'static CacheDBConfiguration,
    version: &'static str,
  ) -> Self {
    CacheDB {
      conn: Arc::new(Mutex::new(OnceCell::new())),
      path: None,
      config,
      version,
    }
  }
  pub fn from_path(
    config: &'static CacheDBConfiguration,
    path: PathBuf,
    version: &'static str,
  ) -> Self {
    log::debug!("Opening cache {}...", path.to_string_lossy());
    let new = Self {
      conn: Arc::new(Mutex::new(OnceCell::new())),
      path: Some(path),
      config,
      version,
    };
    new.spawn_eager_init_thread();
    new
  }
  fn spawn_eager_init_thread(&self) {
    let clone = self.clone();
    debug_assert!(tokio::runtime::Handle::try_current().is_ok());
    spawn_blocking(move || {
      let lock = clone.conn.lock();
      clone.initialize(&lock);
    });
  }
  /// Open the connection in memory or on disk.
  fn actually_open_connection(
    &self,
    path: Option<&Path>,
  ) -> Result<Connection, rusqlite::Error> {
    match path {
      None => Connection::open_in_memory(),
      Some(path) => Connection::open(path),
    }
  }
  /// Attempt to initialize that connection.
  fn initialize_connection(
    config: &CacheDBConfiguration,
    conn: &Connection,
    version: &str,
  ) -> Result<(), rusqlite::Error> {
    let sql = config.create_combined_sql();
    conn.execute_batch(&sql)?;
    let existing_version = conn
      .query_row(
        "SELECT value FROM info WHERE key='CLI_VERSION' LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
      )
      .optional()?
      .unwrap_or_default();
    if existing_version != version {
      conn.execute_batch(config.on_version_change)?;
      let mut stmt = conn
        .prepare("INSERT OR REPLACE INTO info (key, value) VALUES (?1, ?2)")?;
      stmt.execute(["CLI_VERSION", version])?;
    }
    for preheat in config.preheat_queries {
      drop(conn.prepare_cached(preheat)?);
    }
    Ok(())
  }
  /// Open and initialize a connection.
  fn open_connection_and_init(
    &self,
    path: Option<&Path>,
  ) -> Result<Connection, rusqlite::Error> {
    let conn = self.actually_open_connection(path)?;
    Self::initialize_connection(self.config, &conn, self.version)?;
    Ok(conn)
  }
  /// This function represents the policy for dealing with corrupted cache files. We try fairly aggressively
  /// to repair the situation, and if we can't, we prefer to log noisily and continue with in-memory caches.
  fn open_connection(&self) -> Result<ConnectionState, AnyError> {
    open_connection(self.config, self.path.as_deref(), |maybe_path| {
      self.open_connection_and_init(maybe_path)
    })
  }
  fn initialize<'a>(
    &self,
    lock: &'a MutexGuard<OnceCell<ConnectionState>>,
  ) -> &'a ConnectionState {
    lock.get_or_init(|| match self.open_connection() {
      Ok(conn) => conn,
      Err(e) => ConnectionState::Error(e.into()),
    })
  }
  pub fn with_connection<T: Default>(
    &self,
    f: impl FnOnce(&Connection) -> Result<T, AnyError>,
  ) -> Result<T, AnyError> {
    let lock = self.conn.lock();
    let conn = self.initialize(&lock);
    match conn {
      ConnectionState::Blackhole => Ok(T::default()),
      ConnectionState::Error(e) => {
        let err = AnyError::msg(e.clone().to_string());
        Err(err)
      }
      ConnectionState::Connected(conn) => f(conn),
    }
  }
  pub fn execute(
    &self,
    sql: &'static str,
    params: impl Params,
  ) -> Result<usize, AnyError> {
    self.with_connection(|conn| {
      let mut stmt = conn.prepare_cached(sql)?;
      let res = stmt.execute(params)?;
      Ok(res)
    })
  }
  pub fn exists(
    &self,
    sql: &'static str,
    params: impl Params,
  ) -> Result<bool, AnyError> {
    self.with_connection(|conn| {
      let mut stmt = conn.prepare_cached(sql)?;
      let res = stmt.exists(params)?;
      Ok(res)
    })
  }
  /// Query a row from the database with a mapping function.
  pub fn query_row<T, F>(
    &self,
    sql: &'static str,
    params: impl Params,
    f: F,
  ) -> Result<Option<T>, AnyError>
  where
    F: FnOnce(&rusqlite::Row<'_>) -> Result<T, AnyError>,
  {
    let res = self.with_connection(|conn| {
      let mut stmt = conn.prepare_cached(sql)?;
      let mut rows = stmt.query(params)?;
      if let Some(row) = rows.next()? {
        let res = f(row)?;
        Ok(Some(res))
      } else {
        Ok(None)
      }
    })?;
    Ok(res)
  }
}
/// This function represents the policy for dealing with corrupted cache files. We try fairly aggressively
/// to repair the situation, and if we can't, we prefer to log noisily and continue with in-memory caches.
fn open_connection(
  config: &CacheDBConfiguration,
  path: Option<&Path>,
  open_connection_and_init: impl Fn(
    Option<&Path>,
  ) -> Result<Connection, rusqlite::Error>,
) -> Result<ConnectionState, AnyError> {
  let err = match open_connection_and_init(path) {
    Ok(conn) => return Ok(ConnectionState::Connected(conn)),
    Err(err) => err,
  };
  let Some(path) = path.as_ref() else {
    log::error!("Failed to initialize in-memory cache database.");
    return Err(err.into());
  };
  if let rusqlite::Error::SqliteFailure(ffi_err, _) = &err
    && ffi_err.code == rusqlite::ErrorCode::ReadOnly
  {
    log::debug!(
      "Failed creating cache db. Folder readonly: {}",
      path.display()
    );
    return handle_failure_mode(config, err, open_connection_and_init);
  }
  if let Some(parent) = path.parent() {
    match std::fs::create_dir_all(parent) {
      Ok(_) => {
        log::debug!("Created parent directory for cache db.");
      }
      Err(err) => {
        log::debug!("Failed creating the cache db parent dir: {:#}", err);
      }
    }
  }
  log::trace!(
    "Could not initialize cache database '{}', retrying... ({err:?})",
    path.to_string_lossy(),
  );
  let err = match open_connection_and_init(Some(path)) {
    Ok(conn) => return Ok(ConnectionState::Connected(conn)),
    Err(err) => err,
  };
  let is_tty = std::io::stderr().is_terminal();
  log::log!(
    if is_tty {
      log::Level::Warn
    } else {
      log::Level::Trace
    },
    "Could not initialize cache database '{}', deleting and retrying... ({err:?})",
    path.to_string_lossy()
  );
  if std::fs::remove_file(path).is_ok() {
    let res = open_connection_and_init(Some(path));
    if let Ok(conn) = res {
      return Ok(ConnectionState::Connected(conn));
    }
  }
  log_failure_mode(path, is_tty, config);
  handle_failure_mode(config, err, open_connection_and_init)
}
fn log_failure_mode(path: &Path, is_tty: bool, config: &CacheDBConfiguration) {
  match config.on_failure {
    CacheFailure::InMemory => {
      log::log!(
        if is_tty {
          log::Level::Error
        } else {
          log::Level::Trace
        },
        "Failed to open cache file '{}', opening in-memory cache.",
        path.display()
      );
    }
    CacheFailure::Blackhole => {
      log::log!(
        if is_tty {
          log::Level::Error
        } else {
          log::Level::Trace
        },
        "Failed to open cache file '{}', performance may be degraded.",
        path.display()
      );
    }
    CacheFailure::Error => {
      log::error!(
        "Failed to open cache file '{}', expect further errors.",
        path.display()
      );
    }
  }
}
fn handle_failure_mode(
  config: &CacheDBConfiguration,
  err: rusqlite::Error,
  open_connection_and_init: impl Fn(
    Option<&Path>,
  ) -> Result<Connection, rusqlite::Error>,
) -> Result<ConnectionState, AnyError> {
  match config.on_failure {
    CacheFailure::InMemory => {
      Ok(ConnectionState::Connected(open_connection_and_init(None)?))
    }
    CacheFailure::Blackhole => Ok(ConnectionState::Blackhole),
    CacheFailure::Error => Err(err.into()),
  }
}
