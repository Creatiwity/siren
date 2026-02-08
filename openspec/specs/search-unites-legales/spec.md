# search-unites-legales Specification

## Purpose
TBD - created by archiving change 2026-02-06-add-search-endpoints. Update Purpose after archive.
## Requirements
### Requirement: Search unites legales with text query

The system SHALL allow searching legal units by text query on denomination via the `q` query parameter on `GET /v3/unites_legales`.

#### Scenario: Text search on denomination

- **WHEN** a GET request is made to `/v3/unites_legales?q=creati`
- **THEN** the system returns legal units whose `search_denomination` matches the query using BM25 ngram search
- **AND** each result includes a `score` field with the text relevance score

#### Scenario: No text query provided

- **WHEN** a GET request is made to `/v3/unites_legales` without a `q` parameter
- **THEN** the system returns legal units without text filtering
- **AND** the `score` field is absent from results

### Requirement: Filter unites legales by field values

The system SHALL allow filtering legal units by exact field values via query parameters.

#### Scenario: Filter by etat_administratif

- **WHEN** a GET request is made to `/v3/unites_legales?etat_administratif=A`
- **THEN** only legal units with `etat_administratif = 'A'` are returned

#### Scenario: Filter by activite_principale

- **WHEN** a GET request is made to `/v3/unites_legales?activite_principale=62.01Z`
- **THEN** only legal units with `activite_principale = '62.01Z'` are returned

#### Scenario: Filter by categorie_juridique

- **WHEN** a GET request is made to `/v3/unites_legales?categorie_juridique=5710`
- **THEN** only legal units with `categorie_juridique = '5710'` are returned

#### Scenario: Filter by categorie_entreprise

- **WHEN** a GET request is made to `/v3/unites_legales?categorie_entreprise=PME`
- **THEN** only legal units with `categorie_entreprise = 'PME'` are returned

#### Scenario: Filter by date_creation

- **WHEN** a GET request is made to `/v3/unites_legales?date_creation=2024-01-15`
- **THEN** only legal units with `date_creation = '2024-01-15'` are returned

#### Scenario: Filter by date_debut

- **WHEN** a GET request is made to `/v3/unites_legales?date_debut=2024-01-15`
- **THEN** only legal units with `date_debut = '2024-01-15'` are returned

#### Scenario: Combine multiple filters

- **WHEN** a GET request is made to `/v3/unites_legales?etat_administratif=A&activite_principale=62.01Z&q=creati`
- **THEN** the system applies all filters together (AND logic)
- **AND** only legal units matching all criteria are returned

### Requirement: Sort unite legale search results

The system SHALL allow sorting search results via `sort` and `direction` query parameters. The `sort` parameter specifies the field to sort by, and the `direction` parameter specifies the sort direction (`asc` or `desc`). When direction is omitted, sensible defaults apply per field.

#### Scenario: Sort by relevance

- **WHEN** a GET request is made to `/v3/unites_legales?q=creati&sort=relevance`
- **THEN** results are sorted by BM25 text relevance score descending (most relevant first)

#### Scenario: Sort by relevance ascending

- **WHEN** a GET request is made to `/v3/unites_legales?q=creati&sort=relevance&direction=asc`
- **THEN** results are sorted by BM25 text relevance score ascending (least relevant first)

#### Scenario: Sort by date_creation

- **WHEN** a GET request is made to `/v3/unites_legales?sort=date_creation`
- **THEN** results are sorted by `date_creation` descending (newest first)

#### Scenario: Sort by date_creation ascending

- **WHEN** a GET request is made to `/v3/unites_legales?sort=date_creation&direction=asc`
- **THEN** results are sorted by `date_creation` ascending (oldest first)

#### Scenario: Sort by date_debut

- **WHEN** a GET request is made to `/v3/unites_legales?sort=date_debut`
- **THEN** results are sorted by `date_debut` descending (newest first)

#### Scenario: Sort by date_debut ascending

- **WHEN** a GET request is made to `/v3/unites_legales?sort=date_debut&direction=asc`
- **THEN** results are sorted by `date_debut` ascending (oldest first)

#### Scenario: Default sort with text search

- **WHEN** a GET request is made to `/v3/unites_legales?q=creati` without a `sort` parameter
- **THEN** results are sorted by relevance descending

#### Scenario: Default sort without text search

- **WHEN** a GET request is made to `/v3/unites_legales` without a `sort` or `q` parameter
- **THEN** results are sorted by `date_creation` descending

#### Scenario: Invalid sort with relevance

- **WHEN** a GET request is made to `/v3/unites_legales?sort=relevance` without a `q` parameter
- **THEN** the system responds with a 400 error

### Requirement: Paginate unite legale search results

The system SHALL support pagination via `limit` and `offset` query parameters.

#### Scenario: Default pagination

- **WHEN** a GET request is made to `/v3/unites_legales` without `limit` or `offset`
- **THEN** the system returns at most 20 results starting from offset 0

#### Scenario: Custom limit and offset

- **WHEN** a GET request is made to `/v3/unites_legales?limit=50&offset=100`
- **THEN** the system returns at most 50 results starting from offset 100

#### Scenario: Maximum limit

- **WHEN** a GET request is made to `/v3/unites_legales?limit=200`
- **THEN** the system caps the limit to 100 and returns at most 100 results

#### Scenario: Maximum offset

- **WHEN** a GET request is made to `/v3/unites_legales?offset=20000`
- **THEN** the system caps the offset to 10000

### Requirement: Search response format for unites legales

The system SHALL return search results in a structured response with metadata.

#### Scenario: Successful search response

- **WHEN** a search query returns results
- **THEN** the response body contains an `unites_legales` array with each item containing at minimum: `siren`, `etat_administratif`, `date_creation`, `denomination`, `denomination_usuelle_1`, `denomination_usuelle_2`, `denomination_usuelle_3`, `activite_principale`, `categorie_juridique`, `categorie_entreprise`
- **AND** the response includes `total` with the total count of matching results
- **AND** the response includes `limit` and `offset` reflecting the applied pagination
- **AND** the response includes `sort` and `direction` reflecting the resolved sort field and direction

#### Scenario: Empty search results

- **WHEN** a search query matches no legal units
- **THEN** the response body contains an empty `unites_legales` array
- **AND** `total` is 0