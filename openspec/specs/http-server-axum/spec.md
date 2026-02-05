# http-server-axum Specification

## Purpose
TBD - created by archiving change migrate-from-warp-to-axum. Update Purpose after archive.
## Requirements
### Requirement: HTTP Server with Axum

The HTTP server SHALL be migrated from Warp to Axum while maintaining all existing endpoints and functionality.

#### Scenario: Health Check Endpoint

- **WHEN** a GET request is made to `/`
- **THEN** the server responds with `"OK"` and status code `200`

#### Scenario: Index Endpoint

- **WHEN** a GET request is made to `/v3`
- **THEN** the server responds with metadata about the last successful update
- **AND** the response includes `launched_timestamp` and `finished_timestamp`

#### Scenario: Unite Legale Endpoint

- **WHEN** a GET request is made to `/v3/unites_legales/<siren>` where `<siren>` is a valid 9-digit SIREN
- **THEN** the server responds with the corresponding `UniteLegaleResponse`
- **AND** the response includes the unit legale data, associated establishments, and siege establishment

#### Scenario: Etablissement Endpoint

- **WHEN** a GET request is made to `/v3/etablissements/<siret>` where `<siret>` is a valid 14-digit SIRET
- **THEN** the server responds with the corresponding `EtablissementResponse`
- **AND** the response includes the establishment data and associated unit legale with siege establishment

#### Scenario: Admin Update Endpoint

- **WHEN** a POST request is made to `/admin/update` with valid API key and body
- **THEN** the server initiates the update process
- **AND** responds with the appropriate status and update metadata

#### Scenario: Admin Status Endpoint

- **WHEN** a GET request is made to `/admin/update/status` with valid API key
- **THEN** the server responds with the current update status

#### Scenario: Admin Status Error Endpoint

- **WHEN** a POST request is made to `/admin/update/status/error` with valid API key
- **THEN** the server sets the current update status to error

