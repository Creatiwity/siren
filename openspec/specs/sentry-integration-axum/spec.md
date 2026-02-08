# sentry-integration-axum Specification

## Purpose
TBD - created by archiving change migrate-from-warp-to-axum. Update Purpose after archive.
## Requirements
### Requirement: Sentry Error Tracking

The server SHALL integrate Sentry for error tracking and monitoring using Axum's built-in support.

#### Scenario: Error Reporting

- **WHEN** an error occurs in any endpoint handler
- **THEN** the error is automatically reported to Sentry
- **AND** the error includes relevant context (e.g., request details, error type)

#### Scenario: Performance Monitoring

- **WHEN** a request is processed by the server
- **THEN** the request is tracked as a transaction in Sentry
- **AND** the transaction includes timing and performance data

#### Scenario: Breadcrumbs

- **WHEN** a request is processed by the server
- **THEN** breadcrumbs are automatically added to Sentry for debugging context
- **AND** breadcrumbs include request details (e.g., method, path, headers)

