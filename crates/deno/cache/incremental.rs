use super::cache_db::CacheDBConfiguration;
use super::cache_db::CacheFailure;
pub static INCREMENTAL_CACHE_DB: CacheDBConfiguration = CacheDBConfiguration {
  table_initializer: concat!(
    "CREATE TABLE IF NOT EXISTS incrementalcache (",
    "file_path TEXT PRIMARY KEY,",
    "state_hash INTEGER NOT NULL,",
    "source_hash INTEGER NOT NULL",
    ");"
  ),
  on_version_change: "DELETE FROM incrementalcache;",
  preheat_queries: &[],
  on_failure: CacheFailure::Blackhole,
};
