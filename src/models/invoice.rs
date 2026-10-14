use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::utils::{default_page, default_limit};

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::invoices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Invoice {
    pub id: Uuid,
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub due_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::invoices)]
pub struct NewInvoice {
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub due_date: NaiveDate,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct ListInvoiceQuery {
    #[serde(default = "default_page")]
    #[schema(default = 1, example = 1)]
    pub page: i64,

    #[serde(default = "default_limit")]
    #[schema(default = 10, example = 10)]
    pub limit: i64,

    pub customer_id: Option<String>,
    pub status: Option<String>,
}