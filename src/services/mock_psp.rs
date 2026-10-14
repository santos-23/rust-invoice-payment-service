use axum::{
    extract::{Path, Query},
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedBusiness,
    db::get_conn,
    models::{
        invoice::Invoice,
        payment_attempt::{NewPaymentAttempt, PaymentAttempt},
    },
    schema::{invoices, payment_attempts},
};

// Keep your existing PspOutcome enum and process_payment()
// implementation, or import them from your mock PSP module.

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentRequest {
    #[serde(default)]
    pub simulate_failure: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatePaymentResponse {
    pub payment: PaymentAttempt,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentErrorResponse {
    pub is_success: bool,
    pub message: String,
    pub result: Option<PaymentAttempt>,
}

fn payment_error(
    message: impl Into<String>,
) -> PaymentErrorResponse {
    PaymentErrorResponse {
        is_success: false,
        message: message.into(),
        result: None,
    }
}

#[derive(Debug, Clone)]
pub enum PspOutcome {
    Succeeded {
        reference: String,
    },
    Failed {
        reason: String,
    },
}

/// Simulates a PSP response. No real payment is processed.
pub async fn process_payment(
    payment_attempt_id: Uuid,
    simulate_failure: bool,
) -> PspOutcome {
    if simulate_failure {
        PspOutcome::Failed {
            reason: "mock_card_declined".to_string(),
        }
    } else {
        PspOutcome::Succeeded {
            reference: format!("mock_psp_{}", payment_attempt_id.simple()),
        }
    }
}

#[utoipa::path(
    post,
    tag = "payments",
    path = "/api/invoice/{invoice_id}/payment",
    summary = "Create payment",
    description = "Creates a payment attempt for an invoice belonging to the authenticated business and simulates a payment provider response.",
    params(
        (
            "invoice_id" = Uuid,
            Path,
            description = "Invoice UUID"
        )
    ),
    request_body(
        content = CreatePaymentRequest,
        example = json!({
            "simulate_failure": false
        })
    ),
    responses(
        (
            status = 201,
            description = "Payment succeeded",
            body = CreatePaymentResponse,
            example = json!({
                "payment": {
                    "id": "22222222-2222-4222-8222-222222222222",
                    "business_id": "33333333-3333-4333-8333-333333333333",
                    "invoice_id": "11111111-1111-4111-8111-111111111111",
                    "status": "succeeded",
                    "amount_cents": 110000,
                    "currency": "USD",
                    "psp_reference": "mock_psp_22222222222242228222222222222222",
                    "failure_reason": null,
                    "created_at": "2026-10-14T12:00:00Z",
                    "updated_at": "2026-10-14T12:00:01Z"
                },
                "message": "Payment succeeded"
            })
        ),
        (
            status = 200,
            description = "Payment failed",
            body = CreatePaymentResponse,
            example = json!({
                "payment": {
                    "id": "22222222-2222-4222-8222-222222222222",
                    "business_id": "33333333-3333-4333-8333-333333333333",
                    "invoice_id": "11111111-1111-4111-8111-111111111111",
                    "status": "failed",
                    "amount_cents": 110000,
                    "currency": "USD",
                    "psp_reference": null,
                    "failure_reason": "mock_card_declined",
                    "created_at": "2026-10-14T12:00:00Z",
                    "updated_at": "2026-10-14T12:00:01Z"
                },
                "message": "Payment failed"
            })
        ),
        (
            status = 400,
            description = "Invoice total must be greater than zero"
        ),
        (
            status = 401,
            description = "Unauthorized"
        ),
        (
            status = 404,
            description = "Invoice not found"
        ),
        (
            status = 409,
            description = "Invoice cannot be paid in its current status"
        ),
        (
            status = 500,
            description = "Failed to create or update payment attempt"
        )
    ),
    security(
        ("ApiKeyAuth" = [])
    )
)]
pub async fn create_payment(
    Extension(business): Extension<AuthenticatedBusiness>,
    Path(invoice_id): Path<Uuid>,
    Json(payload): Json<CreatePaymentRequest>,
) -> Response {
    let business_id = business.id;
    println!("{:?}", business_id);

    // Step A: Find the invoice and create a pending attempt.
    let setup = async {
        let mut conn = get_conn()
            .await
            .map_err(|error| {
                eprintln!("Database connection failed: {error:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let invoice = invoices::table
            .filter(invoices::id.eq(invoice_id))
            .filter(invoices::business_id.eq(business_id))
            .select(Invoice::as_select())
            .first::<Invoice>(&mut conn)
            .optional()
            .map_err(|error| {
                eprintln!("Invoice lookup failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        println!("{:?}", invoice);

        let Some(invoice) = invoice else {
            return Err(StatusCode::NOT_FOUND);
        };

        if !matches!(invoice.status.as_str(), "open" | "overdue") {
            return Err(StatusCode::CONFLICT);
        }

        if invoice.total_cents <= 0 {
            return Err(StatusCode::BAD_REQUEST);
        }

        let new_attempt = NewPaymentAttempt {
            business_id,
            invoice_id,
            status: "pending".to_string(),
            amount_cents: invoice.total_cents,
            currency: invoice.currency,
        };

        diesel::insert_into(payment_attempts::table)
            .values(&new_attempt)
            .returning(PaymentAttempt::as_returning())
            .get_result::<PaymentAttempt>(&mut conn)
            .map_err(|error| {
                eprintln!("Payment attempt creation failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
    }
    .await;

    let setup = match setup {
        Ok(payment) => payment,
        Err(status) => {
            let message = match status {
                StatusCode::NOT_FOUND => "Invoice not found",
                StatusCode::CONFLICT => "Invoice cannot be paid in its current status",
                StatusCode::BAD_REQUEST => "Invoice total must be greater than zero",
                _ => "Failed to create payment attempt",
            };

            return (
                status,
                Json(payment_error(message)),
            )
                .into_response();
        }
    };

    // Step B: Simulate the payment provider response.
    let outcome = process_payment(
        setup.id,
        payload.simulate_failure,
    )
    .await;

    // Step C: Save the result atomically.
    let updated = async {
        let mut conn = get_conn()
            .await
            .map_err(|error| {
                eprintln!("Database connection failed: {error:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        conn.transaction::<PaymentAttempt, diesel::result::Error, _>(
            |conn| {
                let now = Utc::now();

                match outcome {
                    PspOutcome::Succeeded { reference } => {
                        let payment = diesel::update(
                            payment_attempts::table
                                .filter(payment_attempts::id.eq(setup.id))
                                .filter(
                                    payment_attempts::business_id
                                        .eq(business_id),
                                )
                                .filter(
                                    payment_attempts::status.eq("pending"),
                                ),
                        )
                        .set((
                            payment_attempts::status.eq("succeeded"),
                            payment_attempts::psp_reference
                                .eq(Some(reference)),
                            payment_attempts::failure_reason
                                .eq(None::<String>),
                            payment_attempts::updated_at.eq(now),
                        ))
                        .returning(PaymentAttempt::as_returning())
                        .get_result::<PaymentAttempt>(conn)?;

                        // Only mark the invoice paid after success.
                        diesel::update(
                            invoices::table
                                .filter(
                                    invoices::id.eq(payment.invoice_id),
                                )
                                .filter(
                                    invoices::business_id.eq(business_id),
                                )
                                .filter(
                                    invoices::status.ne("paid"),
                                ),
                        )
                        .set((
                            invoices::status.eq("paid"),
                            invoices::updated_at.eq(now),
                        ))
                        .execute(conn)?;

                        Ok(payment)
                    }

                    PspOutcome::Failed { reason } => {
                        diesel::update(
                            payment_attempts::table
                                .filter(payment_attempts::id.eq(setup.id))
                                .filter(
                                    payment_attempts::business_id
                                        .eq(business_id),
                                )
                                .filter(
                                    payment_attempts::status.eq("pending"),
                                ),
                        )
                        .set((
                            payment_attempts::status.eq("failed"),
                            payment_attempts::failure_reason
                                .eq(Some(reason)),
                            payment_attempts::updated_at.eq(now),
                        ))
                        .returning(PaymentAttempt::as_returning())
                        .get_result::<PaymentAttempt>(conn)
                    }
                }
            },
        )
        .map_err(|error| {
            eprintln!("Payment result update failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
    .await;

    let updated_payment = match updated {
        Ok(payment) => payment,
        Err(status) => {
            return (
                status,
                Json(payment_error("Failed to update payment result")),
            )
                .into_response();
        }
    };

    let message = match updated_payment.status.as_str() {
        "succeeded" => "Payment succeeded",
        "failed" => "Payment failed",
        _ => "Payment is pending",
    };

    let status_code = if updated_payment.status == "failed" {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    (
        status_code,
        Json(CreatePaymentResponse {
            payment: updated_payment,
            message: message.to_string(),
        }),
    )
        .into_response()
}

#[utoipa::path(
    get,
    tag = "payments",
    path = "/api/invoice/{invoice_id}/payments",
    summary = "List invoice payments",
    description = "Returns payment attempts for the specified invoice, provided the invoice belongs to the authenticated business. Results are ordered by creation date, newest first.",
    params(
        (
            "invoice_id" = Uuid,
            Path,
            description = "Invoice UUID"
        )
    ),
    responses(
        (
            status = 200,
            description = "Payment attempts fetched successfully",
            body = [PaymentAttempt],
            example = json!([
                {
                    "id": "22222222-2222-4222-8222-222222222222",
                    "business_id": "33333333-3333-4333-8333-333333333333",
                    "invoice_id": "11111111-1111-4111-8111-111111111111",
                    "status": "succeeded",
                    "amount_cents": 110000,
                    "currency": "USD",
                    "psp_reference": "mock_psp_22222222222242228222222222222222",
                    "failure_reason": null,
                    "created_at": "2026-10-14T12:00:00Z",
                    "updated_at": "2026-10-14T12:00:01Z"
                }
            ])
        ),
        (
            status = 401,
            description = "Unauthorized"
        ),
        (
            status = 404,
            description = "Invoice not found"
        ),
        (
            status = 500,
            description = "Failed to fetch payment attempts"
        )
    ),
    security(
        ("ApiKeyAuth" = [])
    )
)]
pub async fn list_payments(
    Extension(business): Extension<AuthenticatedBusiness>,
    Path(invoice_id): Path<Uuid>,
) -> Response {
    let business_id = business.id;

    let result = async {
        let mut conn = get_conn()
            .await
            .map_err(|error| {
                eprintln!("Database connection failed: {error:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Verify that this invoice belongs to the business.
        let invoice_exists = invoices::table
            .filter(invoices::id.eq(invoice_id))
            .filter(invoices::business_id.eq(business_id))
            .select(invoices::id)
            .first::<Uuid>(&mut conn)
            .optional()
            .map_err(|error| {
                eprintln!("Invoice ownership check failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .is_some();

        if !invoice_exists {
            return Err(StatusCode::NOT_FOUND);
        }

        payment_attempts::table
            .filter(payment_attempts::business_id.eq(business_id))
            .filter(payment_attempts::invoice_id.eq(invoice_id))
            .order(payment_attempts::created_at.desc())
            .select(PaymentAttempt::as_select())
            .load::<PaymentAttempt>(&mut conn)
            .map_err(|error| {
                eprintln!("Payment listing failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
    }
    .await;

    match result {
        Ok(payments) => Json(payments).into_response(),

        Err(status) => {
            let message = if status == StatusCode::NOT_FOUND {
                "Invoice not found"
            } else {
                "Failed to fetch payment attempts"
            };

            (
                status,
                Json(payment_error(message)),
            )
                .into_response()
        }
    }
}