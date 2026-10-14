use std::{env, sync::OnceLock};
use dotenvy::dotenv;


/// ────────────────────────────────
/// Constant Values
/// ────────────────────────────────
pub const DEFAULT_REST_SERVER_HOST: &str = "0.0.0.0";
pub const DEFAULT_REST_SERVER_PORT: u16 = 3000;
pub const DEFAULT_PAGE: i64 = 1;
pub const DEFAULT_LIMIT: i64 = 10;
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";


// Environment variable names
pub const DATABASE_URL: &str = "DATABASE_URL";
pub const REST_SERVER_HOST: &str = "REST_SERVER_HOST";
pub const REST_SERVER_PORT: &str = "REST_SERVER_PORT";

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct Environment {
    // Required
    pub database_url: String,

    // Optional
    pub rest_server_host: Option<String>,
    pub rest_server_port: Option<u16>,
}

impl Environment {
    /// Load environment variables and validate required values.
    pub fn load() -> Result<Self, String> {
        // Load .env if available; production environment variables still work.
        let _ = dotenv();

        Ok(Self {
            database_url: required(DATABASE_URL)?,
            rest_server_host: optional(REST_SERVER_HOST),
            rest_server_port: optional(REST_SERVER_PORT)
                .map(|s| s.parse::<u16>())
                .transpose()
                .map_err(|_| format!("{REST_SERVER_PORT} must be a valid port"))?,
        })
    }
}

/// Get a required variable or return an error.
fn required(key: &str) -> Result<String, String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Required environment variable {key} is missing"))
}

/// Get an optional variable. Missing values return None.
fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Initialize configuration once, during application startup.
pub fn init() -> Result<&'static Environment, String> {
    if let Some(config) = ENVIRONMENT.get() {
        return Ok(config);
    }

    let config = Environment::load()?;

    // Handles concurrent initialization safely.
    let _ = ENVIRONMENT.set(config);

    Ok(ENVIRONMENT
        .get()
        .expect("Environment initialization failed"))
}

/// Access globally initialized configuration.
pub fn get() -> &'static Environment {
    ENVIRONMENT.get().expect(
        "Environment not initialized. Call environment::init() at startup.",
    )
}