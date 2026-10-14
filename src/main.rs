mod models;
mod routes;
mod services;
mod db;
mod schema;
mod auth;
mod utils;
mod config;
use anyhow::Ok;
use axum::{
    Router, http::StatusCode, middleware, response::{IntoResponse, Json, Response}, routing::{get, post},
};
use tokio::net::TcpListener;
use utoipa::{OpenApi, Modify, openapi::security::{ApiKey, ApiKeyValue, SecurityScheme}};
use utoipa_swagger_ui::SwaggerUi;

use crate::{auth::require_api_key, utils::ApiResponse};
use services::{get_all_businesses, create_business, create_customer, get_all_customers, create_invoice, get_all_invoices, list_invoices, create_payment, list_payments};

use crate::services::{
    business_services::{__path_get_all_businesses, __path_create_business},
    customer_service::{__path_create_customer, __path_get_all_customers},
    invoice_services::{__path_create_invoice, __path_get_all_invoices, __path_list_invoices},
    mock_psp::{__path_create_payment, __path_list_payments}
};

#[utoipa::path(
    get,
    tag = "General",
    path = "/",
    description = "Entry point for the API, providing basic information about the service.", 
    summary = "Entry Point",
    responses(
        (
            status = 200,
            body = ApiResponse,
            description = "success",
        )
    )
)]
pub async fn entry_point() -> Response {
    let payload = ApiResponse {
        message: "Welcome to the Invoice Payment Service!".to_string(),
    };

    // Convert the status and JSON payload explicitly into an HTTP response object
    (StatusCode::OK, Json(payload)).into_response()
}

/// API Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Invoice Payment Service API",
        version = "0.1.0-beta",
        description = format!("API for managing invoice payments"),
        license(
            name = "Apache 2.0",
            url = "https://www.apache.org/licenses/LICENSE-2.0.html"
        )
    ),
    paths(
        entry_point,
        get_all_businesses,
        create_business,
        create_customer,
        get_all_customers,
        create_invoice,
        get_all_invoices,
        list_invoices,
        create_payment, 
        list_payments,
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);

        components.add_security_scheme(
            "ApiKeyAuth",
            SecurityScheme::ApiKey(ApiKey::Header(
                ApiKeyValue::new("x-api-key"),
            )),
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::init()
        .map_err(std::io::Error::other)?;

    let public_routes = Router::new()
        .route("/", get(entry_point))
        .route("/api/business", get(get_all_businesses))
        .route("/api/business", post(create_business));

    let protected_routes = Router::new()
        .route("/api/customer", post(create_customer))
        .route("/api/customer", get(get_all_customers))
        .route("/api/invoice", post(create_invoice))
        .route("/api/invoice", get(list_invoices))
        .route("/api/invoice/{invoice_id}", get(get_all_invoices))
        .route("/api/invoice/{invoice_id}/payment", post(create_payment))
        .route("/api/invoice/{invoice_id}/payments", get(list_payments))
        .route_layer(middleware::from_fn(
            require_api_key,
        ));

    let app = public_routes
        .merge(protected_routes)
        .merge(
            SwaggerUi::new("/api/docs")
                .url("/api-doc/openapi.json", ApiDoc::openapi()),
        );

    // Use the configuration loaded from environment variables.
    let address = format!(
        "{}:{}",
        config.rest_server_host.as_deref().unwrap_or(config::DEFAULT_REST_SERVER_HOST),
        config.rest_server_port.unwrap_or(config::DEFAULT_REST_SERVER_PORT).to_string()
    );

    let listener = TcpListener::bind(&address).await?;

    println!(
        "Server running on http://{}",
        listener.local_addr()?
    );
    println!("{}", config.database_url);

    axum::serve(listener, app).await?;

    Ok(())
}