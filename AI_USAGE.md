# AI_USAGE.md

## 1. AI Tools Used and How

I used ChatGPT as a development assistant while building the Invoice Payment Service in Rust.

- **Requirement analysis:** Used ChatGPT to break down the assignment into components, including API-key authentication, business and customer management, invoices, payment attempts, mock PSP integration, transactions, idempotency, and webhook delivery.
- **Database design:** Used ChatGPT to discuss the PostgreSQL schema, table relationships, foreign keys, and constraints. I reviewed the schema against the requirements before using it.
- **Rust and Axum implementation:** Used ChatGPT to clarify Axum handlers, request/response structures, error handling, and database integration patterns.
- **Payment flow design:** Used ChatGPT to reason about payment attempts and mock payment provider integration, including success and failure scenarios.
- **Documentation:** Used ChatGPT to help organize the design document and document completed work, outstanding tasks, and implementation decisions.

AI-generated suggestions were treated as reference material rather than blindly accepted. I remained responsible for the final implementation and design choices.

## 2. Three Engineering Decisions I Made Independently

### Decision 1: Separate invoices from payment attempts

**AI suggestion:** A simpler implementation could handle payment status directly on the invoice.

**My decision:** I chose to keep invoices and payment attempts as separate concepts.

**Why:** An invoice represents the amount owed, while a payment attempt represents an individual effort to collect that amount. Separating them makes retries, failed payments, and payment history easier to model without losing previous attempts.

### Decision 2: Use database constraints alongside application validation

**AI suggestion:** Request validation in the API handlers can catch most invalid input.

**My decision:** I also used PostgreSQL constraints and foreign keys where appropriate, in addition to application-level validation.

**Why:** API validation improves error messages, but database constraints provide another layer of protection against invalid relationships and inconsistent data.

### Decision 3: Treat idempotency and webhook retries as explicit reliability requirements

**AI suggestion:** A basic payment endpoint and direct webhook call could be sufficient for a minimal demonstration.

**My decision:** I treated idempotency and reliable webhook delivery as separate design concerns in the implementation plan.

**Why:** Payment requests may be repeated because of client retries or network failures. Idempotency helps prevent duplicate processing, while retryable webhook delivery helps handle temporary failures when notifying external systems.

## 3. One Thing AI Got Wrong or That I Had to Correct

AI-generated implementation suggestions can be incomplete when they do not account for the project's actual schema, dependencies, or existing code.

During development, I treated suggested code and database designs as starting points rather than assuming they were correct. I checked them against the existing project structure, PostgreSQL schema, Rust types, and assignment requirements before incorporating them.

I also distinguish between features that are implemented and tested and features that are only designed or still pending. I do not consider a feature complete merely because AI suggested an implementation for it.

## 4. Verification and Responsibility

I am responsible for the final code and design decisions. I verify changes by reviewing the implementation, checking database relationships and API behavior, and running the relevant build or tests where available.

AI accelerated research and routine implementation work, but correctness, integration, and the final engineering trade-offs remain my responsibility.
