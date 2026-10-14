use axum::{
    Extension, Json, extract::{Query}, http::StatusCode, response::{IntoResponse, Response},
};
use diesel::{pg::Pg, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedBusiness, config::DEFAULT_DATE_FORMAT, db::get_conn, models::customer::{Customer, ListCustomerQuery, NewCustomer}, schema::customers, utils::{convert_f64_i64, convert_i64_f64, convert_usize_i64, sanitize_wildcard},
};

/// Create business request
#[derive(Deserialize, ToSchema)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub email: String,
}

/// Create customer data
#[derive(Serialize, ToSchema)]
pub struct CreateCustomerData {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub email: String,
}

/// Create customer response model
#[derive(Serialize, ToSchema)]
pub struct CreateCustomerResponse {
    is_success: bool,
    message: String,
    result: CreateCustomerData,
}

/// Success model
#[derive(Serialize, ToSchema)]
pub struct ListCustomerSuccessResponse {
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
pub struct ListCustomerErrorResponse {
    is_success: bool,
    message: String,
    result: Option<CreateCustomerData>
}

/// Data model
#[derive(Debug, Serialize)]
pub struct ListCustomerData {
    pub id: Uuid,
    pub business_id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

const fn build_error_response(is_success: bool, message: String, result: Option<CreateCustomerData>) -> ListCustomerErrorResponse {
    ListCustomerErrorResponse {
        is_success,
        message,
        result
    }
}

#[utoipa::path(
    post,
    tag = "customers",
    path = "/api/customer",
    description = "Used to add new customer data to business", 
    summary = "Add customer data",
    request_body(
        content = CreateCustomerRequest,
        example = json!({
            "name": "",
            "email": "",
        })
    ),
    responses(
        (
            status = 200,
            description = "Success",
            body = CreateCustomerResponse,
            example = json!({
                "is_success": true,
                "message": "New customer successfully added to the business",
                "result": []
            })
        ),
        (
            status = 400,
            description = "Bad request",
            body = CreateCustomerResponse,
            example = json!({
                "is_success": false,
                "message": "Failed to add new customer",
                "result": null
            })
        )
    ),
    security(
        ("ApiKeyAuth" = [])
    )
)]
pub async fn create_customer(
    Extension(business): Extension<AuthenticatedBusiness>,
    Json(payload): Json<CreateCustomerRequest>,
) -> Response {
    let name = payload.name.trim().to_owned();
    let email = payload.email.trim().to_owned();

    if name.is_empty() {
        let response = build_error_response(false, "Name is required to create customer".to_string(), None);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    } else if email.is_empty() {
        let response = build_error_response(false, "Email is required to create customer".to_string(), None);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    } else if !email.contains('@') {
        let response = build_error_response(false, "Invalid email format".to_string(), None);
        return (StatusCode::BAD_REQUEST, Json(response)).into_response();
    }

    let business_id = business.id;
    let new_customer = NewCustomer {
        business_id,
        name: &name,
        email: &email,
    };

    match add_customer(&new_customer).await {
        Ok(id) => {
            let response = build_error_response(
                true, 
                "New business added successfully".to_string(), 
                Some(CreateCustomerData {
                    id: id.to_string(),
                    business_id: business_id.to_string(),
                    name,
                    email,
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
    tag = "customers",
    path = "/api/customer",
    description = "Returns all the customers", 
    summary = "View all the businesses",
    params(
        ("page" = Option<i64>, Query, description = "Filter by page", example=1),
        ("limit" = Option<i64>, Query, description = "Filter by limit", example=10),
        ("name" = Option<String>, Query, description = "Filter by name"),
        ("email" = Option<String>, Query, description = "Filter by email"),
    ),
    responses(
        (
            status = 200,
            description = "Success",
            body = ListCustomerSuccessResponse,
            example = json!({
                "is_success": true,
                "message": "Customer data fetched successfully",
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
            body = ListCustomerErrorResponse,
            example = json!({
                "is_success": false,
                "message": "Failed to fetch customer data",
                "result": null
            })
        )
    ),
    security(
        ("ApiKeyAuth" = [])
    )
)]
pub async fn get_all_customers(
    Extension(business): Extension<AuthenticatedBusiness>,
    Query(customer_query): Query<ListCustomerQuery>,
) -> Response {
    let mut customer_query = customer_query;
    customer_query.business_id = Some(business.id.to_string());
    println!("Query: {:?}", customer_query);

    match fetch_customer_records_count(customer_query.clone()).await {
        Ok(total_records) => {
            if total_records <= 0 {
                let response = build_error_response(true, "No data available".to_string(), None);
                return (StatusCode::OK, Json(response)).into_response();
            }
            let page = customer_query.clone().page;
            let limit = customer_query.clone().limit;
            let total_pages = convert_f64_i64((convert_i64_f64(total_records) / convert_i64_f64(limit)).ceil());
            match fetch_customer_records(customer_query.clone()).await {
                Ok(data) => {
                    let mut response_data: Vec<ListCustomerData> = Vec::new();
                    for record in data {
                        let record = ListCustomerData {
                            id: record.id,
                            business_id: record.business_id,
                            name: record.name,
                            email: record.email,
                            created_at: record.created_at.format(DEFAULT_DATE_FORMAT).to_string(),
                        };
                        response_data.push(record);
                    }
                    let json_values_result: Result<Vec<Value>, serde_json::Error> = response_data
                        .into_iter().map(serde_json::to_value).collect();
                    match json_values_result {
                        Ok(json_values) => {
                           let response = ListCustomerSuccessResponse {
                                is_success: true,
                                message: "Customer data fetched successfully".to_string(),
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
                            let response = build_error_response(false, "Failed to serialize customer data".to_string(), None);
                            (StatusCode::BAD_REQUEST, Json(response)).into_response()
                        }
                    }
                },
                Err(_e) => {
                    let response = build_error_response(false, "Failed to fetch customer data".to_string(), None);
                    (StatusCode::BAD_REQUEST, Json(response)).into_response()
                }
            }
        },
        Err(_e) => {
            let response = build_error_response(false, "Failed to fetch customer data".to_string(), None);
            (StatusCode::BAD_REQUEST, Json(response)).into_response()
        }
    }
}


///---------------------------
/// Data functions
///---------------------------
pub async fn add_customer(customer_data: &NewCustomer<'_>) -> Result<Uuid, anyhow::Error> {
    use crate::schema::customers::dsl::customers;

    let mut conn = get_conn().await?;
    let id = diesel::insert_into(customers)
        .values(customer_data)
        .returning(crate::schema::customers::id)
        .get_result::<Uuid>(&mut conn)
        .map_err(anyhow::Error::new)?;

    Ok(id)
}

pub async fn fetch_customer_records(query_info: ListCustomerQuery) -> Result<Vec<Customer>, anyhow::Error> {
    let mut conn = get_conn().await?;
    let mut query_builder = customers::table.into_boxed();
    query_builder = apply_filters(query_builder, query_info.clone());
    query_builder = apply_pagination(query_builder, &query_info);
    query_builder = query_builder.order_by(customers::created_at.desc());
    let result = query_builder.load::<Customer>(&mut conn)?;

    Ok(result)
}

pub async fn fetch_customer_records_count(query_info: ListCustomerQuery) -> Result<i64, anyhow::Error> {
    let mut conn = get_conn().await?;
    let mut query_builder = customers::table.into_boxed();
    query_builder = apply_filters(query_builder, query_info.clone());
    let count = query_builder.count().get_result(&mut conn)?;
    Ok(count)
}

pub async fn get_business_data_by_id(id: Uuid) -> Result<Vec<Customer>, anyhow::Error> {
    let mut conn = get_conn().await?;
    let result = customers::table
        .select(Customer::as_select())
        .filter(customers::id.eq(id))
        .load::<Customer>(&mut conn)?;

    Ok(result)
}

///---------------------------
/// Helper functions
///---------------------------
fn apply_pagination<'a>(
    query_builder: customers::BoxedQuery<'a, Pg>,
    query_info: &'a ListCustomerQuery
) -> customers::BoxedQuery<'a, Pg> {
    let page = query_info.page;
    let limit = query_info.limit;
    let offset = (page - 1) * limit;
    query_builder.limit(limit).offset(offset)
}

fn apply_filters(
    mut query_builder: customers::BoxedQuery<'_, Pg>,
    query_info: ListCustomerQuery
) -> customers::BoxedQuery<'_, Pg> {
    if let Some(name) = query_info.name {
        query_builder = query_builder.filter(customers::name.ilike(sanitize_wildcard(&name)));
    }
    if let Some(email) = query_info.email {
        query_builder = query_builder.filter(customers::email.ilike(sanitize_wildcard(&email)));
    }
    if let Some(business_id) = query_info.business_id {
        if let Ok(business_uuid) = Uuid::parse_str(&business_id) {
            query_builder =
                query_builder.filter(customers::business_id.eq(business_uuid));
        }
    }
    
    query_builder
}