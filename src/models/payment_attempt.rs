use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::schema::payment_attempts;

#[derive(Debug, Queryable, Selectable, serde::Serialize, ToSchema)]
#[diesel(table_name = payment_attempts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PaymentAttempt {
    pub id: Uuid,
    pub business_id: Uuid,
    pub invoice_id: Uuid,
    pub status: String,
    pub amount_cents: i64,
    pub currency: String,
    pub psp_reference: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = payment_attempts)]
pub struct NewPaymentAttempt {
    pub business_id: Uuid,
    pub invoice_id: Uuid,
    pub status: String,
    pub amount_cents: i64,
    pub currency: String,
}