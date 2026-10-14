
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::webhook_endpoints)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub business_id: Uuid,
    pub url: String,
    pub secret_hash: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::webhook_endpoints)]
pub struct NewWebhookEndpoint<'a> {
    pub business_id: Uuid,
    pub url: &'a str,
    pub secret_hash: Option<&'a str>,
}