use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::invoice::Invoice;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::invoice_line_items)]
#[diesel(belongs_to(Invoice, foreign_key = invoice_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InvoiceLineItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub description: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::invoice_line_items)]
pub struct NewInvoiceLineItem<'a> {
    pub invoice_id: Uuid,
    pub description: &'a str,
    pub quantity: i32,
    pub unit_price_cents: i64,
}