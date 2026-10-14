use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::utils::{default_page, default_limit};

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Customer {
    pub id: Uuid,
    pub business_id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::customers)]
pub struct NewCustomer<'a> {
    pub business_id: Uuid,
    pub name: &'a str,
    pub email: &'a str,
}

/// Request model
#[derive(Deserialize, Serialize, Clone, utoipa::ToSchema, Debug)]
pub struct ListCustomerQuery {
    pub name: Option<String>,
    pub email: Option<String>,

    #[schema(ignore)]
    #[serde(default)]
    pub business_id: Option<String>,

    #[serde(default = "default_page")]
    #[schema(default = 1, example = 1)]
    pub page: i64,

    #[serde(default = "default_limit")]
    #[schema(default = 10, example = 10)]
    pub limit: i64
}