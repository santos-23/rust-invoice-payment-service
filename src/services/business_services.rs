use axum::{Json, extract::{Query}, http::StatusCode, response::{IntoResponse, Response}};
use diesel::{QueryDsl, pg::Pg};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use diesel::prelude::*;

use crate::{auth::hash_api_key, config::{DEFAULT_DATE_FORMAT}, db::get_conn, models::business::{Business, ListBusinessQuery, NewBusiness}, schema::businesses, utils::{convert_f64_i64, convert_i64_f64, convert_usize_i64, sanitize_wildcard}};

/// Create business request
#[derive(Deserialize, ToSchema)]
pub struct CreateBusinessRequest {
    pub name: String,
}

/// Create business data
#[derive(Serialize, ToSchema)]
pub struct CreateBusinessData {
    pub id: String,
    pub name: String,
    pub api_key: String,
}

/// Create business response model
#[derive(Serialize, ToSchema)]
pub struct CreateBusinessResponse {
    is_success: bool,
    message: String,
    result: CreateBusinessData,
}

/// Success model
#[derive(Serialize, ToSchema)]
pub struct ListBusinessSuccessResponse {
    is_success: bool,
    message: String,
    page: i64,
    limit: i64,
    record_count: i64,
    total_pages: i64,
    previous: bool,
    next: bool,
    total_records: i64,
    result: Option<Vec<Value>>
}

/// Error model
#[derive(Serialize, ToSchema)]
pub struct ListBusinessErrorResponse {
    is_success: bool,
    message: String,
    result: Option<CreateBusinessData>
}

/// Data model
#[derive(Debug, Serialize)]
pub struct ListBusinessData {
    pub id: Uuid,
    pub name: String,
    pub api_key_hash: String,
    pub created_at: String,
}

const fn build_error_response(is_success: bool, message: String, result: Option<CreateBusinessData>) -> ListBusinessErrorResponse {
    ListBusinessErrorResponse {
        is_success,
        message,
        result
    }
}

#[utoipa::path(
    post,
    tag = "business",
    path = "/api/business",
    description = "Used to add new business data", 
    summary = "Add business data",
    request_body(
        content = CreateBusinessRequest,
        example = json!({
            "name": "",
        })
    ),
    responses(
        (
            status = 200,
            description = "Success",
            body = CreateBusinessResponse,
            example = json!({
                "is_success": true,
                "message": "New business added successfully",
                "result": []
            })
        ),
        (
            status = 400,
            description = "Bad request",
            body = CreateBusinessResponse,
            example = json!({
                "is_success": false,
                "message": "Failed to add new business",
                "result": null
            })
        )
    ),
)]
pub async fn create_business(
    Json(payload): Json<CreateBusinessRequest>,
) -> Response {
    let name = payload.name.trim().to_owned();

    if name.is_empty() {
        let response = build_error_response(false, "Name is required to create business".to_string(), None);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    // Generate a high-entropy key and return it only once.
    let api_key = format!(
        "biz_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );

    let api_key_hash = hash_api_key(&api_key);
    let new_business = NewBusiness {
        name: &name,
        api_key_hash: &api_key_hash,
    };

    match add_business(&new_business).await {
        Ok(id) => {
            let response = build_error_response(
                true, 
                "New business added successfully".to_string(), 
                Some(CreateBusinessData {
                    id: id.to_string(),
                    name,
                    api_key
                })
            );

            (StatusCode::CREATED, Json(response)).into_response()
        }

        Err(e) => {
            eprintln!("Failed to add business: {e:?}");
            let response = build_error_response(false, "Failed to add business data".to_string(), None);
            return (StatusCode::BAD_REQUEST, Json(response)).into_response();
        }
    }
}

#[utoipa::path(
    get,
    tag = "business",
    path = "/api/business",
    description = "Returns all the businesses", 
    summary = "View all the businesses",
    params(
        ("page" = Option<i64>, Query, description = "Filter by page", example=1),
        ("limit" = Option<i64>, Query, description = "Filter by limit", example=10),
        ("name" = Option<String>, Query, description = "Filter by name"),
    ),
    responses(
        (
            status = 200,
            description = "Success",
            body = ListBusinessSuccessResponse,
            example = json!({
                "is_success": true,
                "message": "Business data fetched successfully",
                "page": 1,
                "limit": 10,
                "record_count": 10,
                "total_pages": 1,
                "previous": false,
                "next": false,
                "total_records": 10,
                "result": []
            })
        ),
        (
            status = 400,
            description = "Bad request",
            body = ListBusinessErrorResponse,
            example = json!({
                "is_success": false,
                "message": "Failed to fetch business data",
                "result": null
            })
        )
    )
)]
pub async fn get_all_businesses(
    // request: Request<Body>,
    Query(business_query): Query<ListBusinessQuery>,
) -> Response {
    match fetch_business_records_count(business_query.clone()).await {
        Ok(total_records) => {
            if total_records <= 0 {
                let response = build_error_response(true, "No data available".to_string(), None);
                return (StatusCode::OK, Json(response)).into_response();
            }
            let page = business_query.clone().page;
            let limit = business_query.clone().limit;
            let total_pages = convert_f64_i64((convert_i64_f64(total_records) / convert_i64_f64(limit)).ceil());
            match fetch_business_records(business_query.clone()).await {
                Ok(data) => {
                    let mut response_data: Vec<ListBusinessData> = Vec::new();
                    for record in data {
                        let record = ListBusinessData {
                            id: record.id,
                            name: record.name,
                            api_key_hash: record.api_key_hash,
                            created_at: record.created_at.format(DEFAULT_DATE_FORMAT).to_string(),
                        };
                        response_data.push(record);
                    }
                    let json_values_result: Result<Vec<Value>, serde_json::Error> = response_data
                        .into_iter().map(serde_json::to_value).collect();
                    match json_values_result {
                        Ok(json_values) => {
                           let response = ListBusinessSuccessResponse {
                                is_success: true,
                                message: "Business data fetched successfully".to_string(),
                                page,
                                limit,
                                record_count: convert_usize_i64(json_values.len()),
                                total_pages,
                                previous: page > 1,
                                next: page < total_pages,
                                total_records,
                                result: if json_values.is_empty() { None } else { Some(json_values) }
                            };
                            (StatusCode::OK, Json(response)).into_response()
                        },
                        Err(_e) => {
                            let response = build_error_response(false, "Failed to serialize business data".to_string(), None);
                            (StatusCode::BAD_REQUEST, Json(response)).into_response()
                        }
                    }
                },
                Err(_e) => {
                    let response = build_error_response(false, "Failed to fetch business data".to_string(), None);
                    (StatusCode::BAD_REQUEST, Json(response)).into_response()
                }
            }
        },
        Err(_e) => {
            let response = build_error_response(false, "Failed to fetch business data".to_string(), None);
            (StatusCode::BAD_REQUEST, Json(response)).into_response()
        }
    }
}

///---------------------------
/// Data functions
///---------------------------
pub async fn add_business(business_data: &NewBusiness<'_>) -> Result<Uuid, anyhow::Error> {
    use crate::schema::businesses::dsl::businesses;

    let mut conn = get_conn().await?;
    let id = diesel::insert_into(businesses)
        .values(business_data)
        .returning(crate::schema::businesses::id)
        .get_result::<Uuid>(&mut conn)
        .map_err(anyhow::Error::new)?;

    Ok(id)
}

pub async fn fetch_business_records(query_info: ListBusinessQuery) -> Result<Vec<Business>, anyhow::Error> {
    let mut conn = get_conn().await?;
    let mut query_builder = businesses::table.into_boxed();
    query_builder = apply_filters(query_builder, query_info.clone());
    query_builder = apply_pagination(query_builder, &query_info);
    query_builder = query_builder.order_by(businesses::created_at.desc());
    let result = query_builder.load::<Business>(&mut conn)?;

    Ok(result)
}

pub async fn fetch_business_records_count(query_info: ListBusinessQuery) -> Result<i64, anyhow::Error> {
    let mut conn = get_conn().await?;
    let mut query_builder = businesses::table.into_boxed();
    query_builder = apply_filters(query_builder, query_info.clone());
    let count = query_builder.count().get_result(&mut conn)?;
    Ok(count)
}

pub async fn get_business_data_by_id(id: Uuid) -> Result<Vec<Business>, anyhow::Error> {
    let mut conn = get_conn().await?;
    let result = businesses::table
        .select(Business::as_select())
        .filter(businesses::id.eq(id))
        .load::<Business>(&mut conn)?;

    Ok(result)
}

pub async fn get_business_data_by_key_hash(key_hash: String) -> Result<Option<Business>, anyhow::Error> {
    let mut conn = get_conn().await?;
    let result = businesses::table
        .select(Business::as_select())
        .filter(businesses::api_key_hash.eq(key_hash))
        .first::<Business>(&mut conn)
        .optional()?;

    Ok(result)
}

///---------------------------
/// Helper functions
///---------------------------
fn apply_pagination<'a>(
    query_builder: businesses::BoxedQuery<'a, Pg>,
    query_info: &'a ListBusinessQuery
) -> businesses::BoxedQuery<'a, Pg> {
    let page = query_info.page;
    let limit = query_info.limit;
    let offset = (page - 1) * limit;
    query_builder.limit(limit).offset(offset)
}

fn apply_filters(
    mut query_builder: businesses::BoxedQuery<'_, Pg>,
    query_info: ListBusinessQuery
) -> businesses::BoxedQuery<'_, Pg> {
    if let Some(name) = query_info.name {
        query_builder = query_builder.filter(businesses::name.ilike(sanitize_wildcard(&name)));
    }
    query_builder
}