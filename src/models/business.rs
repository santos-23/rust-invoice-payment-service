use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::utils::{default_page, default_limit};


#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::businesses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Business {
    pub id: Uuid,
    pub name: String,
    pub api_key_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::businesses)]
pub struct NewBusiness<'a> {
    pub name: &'a str,
    pub api_key_hash: &'a str,
}

/// Request model
#[derive(Deserialize, Clone, utoipa::ToSchema, Debug)]
pub struct ListBusinessQuery {
    pub name: Option<String>,
    
    #[serde(default = "default_page")]
    #[schema(default = 1, example = 1)]
    pub page: i64,

    #[serde(default = "default_limit")]
    #[schema(default = 10, example = 10)]
    pub limit: i64
}