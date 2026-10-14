// @generated automatically by Diesel CLI.

diesel::table! {
    businesses (id) {
        id -> Uuid,
        name -> Text,
        api_key_hash -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    customers (id) {
        id -> Uuid,
        business_id -> Uuid,
        name -> Text,
        email -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    idempotency_keys (id) {
        id -> Uuid,
        business_id -> Uuid,
        operation -> Text,
        idempotency_key -> Text,
        request_hash -> Text,
        response_status -> Nullable<Int4>,
        response_body -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    invoice_line_items (id) {
        id -> Uuid,
        invoice_id -> Uuid,
        description -> Text,
        quantity -> Int4,
        unit_price_cents -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    invoices (id) {
        id -> Uuid,
        business_id -> Uuid,
        customer_id -> Uuid,
        status -> Text,
        total_cents -> Int8,
        #[max_length = 3]
        currency -> Bpchar,
        due_date -> Date,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    payment_attempts (id) {
        id -> Uuid,
        business_id -> Uuid,
        invoice_id -> Uuid,
        status -> Text,
        amount_cents -> Int8,
        #[max_length = 3]
        currency -> Bpchar,
        psp_reference -> Nullable<Text>,
        failure_reason -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    webhook_endpoints (id) {
        id -> Uuid,
        business_id -> Uuid,
        url -> Text,
        secret_hash -> Nullable<Text>,
        enabled -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    webhook_events (id) {
        id -> Uuid,
        business_id -> Uuid,
        endpoint_id -> Uuid,
        invoice_id -> Nullable<Uuid>,
        event_type -> Text,
        payload -> Jsonb,
        status -> Text,
        attempt_count -> Int4,
        next_attempt_at -> Timestamptz,
        last_error -> Nullable<Text>,
        created_at -> Timestamptz,
        delivered_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(customers -> businesses (business_id));
diesel::joinable!(idempotency_keys -> businesses (business_id));
diesel::joinable!(invoice_line_items -> invoices (invoice_id));
diesel::joinable!(invoices -> businesses (business_id));
diesel::joinable!(payment_attempts -> businesses (business_id));
diesel::joinable!(webhook_endpoints -> businesses (business_id));
diesel::joinable!(webhook_events -> businesses (business_id));
diesel::joinable!(webhook_events -> invoices (invoice_id));
diesel::joinable!(webhook_events -> webhook_endpoints (endpoint_id));

diesel::allow_tables_to_appear_in_same_query!(
    businesses,
    customers,
    idempotency_keys,
    invoice_line_items,
    invoices,
    payment_attempts,
    webhook_endpoints,
    webhook_events,
);
