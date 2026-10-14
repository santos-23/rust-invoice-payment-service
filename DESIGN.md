# Invoice & Payment Service — Design Document

## 1. Project Overview

Build a multi-tenant invoice and payment backend using **Rust, Axum, PostgreSQL, and Diesel ORM**. The service allows businesses to manage customers, create invoices with line items, process payment attempts through a mock Payment Service Provider (PSP), and deliver payment-related webhooks reliably.

The project emphasizes secure API-key authentication, tenant isolation, accurate monetary calculations, transaction safety, idempotency, and failure recovery.

### Technology Stack

- **Language:** Rust
- **HTTP framework:** Axum
- **Database:** PostgreSQL
- **ORM and migrations:** Diesel ORM and Diesel CLI
- **Async runtime:** Tokio
- **Serialization:** Serde / Serde JSON
- **Authentication:** Business API keys hashed using SHA-256
- **Payment integration:** Mock PSP
- **Containerization:** Docker and Docker Compose
- **Testing:** Rust unit tests and integration tests

## 2. Project Scope and Status

The implementation is divided into four stages.

| Stage | Scope | Status |
|---|---|---|
| 1 | API-key authentication, customers, invoices and line items | Drafted; compilation and testing still required |
| 2 | Payment attempts and mock PSP integration | Drafted; compilation and testing still required |
| 3 | Idempotency, concurrency safety, webhook outbox and retries | Remaining |
| 4 | Automated tests, Docker setup, documentation and demo | Remaining |

**Important:** The Stage 1 and Stage 2 code has been drafted but has not yet been confirmed to compile or pass tests. Update these statuses after verification.

## 3. Architecture

The service follows a layered architecture:

1. **HTTP layer:** Axum routes, request validation and HTTP responses.
2. **Authentication layer:** Validates the `X-API-Key` header and identifies the business.
3. **Application/service layer:** Implements invoice, payment and webhook workflows.
4. **Persistence layer:** Diesel ORM, PostgreSQL transactions and database constraints.
5. **Integration layer:** Mock PSP and webhook delivery worker.

### Request Flow

A client sends an HTTP request containing its API key. Authentication middleware hashes the key and looks up the associated business. Protected handlers use that business ID to scope database queries and writes.

The service must never trust a client-provided `business_id` for authorization.

## 4. Data Model

### Businesses

Stores each registered business and its API-key hash.

- `id`: UUID primary key
- `name`: Business name
- `api_key_hash`: Unique hash of the API key
- `created_at`: Creation timestamp

The raw API key should be returned only when created and must not be stored in plaintext or written to logs.

### Customers

Stores customers belonging to businesses.

- `id`: UUID primary key
- `business_id`: Business owner
- `name`: Customer name
- `email`: Customer email
- `created_at`: Creation timestamp

### Invoices

Stores invoice-level information.

- `id`: UUID primary key
- `business_id`: Business owner
- `customer_id`: Associated customer
- `status`: `draft`, `open`, `paid`, or `overdue`
- `total_cents`: Invoice total in integer cents
- `currency`: USD
- `due_date`: Due date
- `created_at` and `updated_at`: Timestamps

Invoice-customer relationships must enforce matching business ownership at the database level.

### Invoice Line Items

Each invoice can contain multiple line items.

- `id`: UUID primary key
- `invoice_id`: Associated invoice
- `description`: Item description
- `quantity`: Positive integer
- `unit_price_cents`: Non-negative integer

Calculate totals using checked integer arithmetic. Never use floating-point values for monetary calculations.

### Payment Attempts

Stores individual attempts to pay an invoice.

- `id`: UUID primary key
- `business_id`: Business owner
- `invoice_id`: Associated invoice
- `status`: `pending`, `succeeded`, or `failed`
- `amount_cents`: Amount charged in cents
- `currency`: USD
- `psp_reference`: Mock provider reference
- `failure_reason`: Optional failure reason
- `created_at` and `updated_at`: Timestamps

A failed attempt must not mark an invoice as paid.

### Remaining Stage 3 Tables

Implement these tables during Stage 3.

**Idempotency keys**
- Business ID and operation
- Client-supplied idempotency key
- Request hash
- Stored response status and body
- Creation and expiration timestamps
- Unique constraint on business, operation and key

**Webhook endpoints**
- Business ID
- Destination URL
- Signing configuration
- Enabled state
- Creation timestamp

**Webhook events / outbox**
- Event ID and business ID
- Event type and JSON payload
- Delivery status
- Attempt count
- Next retry timestamp
- Last error
- Creation and update timestamps

## 5. API Design

All protected endpoints require the `X-API-Key` header.

### Business Registration

`POST /api/businesses`

Creates a business and returns its generated API key once.

Example request:

```json
{
  "name": "Example Business"
}
```

### Customers

`POST /api/customers`

Creates a customer for the authenticated business.

```json
{
  "name": "Jane Doe",
  "email": "jane@example.com"
}
```

`GET /api/customers`

Lists customers belonging to the authenticated business.

### Invoices

`POST /api/invoices`

Creates an invoice with line items.

```json
{
  "customer_id": "CUSTOMER_UUID",
  "due_date": "2026-11-01",
  "line_items": [
    {
      "description": "Backend development",
      "quantity": 2,
      "unit_price_cents": 15000
    }
  ]
}
```

The expected total is calculated by the server. In this example, the total is 30,000 cents, or $300.00.

`GET /api/invoices`

Lists invoices belonging to the authenticated business.

`GET /api/invoices/{invoice_id}`

Returns an invoice and its line items, after checking business ownership.

### Payments

`POST /api/invoices/{invoice_id}/payments`

Creates a payment attempt for an eligible invoice.

The request must include an `Idempotency-Key` header once Stage 3 is implemented.

The current mock implementation accepts a development-only failure simulation flag:

```json
{
  "simulate_failure": false
}
```

This flag exists only for testing. A production payment integration must use provider responses rather than trusting a client-controlled success or failure value.

`GET /api/invoices/{invoice_id}/payments`

Lists payment attempts for an invoice owned by the authenticated business.

### Webhooks — Remaining

Implement endpoints to register webhook destinations and inspect delivery status, subject to the assignment requirements.

## 6. Authentication and Tenant Isolation

Authentication middleware must:

1. Read the `X-API-Key` header.
2. Reject a missing or malformed key with HTTP 401.
3. Hash the supplied key.
4. Find the corresponding business.
5. Reject an unknown key with HTTP 401.
6. Attach the authenticated business ID to the request.
7. Allow protected handlers to continue.

Every customer, invoice, payment and webhook query must be scoped to the authenticated business.

Return appropriate HTTP errors:
- `400 Bad Request`: Invalid request data
- `401 Unauthorized`: Missing or invalid API key
- `404 Not Found`: Resource does not exist or is not accessible to the current business
- `409 Conflict`: Idempotency conflict or incompatible state transition
- `500 Internal Server Error`: Unexpected internal failure

Do not expose database credentials, raw API keys, or sensitive internal errors in responses.

## 7. Invoice and Payment State Machines

### Invoice

```text
draft -> open -> paid
             -> overdue -> paid
```

Only valid transitions should be allowed. The system should define when an invoice becomes overdue and ensure that a failed payment does not incorrectly change its state to paid.

### Payment Attempt

```text
pending -> succeeded
        -> failed
```

Each attempt represents one payment operation. A retry should create a new attempt unless an existing idempotent request is being replayed.

## 8. Stage 1 — Customers and Invoices

The initial implementation covers:

- Business creation and API-key generation
- API-key authentication middleware
- Customer creation and listing
- Invoice creation and listing
- Invoice detail retrieval
- Invoice line-item persistence
- Server-side total calculation
- Database constraints for tenant ownership

### Verification Required

- Generate Diesel schema from the actual migrations.
- Run `cargo fmt`.
- Run `cargo check`.
- Test missing and invalid API keys.
- Test invoice creation with valid and invalid customers.
- Confirm one business cannot access another business's records.
- Confirm a failed line-item insert rolls back the invoice transaction.

## 9. Stage 2 — Mock PSP Integration

The payment endpoint should:

1. Verify invoice existence and ownership.
2. Check that the invoice is eligible for payment.
3. Create a pending payment attempt.
4. Call the mock PSP.
5. Persist the final payment result.
6. Mark the invoice paid only after a successful result.
7. Preserve an eligible invoice state after failure.

The mock PSP returns either a success reference or a failure reason.

### Current Limitations

The drafted implementation performs the PSP simulation between database writes. A process crash or database failure after simulated provider success could leave an attempt pending. Concurrent payment requests may also race.

Stage 3 must address these failure modes. The implementation must not be considered production-ready until the concurrency and recovery behavior is tested.

## 10. Stage 3 — Remaining Work

### 10.1 Idempotency

Require `Idempotency-Key` for payment creation.

- Scope keys by business and operation.
- Store a hash of the request and the resulting response.
- Return the original response for an identical retry.
- Return HTTP 409 when a key is reused with a different request.
- Enforce uniqueness in PostgreSQL to handle concurrent requests.
- Test replay behavior and concurrent requests using the same key.

### 10.2 Concurrency Safety

Prevent multiple concurrent requests from successfully charging the same invoice unintentionally.

- Lock or atomically reserve the invoice before starting payment processing.
- Enforce valid state transitions.
- Use database constraints where appropriate.
- Do not hold a database transaction open while waiting for a real external PSP.
- Use provider-side idempotency and reconciliation to handle crashes after provider success but before local persistence.
- Define recovery for stale pending attempts.

### 10.3 Webhook Outbox and Retries

Persist webhook events in the same database transaction that records the payment and invoice state change.

A background worker must:

- Select pending events safely.
- Deliver events with HTTP timeouts.
- Treat successful HTTP responses as delivered.
- Retry transient failures.
- Use bounded exponential backoff with jitter.
- Record attempt counts and the last error.
- Stop after a configured retry limit and retain failed events for inspection.
- Sign payloads if webhook signature verification is part of the contract.
- Avoid duplicate side effects when an event is delivered more than once.

The outbox ensures the payment result and the intention to notify a subscriber are committed together.

## 11. Stage 4 — Remaining Work

### 11.1 Automated Tests

Add unit and integration tests for:

- API-key hashing
- Authentication success and failure
- Tenant isolation
- Invoice total calculation and overflow handling
- Invoice transaction rollback
- Successful and failed payment attempts
- Invalid invoice state transitions
- Idempotent replay
- Idempotency-key reuse with a different request
- Concurrent payment requests
- Webhook delivery and retry behavior

Use a test database and ensure tests clean up their own data or use isolated fixtures.

### 11.2 Docker and Local Setup

Create:

- `Dockerfile`
- `docker-compose.yml`
- PostgreSQL service with a persistent volume
- Database health check
- API service configured through environment variables
- A reliable migration step before the API accepts traffic

A fresh clone should be runnable using documented commands without manually editing source code.

Do not commit `.env`, passwords, API keys or other secrets.

### 11.3 Documentation

Update this document to reflect the final implementation rather than intended behavior.

Include:

- Prerequisites
- Environment variables
- Migration commands
- Local startup commands
- Docker Compose startup commands
- API request examples
- Test commands
- Design decisions and known limitations


## 12. Suggested Project Structure

```text
rust-invoice-payment-service/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── Dockerfile
├── docker-compose.yml
├── DESIGN.md
├── AI_USAGE.md
├── migrations/
├── src/
│   ├── main.rs
│   ├── db.rs
│   ├── schema.rs
│   ├── models.rs
│   ├── auth.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── businesses.rs
│   │   ├── customers.rs
│   │   ├── invoices.rs
│   │   └── payments.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── mock_psp.rs
│   │   └── webhooks.rs
│   └── workers/
│       └── webhook_worker.rs
└── tests/
    ├── auth_tests.rs
    ├── invoice_tests.rs
    ├── payment_tests.rs
    └── webhook_tests.rs
```

Adjust this structure to match the actual source files as the project evolves.
