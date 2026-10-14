use axum::{
    Extension, Json, extract::{Path, Query, State}, http::StatusCode, response::{IntoResponse, Response},
};
use chrono::NaiveDate;
use diesel::{pg::Pg, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedBusiness, db::get_conn, models::{invoice::{Invoice, ListInvoiceQuery, NewInvoice}, invoice_line_item::{InvoiceLineItem, NewInvoiceLineItem}}, schema::{customers, invoice_line_items, invoices},
};

#[derive(Deserialize, ToSchema)]
pub struct CreateLineItemRequest {
    pub description: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateInvoiceRequest {
    pub customer_id: Uuid,
    pub due_date: NaiveDate,
    pub line_items: Vec<CreateLineItemRequest>,
}

/// Create customer data
#[derive(Serialize, ToSchema)]
pub struct CreateInvoiceLineItemsData {
    pub id: String,
    pub invoice_id: String,
    pub description: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
}

/// Create customer data
#[derive(Serialize, ToSchema)]
pub struct CreateInvoiceData {
    pub id: String,
    pub business_id: String,
    pub customer_id: String,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub due_date: String,
    pub invoice_line_item: Vec<CreateInvoiceLineItemsData>,
}

/// Create invoice response model
#[derive(Serialize, ToSchema)]
pub struct CreateInvoiceResponse {
    is_success: bool,
    message: String,
    result: CreateInvoiceData,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListInvoiceData {
    pub id: Uuid,
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub due_date: NaiveDate,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct GetInvoiceData {
    pub id: String,
    pub business_id: String,
    pub customer_id: String,
    pub status: String,
    pub total_cents: i64,
    pub currency: String,
    pub due_date: String,
    pub created_at: String,
    pub updated_at: String,
    pub invoice_line_item: Vec<CreateInvoiceLineItemsData>,
}

/// Success model
#[derive(Serialize, ToSchema)]
pub struct ListInvoiceSuccessResponse {
    is_success: bool,
    message: String,
    page: i64,
    limit: i64,
    record_count: i64,
    total_pages: i64,
    previous: bool,
    next: bool,
    total_records: i64,
    result: Option<Vec<ListInvoiceData>>
}

#[derive(Serialize, ToSchema)]
pub struct GetInvoiceResponse {
    pub is_success: bool,
    pub message: String,
    pub result: Option<GetInvoiceData>,
}

/// Error model
#[derive(Serialize, ToSchema)]
pub struct ListInvoiceErrorResponse {
    is_success: bool,
    message: String,
    result: Option<CreateInvoiceData>
}

const fn build_error_response(is_success: bool, message: String, result: Option<CreateInvoiceData>) -> ListInvoiceErrorResponse {
    ListInvoiceErrorResponse {
        is_success,
        message,
        result
    }
}

struct InvoiceDetails {
    invoice: Invoice,
    line_items: Vec<InvoiceLineItem>,
}

#[utoipa::path(
    post,
    tag = "invoices",
    path = "/api/invoice",
    summary = "Create invoice with line items",
    description = "Creates an invoice and all its line items in one transaction.",
    request_body(
        content = CreateInvoiceRequest,
        example = json!({
            "customer_id": "11111111-1111-4111-8111-111111111111",
            "due_date": "2026-11-15",
            "line_items": [
                {
                    "description": "Web development",
                    "quantity": 2,
                    "unit_price_cents": 50000
                },
                {
                    "description": "Hosting",
                    "quantity": 1,
                    "unit_price_cents": 10000
                }
            ]
        })
    ),
    responses(
        (
            status = 201,
            description = "Invoice and line items created successfully",
            body = CreateInvoiceResponse
        ),
        (
            status = 400,
            description = "Invalid invoice request",
            body = ListInvoiceErrorResponse
        ),
        (
            status = 404,
            description = "Customer not found for this business",
            body = ListInvoiceErrorResponse
        ),
        (
            status = 500,
            description = "Failed to create invoice",
            body = ListInvoiceErrorResponse
        )
    ),
    security(("ApiKeyAuth" = []))
)]
pub async fn create_invoice(
    Extension(business): Extension<AuthenticatedBusiness>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Response {
    // Validate line items and calculate total in cents.
    if payload.line_items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(build_error_response(
                false,
                "Line items are required to create invoice".to_string(),
                None,
            )),
        )
            .into_response();
    }

    let mut total_cents: i64 = 0;

    for item in &payload.line_items {
        if item.description.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(build_error_response(
                    false,
                    "Item description cannot be empty".to_string(),
                    None,
                )),
            )
                .into_response();
        }

        if item.quantity <= 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(build_error_response(
                    false,
                    "Item quantity must be greater than 0".to_string(),
                    None,
                )),
            )
                .into_response();
        }

        if item.unit_price_cents < 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(build_error_response(
                    false,
                    "Unit price cannot be negative".to_string(),
                    None,
                )),
            )
                .into_response();
        }

        let Some(line_total) =
            i64::from(item.quantity).checked_mul(item.unit_price_cents)
        else {
            return (
                StatusCode::BAD_REQUEST,
                Json(build_error_response(
                    false,
                    "Line item total exceeds the supported amount".to_string(),
                    None,
                )),
            )
                .into_response();
        };

        let Some(new_total) = total_cents.checked_add(line_total) else {
            return (
                StatusCode::BAD_REQUEST,
                Json(build_error_response(
                    false,
                    "Invoice total exceeds the supported amount".to_string(),
                    None,
                )),
            )
                .into_response();
        };

        total_cents = new_total;
    }

    let business_id = business.id;
    let customer_id = payload.customer_id;
    let due_date = payload.due_date;

    // Open one connection and use one transaction for every insert.
    let result = async {
        let mut conn = get_conn().await?;

        let details = conn.transaction::<InvoiceDetails, diesel::result::Error, _>(
            |conn| {
                // Ensure the customer belongs to the authenticated business.
                let customer_exists = customers::table
                    .filter(customers::id.eq(customer_id))
                    .filter(customers::business_id.eq(business_id))
                    .select(customers::id)
                    .first::<Uuid>(conn)
                    .optional()?
                    .is_some();

                if !customer_exists {
                    return Err(diesel::result::Error::NotFound);
                }

                // 1. Insert the invoice.
                let new_invoice = NewInvoice {
                    business_id,
                    customer_id,
                    status: "open".to_string(),
                    total_cents,
                    currency: "USD".to_string(),
                    due_date,
                };

                let invoice = diesel::insert_into(invoices::table)
                    .values(&new_invoice)
                    .returning(Invoice::as_returning())
                    .get_result::<Invoice>(conn)?;

                // 2. Insert all line items using the generated invoice ID.
                let mut saved_items =
                    Vec::with_capacity(payload.line_items.len());

                for item in &payload.line_items {
                    let description = item.description.trim();

                    let new_item = NewInvoiceLineItem {
                        invoice_id: invoice.id,
                        description,
                        quantity: item.quantity,
                        unit_price_cents: item.unit_price_cents,
                    };

                    let saved_item =
                        diesel::insert_into(invoice_line_items::table)
                            .values(&new_item)
                            .returning(InvoiceLineItem::as_returning())
                            .get_result::<InvoiceLineItem>(conn)?;

                    saved_items.push(saved_item);
                }

                Ok(InvoiceDetails {
                    invoice,
                    line_items: saved_items,
                })
            },
        )?;

        Ok::<InvoiceDetails, anyhow::Error>(details)
    }
    .await;

    match result {
        Ok(details) => {
            let invoice = details.invoice;

            let response_data = CreateInvoiceData {
                id: invoice.id.to_string(),
                business_id: invoice.business_id.to_string(),
                customer_id: invoice.customer_id.to_string(),
                status: invoice.status,
                total_cents: invoice.total_cents,
                currency: invoice.currency,
                due_date: invoice.due_date.to_string(),
                invoice_line_item: details
                    .line_items
                    .into_iter()
                    .map(|item| CreateInvoiceLineItemsData {
                        id: item.id.to_string(),
                        invoice_id: item.invoice_id.to_string(),
                        description: item.description,
                        quantity: item.quantity,
                        unit_price_cents: item.unit_price_cents,
                    })
                    .collect(),
            };

            (
                StatusCode::CREATED,
                Json(CreateInvoiceResponse {
                    is_success: true,
                    message:
                        "Invoice and line items created successfully".to_string(),
                    result: response_data,
                }),
            )
                .into_response()
        }

        Err(error) => {
            eprintln!("Failed to create invoice: {error:?}");

            let is_customer_not_found =
                matches!(
                    error.downcast_ref::<diesel::result::Error>(),
                    Some(diesel::result::Error::NotFound)
                );

            let (status, message) = if is_customer_not_found {
                (
                    StatusCode::NOT_FOUND,
                    "Customer not found for this business".to_string(),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to create invoice".to_string(),
                )
            };

            (
                status,
                Json(build_error_response(false, message, None)),
            )
                .into_response()
        }
    }
}



#[utoipa::path(
    get,
    tag = "invoices",
    path = "/api/invoice",
    summary = "List invoices",
    description = "Returns paginated invoices for the authenticated business.",
    params(
        ("page" = Option<i64>, Query, description = "Page number", example=1),
        ("limit" = Option<i64>, Query, description = "Records per page", example=10),
        ("customer_id" = Option<String>, Query, description = "Filter by customer UUID"),
        ("status" = Option<String>, Query, description = "Filter by invoice status")
    ),
    responses(
        (status = 200, description = "Invoices fetched successfully",
         body = ListInvoiceSuccessResponse),
        (status = 400, description = "Failed to fetch invoices",
         body = ListInvoiceErrorResponse)
    ),
    security(("ApiKeyAuth" = []))
)]
pub async fn list_invoices(
    Extension(business): Extension<AuthenticatedBusiness>,
    Query(query): Query<ListInvoiceQuery>,
) -> Response {
    // Validate pagination before running the query.
    if query.page < 1 || query.limit < 1 || query.limit > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ListInvoiceErrorResponse {
                is_success: false,
                message: "Page must be positive and limit must be between 1 and 100"
                    .to_string(),
                result: None,
            }),
        )
            .into_response();
    }

    let business_id = business.id;

    // Fetch invoices and count records using the same filters.
    let result = async {
        let mut conn = get_conn().await?;

        let mut count_query = invoices::table
            .filter(invoices::business_id.eq(business_id))
            .into_boxed::<Pg>();

        let mut data_query = invoices::table
            .filter(invoices::business_id.eq(business_id))
            .into_boxed::<Pg>();

        if let Some(customer_id) = &query.customer_id {
            let customer_uuid = Uuid::parse_str(customer_id)
                .map_err(anyhow::Error::new)?;

            count_query =
                count_query.filter(invoices::customer_id.eq(customer_uuid));

            data_query =
                data_query.filter(invoices::customer_id.eq(customer_uuid));
        }

        if let Some(status) = &query.status {
            count_query =
                count_query.filter(invoices::status.eq(status.clone()));

            data_query =
                data_query.filter(invoices::status.eq(status.clone()));
        }

        let total_records: i64 = count_query.count().get_result(&mut conn)?;

        let total_pages =
            (total_records + query.limit - 1) / query.limit;

        let offset = (query.page - 1)
            .checked_mul(query.limit)
            .ok_or_else(|| anyhow::anyhow!("Invalid pagination offset"))?;

        let records = data_query
            .order(invoices::created_at.desc())
            .limit(query.limit)
            .offset(offset)
            .select(Invoice::as_select())
            .load::<Invoice>(&mut conn)?;

        Ok::<_, anyhow::Error>((total_records, total_pages, records))
    }
    .await;

    match result {
        Ok((total_records, total_pages, records)) => {
            let result: Vec<ListInvoiceData> = records
                .into_iter()
                .map(|invoice| ListInvoiceData {
                    id: invoice.id,
                    business_id: invoice.business_id,
                    customer_id: invoice.customer_id,
                    status: invoice.status,
                    total_cents: invoice.total_cents,
                    currency: invoice.currency,
                    due_date: invoice.due_date,
                    created_at: invoice.created_at.to_rfc3339(),
                    updated_at: invoice.updated_at.to_rfc3339(),
                })
                .collect();

            (
                StatusCode::OK,
                Json(ListInvoiceSuccessResponse {
                    is_success: true,
                    message: "Invoice data fetched successfully".to_string(),
                    page: query.page,
                    limit: query.limit,
                    record_count: result.len() as i64,
                    total_pages,
                    previous: query.page > 1,
                    next: query.page < total_pages,
                    total_records,
                    result: if result.is_empty() {
                        None
                    } else {
                        Some(result)
                    },
                }),
            )
                .into_response()
        }
        Err(error) => {
            eprintln!("Failed to fetch invoices: {error:?}");

            (
                StatusCode::BAD_REQUEST,
                Json(ListInvoiceErrorResponse {
                    is_success: false,
                    message: "Failed to fetch invoice data".to_string(),
                    result: None,
                }),
            )
                .into_response()
        }
    }
}


#[utoipa::path(
    get,
    tag = "invoices",
    path = "/api/invoice/{invoice_id}",
    summary = "Get invoice by ID",
    description = "Returns an invoice and all its line items.",
    params(
        ("invoice_id" = Uuid, Path, description = "Invoice UUID")
    ),
    responses(
        (status = 200, description = "Invoice fetched successfully",
         body = GetInvoiceResponse),
        (status = 404, description = "Invoice not found",
         body = GetInvoiceResponse),
        (status = 500, description = "Failed to fetch invoice",
         body = GetInvoiceResponse)
    ),
    security(("ApiKeyAuth" = []))
)]
pub async fn get_all_invoices(
    Extension(business): Extension<AuthenticatedBusiness>,
    Path(invoice_id): Path<Uuid>,
) -> Response {
    let business_id = business.id;

    let result = async {
        let mut conn = get_conn().await?;

        // Find the invoice within the authenticated business.
        let invoice = invoices::table
            .filter(invoices::id.eq(invoice_id))
            .filter(invoices::business_id.eq(business_id))
            .select(Invoice::as_select())
            .first::<Invoice>(&mut conn)
            .optional()?
            .ok_or(diesel::result::Error::NotFound)?;

        // Retrieve all line items associated with the invoice.
        let items = invoice_line_items::table
            .filter(invoice_line_items::invoice_id.eq(invoice.id))
            .order(invoice_line_items::created_at.asc())
            .select(InvoiceLineItem::as_select())
            .load::<InvoiceLineItem>(&mut conn)?;

        Ok::<_, anyhow::Error>((invoice, items))
    }
    .await;

    match result {
        Ok((invoice, items)) => {
            let response_data = GetInvoiceData {
                id: invoice.id.to_string(),
                business_id: invoice.business_id.to_string(),
                customer_id: invoice.customer_id.to_string(),
                status: invoice.status,
                total_cents: invoice.total_cents,
                currency: invoice.currency,
                due_date: invoice.due_date.to_string(),
                created_at: invoice.created_at.to_rfc3339(),
                updated_at: invoice.updated_at.to_rfc3339(),
                invoice_line_item: items
                    .into_iter()
                    .map(|item| CreateInvoiceLineItemsData {
                        id: item.id.to_string(),
                        invoice_id: item.invoice_id.to_string(),
                        description: item.description,
                        quantity: item.quantity,
                        unit_price_cents: item.unit_price_cents,
                    })
                    .collect(),
            };

            (
                StatusCode::OK,
                Json(GetInvoiceResponse {
                    is_success: true,
                    message: "Invoice data fetched successfully".to_string(),
                    result: Some(response_data),
                }),
            )
                .into_response()
        }
        Err(error) => {
            eprintln!("Failed to fetch invoice: {error:?}");

            let not_found = matches!(
                error.downcast_ref::<diesel::result::Error>(),
                Some(diesel::result::Error::NotFound)
            );

            let (status, message) = if not_found {
                (
                    StatusCode::NOT_FOUND,
                    "Invoice not found".to_string(),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch invoice data".to_string(),
                )
            };

            (
                status,
                Json(GetInvoiceResponse {
                    is_success: false,
                    message,
                    result: None,
                }),
            )
                .into_response()
        }
    }
}