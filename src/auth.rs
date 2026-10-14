use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use diesel::prelude::*;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    db::Pool, schema::businesses, services::business_services::get_business_data_by_key_hash,
};

#[derive(Clone, Debug)]
pub struct AuthenticatedBusiness {
    pub id: Uuid,
}

pub fn hash_api_key(api_key: &str) -> String {
    hex::encode(Sha256::digest(api_key.as_bytes()))
}

pub async fn require_api_key(
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    let api_key = match request
        .headers()
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
    {
        Some(key) if !key.trim().is_empty() => key,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing API key"
                })),
            )
                .into_response();
        }
    };

    let key_hash = hash_api_key(api_key);
    let business = match get_business_data_by_key_hash(key_hash).await {
        Ok(Some(business)) => business,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid API key"
                })),
            )
                .into_response();
        }
        Err(error) => {
            eprintln!("Database authentication error: {error}");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Authentication service unavailable"
                })),
            )
                .into_response();
        }
    };

    request
        .extensions_mut()
        .insert(AuthenticatedBusiness { id: business.id });

    next.run(request).await
}