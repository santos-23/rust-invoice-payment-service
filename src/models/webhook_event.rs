use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::webhook_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WebhookEvent {
    pub id: Uuid,
    pub business_id: Uuid,
    pub endpoint_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub event_type: String,
    pub payload: Value,
    pub status: String,
    pub attempt_count: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::webhook_events)]
pub struct NewWebhookEvent {
    pub business_id: Uuid,
    pub endpoint_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub event_type: String,
    pub payload: Value,
}