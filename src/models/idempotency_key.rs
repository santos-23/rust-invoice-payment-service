use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::idempotency_keys)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IdempotencyKey {
    pub id: Uuid,
    pub business_id: Uuid,
    pub operation: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub response_status: Option<i32>,
    pub response_body: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::idempotency_keys)]
pub struct NewIdempotencyKey<'a> {
    pub business_id: Uuid,
    pub operation: &'a str,
    pub idempotency_key: &'a str,
    pub request_hash: &'a str,
}