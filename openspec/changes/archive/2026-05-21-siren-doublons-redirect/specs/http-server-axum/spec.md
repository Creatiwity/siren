## MODIFIED Requirements

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

#### Scenario: Unite Legale Endpoint — SIREN doublon connu

- **WHEN** a GET request is made to `/v3/unites_legales/<siren>` where `<siren>` is not found in `unites_legales`
- **AND** `<siren>` is present in `siren_doublons.siren_doublon`
- **THEN** the server responds with status code `301`
- **AND** the `Location` header is set to `/v3/unites_legales/<siren_canonique>`

#### Scenario: Unite Legale Endpoint — SIREN absent

- **WHEN** a GET request is made to `/v3/unites_legales/<siren>` where `<siren>` is not found in `unites_legales`
- **AND** `<siren>` is not present in `siren_doublons`
- **THEN** the server responds with status code `404`

#### Scenario: Search Unites Legales Endpoint

- **WHEN** a GET request is made to `/v3/unites_legales` with optional query parameters
- **THEN** the server responds with a paginated list of matching legal units
- **AND** the response is documented in OpenAPI via Scalar

#### Scenario: Etablissement Endpoint

- **WHEN** a GET request is made to `/v3/etablissements/<siret>` where `<siret>` is a valid 14-digit SIRET
- **THEN** the server responds with the corresponding `EtablissementResponse`
- **AND** the response includes the establishment data and associated unit legale with siege establishment

#### Scenario: Etablissement Endpoint — SIREN doublon connu

- **WHEN** a GET request is made to `/v3/etablissements/<siret>` where `<siret>` is not found in `etablissements`
- **AND** the SIREN extracted from `<siret>` (first 9 digits) is present in `siren_doublons.siren_doublon`
- **AND** the canonical SIREN has a siege establishment
- **THEN** the server responds with status code `301`
- **AND** the `Location` header is set to `/v3/etablissements/<siret_siege_canonique>`

#### Scenario: Etablissement Endpoint — SIREN absent des doublons

- **WHEN** a GET request is made to `/v3/etablissements/<siret>` where `<siret>` is not found in `etablissements`
- **AND** the SIREN extracted from `<siret>` is not present in `siren_doublons`
- **THEN** the server responds with status code `404`

#### Scenario: Search Etablissements Endpoint

- **WHEN** a GET request is made to `/v3/etablissements` with optional query parameters
- **THEN** the server responds with a paginated list of matching establishments
- **AND** the response is documented in OpenAPI via Scalar

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
