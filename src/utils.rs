use serde::Serialize;
use utoipa::ToSchema;

use crate::config::{DEFAULT_LIMIT, DEFAULT_PAGE};

#[derive(Serialize, ToSchema)]
pub struct ApiResponse {
    pub message: String,
}

/// Converts wildcard characters in a string to SQL-compatible wildcards.
pub fn sanitize_wildcard(input: &str) -> String {
    input.trim().replace('*', "%")
}

pub const fn convert_f64_i64(value: f64) -> i64 {
    value as i64
}

pub fn convert_usize_i64(value: usize) -> i64 {
    let Ok(result) = value.try_into() else { return i64::MAX; };
    result
}

pub const fn convert_i64_f64(value: i64) -> f64 {
    if value > f64::MAX as i64 {
        return f64::MAX;
    }
    value as f64
}

pub fn default_page() -> i64 {
    DEFAULT_PAGE
}

pub fn default_limit() -> i64 {
    DEFAULT_LIMIT
}