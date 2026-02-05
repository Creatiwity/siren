# Spec: Sentry Integration with Axum

## ADDED Requirements

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

## Configuration

### Requirement: Sentry Configuration

The Sentry integration must be configurable via environment variables.

#### Scenario: Sentry DSN

- **WHEN** the `SENTRY_DSN` environment variable is set
- **THEN** the server initializes Sentry with the provided DSN

#### Scenario: Sentry Environment

- **WHEN** the `SIRENE_ENV` environment variable is set
- **THEN** the server configures Sentry with the corresponding environment

#### Scenario: Sentry Release

- **WHEN** the server starts
- **THEN** the Sentry release is automatically set to the current version

## Middleware

### Requirement: Sentry Middleware

The server must use Sentry middleware for automatic error tracking.

#### Scenario: Middleware Integration

- **WHEN** a request is processed by the server
- **THEN** the Sentry middleware captures the request context
- **AND** the context is included in any error reports

#### Scenario: Error Handling

- **WHEN** an error occurs in a handler
- **THEN** the Sentry middleware captures the error
- **AND** the error is reported to Sentry with full context
