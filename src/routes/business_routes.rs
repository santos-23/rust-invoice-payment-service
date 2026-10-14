// use axum::body::Body;
// use axum::extract::Request;
// use axum::{
//     extract::Path,
//     http::StatusCode,
//     Json,
// };
// use utoipa::openapi::security::ApiKey::Query;

// use crate::models::business::{Business, ListBusinessQuery};
// use crate::services::business_services;

// pub async fn get_businesses(
//     request: Request<Body>,
//     Query(business_query): Query<ListBusinessQuery>,
// ) -> Json<Vec<Business>> {
//     Json(business_services::get_all_businesses(request, business_query).await)
// }