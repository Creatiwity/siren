## ADDED Requirements

### Requirement: Search etablissements with text query

The system SHALL allow searching establishments by text query on denomination and commune name via the `q` query parameter on `GET /v3/etablissements`.

#### Scenario: Text search on denomination

- **WHEN** a GET request is made to `/v3/etablissements?q=creati`
- **THEN** the system returns establishments whose `search_denomination` matches the query using BM25 ngram search
- **AND** each result includes a `score` field with the text relevance score

#### Scenario: Text search on commune name

- **WHEN** a GET request is made to `/v3/etablissements?q=paris`
- **THEN** the system also matches against `libelle_commune` using BM25 ngram search

#### Scenario: No text query provided

- **WHEN** a GET request is made to `/v3/etablissements` without a `q` parameter
- **THEN** the system returns establishments without text filtering
- **AND** the `score` field is absent from results

### Requirement: Filter etablissements by field values

The system SHALL allow filtering establishments by exact field values via query parameters.

#### Scenario: Filter by etat_administratif

- **WHEN** a GET request is made to `/v3/etablissements?etat_administratif=A`
- **THEN** only establishments with `etat_administratif = 'A'` are returned

#### Scenario: Filter by code_postal

- **WHEN** a GET request is made to `/v3/etablissements?code_postal=75001`
- **THEN** only establishments with `code_postal = '75001'` are returned

#### Scenario: Filter by siren

- **WHEN** a GET request is made to `/v3/etablissements?siren=123456789`
- **THEN** only establishments with `siren = '123456789'` are returned

#### Scenario: Filter by code_commune

- **WHEN** a GET request is made to `/v3/etablissements?code_commune=75101`
- **THEN** only establishments with `code_commune = '75101'` are returned

#### Scenario: Filter by activite_principale

- **WHEN** a GET request is made to `/v3/etablissements?activite_principale=62.01Z`
- **THEN** only establishments with `activite_principale = '62.01Z'` are returned

#### Scenario: Filter by etablissement_siege

- **WHEN** a GET request is made to `/v3/etablissements?etablissement_siege=true`
- **THEN** only establishments where `etablissement_siege` is true are returned

#### Scenario: Combine multiple filters

- **WHEN** a GET request is made to `/v3/etablissements?etat_administratif=A&code_postal=75001&q=creati`
- **THEN** the system applies all filters together (AND logic)
- **AND** only establishments matching all criteria are returned

### Requirement: Geographic search on etablissements

The system SHALL allow filtering establishments within a geographic radius from a reference point.

#### Scenario: Search within radius

- **WHEN** a GET request is made to `/v3/etablissements?lat=48.8566&lng=2.3522&radius=1000`
- **THEN** only establishments within 1000 meters of the reference point (48.8566, 2.3522) are returned
- **AND** each result includes a `meter_distance` field with the distance in meters from the reference point

#### Scenario: Missing geographic parameters

- **WHEN** a GET request is made with `lat` but without `lng` or `radius`
- **THEN** the system responds with a 400 error indicating the missing parameters

#### Scenario: No geographic parameters

- **WHEN** a GET request is made without `lat`, `lng`, and `radius`
- **THEN** the system returns results without geographic filtering
- **AND** the `meter_distance` field is absent from results

### Requirement: Sort etablissement search results

The system SHALL allow sorting search results via `sort` and `direction` query parameters. The `sort` parameter specifies the field to sort by, and the `direction` parameter specifies the sort direction (`asc` or `desc`). When direction is omitted, sensible defaults apply per field.

#### Scenario: Sort by distance

- **WHEN** a GET request is made to `/v3/etablissements?lat=48.8566&lng=2.3522&radius=5000&sort=distance`
- **THEN** results are sorted by geographic distance ascending (nearest first)

#### Scenario: Sort by distance descending

- **WHEN** a GET request is made to `/v3/etablissements?lat=48.8566&lng=2.3522&radius=5000&sort=distance&direction=desc`
- **THEN** results are sorted by geographic distance descending (farthest first)

#### Scenario: Sort by relevance

- **WHEN** a GET request is made to `/v3/etablissements?q=creati&sort=relevance`
- **THEN** results are sorted by BM25 text relevance score descending (most relevant first)

#### Scenario: Sort by relevance ascending

- **WHEN** a GET request is made to `/v3/etablissements?q=creati&sort=relevance&direction=asc`
- **THEN** results are sorted by BM25 text relevance score ascending (least relevant first)

#### Scenario: Sort by date_creation

- **WHEN** a GET request is made to `/v3/etablissements?sort=date_creation`
- **THEN** results are sorted by `date_creation` descending (newest first)

#### Scenario: Sort by date_creation ascending

- **WHEN** a GET request is made to `/v3/etablissements?sort=date_creation&direction=asc`
- **THEN** results are sorted by `date_creation` ascending (oldest first)

#### Scenario: Sort by date_debut

- **WHEN** a GET request is made to `/v3/etablissements?sort=date_debut`
- **THEN** results are sorted by `date_debut` descending (newest first)

#### Scenario: Sort by date_debut ascending

- **WHEN** a GET request is made to `/v3/etablissements?sort=date_debut&direction=asc`
- **THEN** results are sorted by `date_debut` ascending (oldest first)

#### Scenario: Default sort with text search

- **WHEN** a GET request is made to `/v3/etablissements?q=creati` without a `sort` parameter
- **THEN** results are sorted by relevance descending

#### Scenario: Default sort without text search

- **WHEN** a GET request is made to `/v3/etablissements` without a `sort` or `q` parameter
- **THEN** results are sorted by `date_creation` descending

#### Scenario: Invalid sort with distance

- **WHEN** a GET request is made to `/v3/etablissements?sort=distance` without geographic parameters
- **THEN** the system responds with a 400 error

#### Scenario: Invalid sort with relevance

- **WHEN** a GET request is made to `/v3/etablissements?sort=relevance` without a `q` parameter
- **THEN** the system responds with a 400 error

### Requirement: Paginate etablissement search results

The system SHALL support pagination via `limit` and `offset` query parameters.

#### Scenario: Default pagination

- **WHEN** a GET request is made to `/v3/etablissements` without `limit` or `offset`
- **THEN** the system returns at most 20 results starting from offset 0

#### Scenario: Custom limit and offset

- **WHEN** a GET request is made to `/v3/etablissements?limit=50&offset=100`
- **THEN** the system returns at most 50 results starting from offset 100

#### Scenario: Maximum limit

- **WHEN** a GET request is made to `/v3/etablissements?limit=200`
- **THEN** the system caps the limit to 100 and returns at most 100 results

#### Scenario: Maximum offset

- **WHEN** a GET request is made to `/v3/etablissements?offset=20000`
- **THEN** the system caps the offset to 10000

### Requirement: Search response format for etablissements

The system SHALL return search results in a structured response with metadata.

#### Scenario: Successful search response

- **WHEN** a search query returns results
- **THEN** the response body contains an `etablissements` array with each item containing at minimum: `siret`, `siren`, `etat_administratif`, `date_creation`, `denomination_usuelle`, `enseigne_1`, `enseigne_2`, `enseigne_3`, `code_postal`, `libelle_commune`, `activite_principale`, `etablissement_siege`
- **AND** the response includes `total` with the total count of matching results
- **AND** the response includes `limit` and `offset` reflecting the applied pagination
- **AND** the response includes `sort` and `direction` reflecting the resolved sort field and direction

#### Scenario: Empty search results

- **WHEN** a search query matches no establishments
- **THEN** the response body contains an empty `etablissements` array
- **AND** `total` is 0
