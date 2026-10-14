
use std::sync::Arc;
use std::time::Duration;
use std::sync::LazyLock;
use anyhow::Result;
use anyhow::anyhow;
use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};

use crate::config;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

static POOL: LazyLock<Arc<Pool>> = LazyLock::new(|| {
    let pool = initialize_connection_pool().expect("Failed to initialize connection pool");
    Arc::new(pool)
});

/// Returns a clone of the connection pool for use in blocking tasks
pub fn get_pool() -> Arc<Pool> {
    POOL.clone()
}

/// Creates a connection pool to the database
pub fn create_pool(database_url: &str) -> Result<Pool> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Ok(
        Pool::builder()
            .max_size(20)
            .connection_timeout(Duration::from_secs(3))
            .min_idle(Some(2))
            .build(manager)?
    )
}

/// Creates a connection pool to the database using the `DATABASE_URL` environment variable.
pub fn initialize_connection_pool() -> Result<Pool> {
    let config = config::init()
        .map_err(std::io::Error::other)?;

    create_pool(&config.database_url)
}

/// Fetches a connection from the connection pool.
pub async fn get_conn() -> Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>, anyhow::Error> {
    let pool = POOL.clone();
    tokio::task::spawn_blocking(move || {
        pool.get().map_err(|e| anyhow!("Failed to get connection: {}", e))
    })
    .await
    .map_err(|e| anyhow!("Join error: {}", e))?
}
