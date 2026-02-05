# Spec: Scalar API Documentation

## ADDED Requirements

### Requirement: Scalar API Documentation

The SIREN API SHALL provide an interactive and well-documented reference using Scalar.

#### Scenario: Access Scalar API Documentation

- **WHEN** a GET request is made to `/scalar`
- **THEN** the server responds with the Scalar API documentation interface
- **AND** the response includes documentation for all existing endpoints

#### Scenario: Scalar API Documentation Content

- **WHEN** the Scalar API documentation is accessed
- **THEN** it includes documentation for the following endpoints:
  - `GET /`
  - `GET /v3`
  - `GET /v3/unites_legales/<siren>`
  - `GET /v3/etablissements/<siret>`
  - `POST /admin/update`
  - `GET /admin/update/status`
  - `POST /admin/update/status/error`

#### Scenario: Scalar API Documentation Interface

- **WHEN** the Scalar API documentation is accessed
- **THEN** it provides an interactive interface for exploring the API
- **AND** the interface includes search functionality
- **AND** the interface includes example requests and responses

## Configuration

### Requirement: Scalar Configuration

The Scalar API documentation SHALL be configurable via environment variables or configuration files.

#### Scenario: Scalar Configuration Options

- **WHEN** the Scalar API documentation is configured
- **THEN** it supports the following configuration options:
  - Custom title and description
  - Custom theme and branding
  - Custom OpenAPI specification

## Integration

### Requirement: Scalar Integration with Axum

The Scalar API documentation SHALL be integrated with the Axum HTTP server.

#### Scenario: Scalar Integration

- **WHEN** the Scalar API documentation is integrated with Axum
- **THEN** it uses the existing Axum router and middleware
- **AND** it is served on the same port as the main API