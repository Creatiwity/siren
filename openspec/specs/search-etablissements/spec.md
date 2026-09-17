# search-etablissements Specification

## Purpose
TBD - created by archiving change 2026-02-06-add-search-endpoints. Update Purpose after archive.
## Requirements
### Requirement: Search etablissements with text query

The system SHALL allow searching establishments by text query on denomination and enseigne fields via the `q` query parameter on `GET /v3/etablissements`. Commune names are NOT part of the text index; they are addressed by the dedicated `commune` parameter.

#### Scenario: Text search on denomination

- **WHEN** a GET request is made to `/v3/etablissements?q=creati`
- **THEN** the system returns establishments whose `denomination_usuelle`, `enseigne_1`, `enseigne_2` or `enseigne_3` matches the query using PostgreSQL full-text search
- **AND** each result includes a `score` field with the text relevance score

#### Scenario: Multiple words are combined with AND

- **WHEN** a GET request is made to `/v3/etablissements?q=boulangerie+du+village`
- **THEN** only establishments matching every significant term are returned

#### Scenario: Query shorter than three characters

- **WHEN** a GET request is made to `/v3/etablissements?q=le`
- **THEN** the system responds with a 400 error indicating that `q` must be at least 3 characters long

#### Scenario: No text query provided

- **WHEN** a GET request is made to `/v3/etablissements` without a `q` parameter
- **THEN** the system returns establishments without text filtering
- **AND** the `score` field is absent from results

### Requirement: Tolerate misspellings in text queries

The system SHALL tolerate misspelled query terms without losing exact matches.

#### Scenario: Misspelled term is augmented, never replaced

- **WHEN** a GET request is made to `/v3/etablissements?q=carefour`
- **THEN** the system searches for both the typed term and the corrected term
- **AND** establishments literally named `carefour` are still returned alongside `CARREFOUR`

#### Scenario: Rare but correct name is preserved

- **WHEN** a GET request is made with a rare legitimate name that resembles a more frequent word
- **THEN** the system still returns the establishments matching the typed name

#### Scenario: Suggestion on empty results

- **WHEN** a text search returns no establishment and a close term exists in the corpus
- **THEN** the response includes a `suggestion` field with the reformulated query
- **AND** the `suggestion` field is absent when the search returns at least one result

#### Scenario: Trigram fallback

- **WHEN** a text search returns no establishment through full-text search
- **THEN** the system retries once with trigram similarity before returning an empty result set

#### Scenario: Phonetic correction

- **WHEN** a GET request is made to `/v3/etablissements?q=filipe`
- **THEN** the system also searches for the phonetically equivalent corpus term `philippe`
- **AND** the phonetic source is consulted only when trigram similarity yields no candidate

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

### Requirement: Filter etablissements by multiple values

The system SHALL accept several comma-separated values on `code_postal`, `code_commune`, `siren` and `activite_principale`, and SHALL return establishments matching any of them.

#### Scenario: Several postal codes

- **WHEN** a GET request is made to `/v3/etablissements?code_postal=75001,75002`
- **THEN** only establishments whose postal code is one of the two are returned

#### Scenario: Single value stays valid

- **WHEN** a GET request is made to `/v3/etablissements?code_postal=75001`
- **THEN** the behaviour is unchanged from the single-value form

### Requirement: Exclude etablissements by field values

The system SHALL allow excluding values via `code_postal_not`, `code_commune_not` and `activite_principale_not`. Establishments whose field is null are kept: excluding a value is not a statement about unknown values.

#### Scenario: Exclude an activity

- **WHEN** a GET request is made to `/v3/etablissements?q=boulangerie&activite_principale_not=10.71C`
- **THEN** no returned establishment has `activite_principale = '10.71C'`

#### Scenario: Null values survive exclusion

- **WHEN** an exclusion filter is applied
- **THEN** establishments with no value for that field are still returned

### Requirement: Filter etablissements by date range

The system SHALL allow bounding `date_creation` and `date_debut` via `date_creation_min`, `date_creation_max`, `date_debut_min` and `date_debut_max`. Both bounds are inclusive and each is optional.

#### Scenario: Bounded range

- **WHEN** a GET request is made to `/v3/etablissements?date_creation_min=2024-01-01&date_creation_max=2024-12-31`
- **THEN** only establishments created within that range are returned

#### Scenario: Open-ended range

- **WHEN** a GET request is made to `/v3/etablissements?date_creation_min=2024-01-01`
- **THEN** only establishments created on or after that date are returned

### Requirement: Facet etablissement search results

The system SHALL compute value counts for the fields listed in the `facette` query parameter, over the same capped subset used for `total`. Allowed fields are `etat_administratif`, `code_postal`, `code_commune`, `activite_principale` and `etablissement_siege`.

#### Scenario: Request facets

- **WHEN** a GET request is made to `/v3/etablissements?q=boulangerie&facette=activite_principale,code_commune`
- **THEN** the response contains a `facettes` object with one entry per requested field
- **AND** each entry lists values sorted by descending count

#### Scenario: Facets are absent when not requested

- **WHEN** a GET request is made without `facette`
- **THEN** the response contains no `facettes` field

#### Scenario: Unknown facet field

- **WHEN** a GET request is made to `/v3/etablissements?facette=siret`
- **THEN** the system responds with a 400 error listing the allowed fields

### Requirement: Filter etablissements by commune name

The system SHALL allow filtering establishments by plain-text commune name via the `commune` query parameter. The name is resolved against a commune dimension, accent- and case-insensitively, matching each typed word as a prefix of any word of the commune name, with a fallback tolerant to misspellings.

#### Scenario: Filter by commune name

- **WHEN** a GET request is made to `/v3/etablissements?commune=paris`
- **THEN** establishments located in a commune having a word starting with `paris` are returned — every Paris arrondissement, and also `LE TOUQUET-PARIS-PLAGE`

#### Scenario: Match on any word of the commune name

- **WHEN** a GET request is made to `/v3/etablissements?commune=etienne`
- **THEN** establishments in `SAINT-ETIENNE` and other communes containing that word are returned

#### Scenario: Misspelled commune name

- **WHEN** a GET request is made to `/v3/etablissements?commune=marseile`
- **THEN** the system falls back to trigram similarity and returns establishments in `MARSEILLE`

#### Scenario: Unknown commune name

- **WHEN** a GET request is made to `/v3/etablissements?commune=zzzzzz`
- **THEN** the response contains an empty `etablissements` array and `total` is 0

#### Scenario: Combine commune with a text query

- **WHEN** a GET request is made to `/v3/etablissements?commune=paris&q=boulangerie`
- **THEN** only establishments matching both the commune and the text query are returned

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
- **THEN** results are sorted by full-text relevance score descending (most relevant first)

#### Scenario: Sort by relevance ascending

- **WHEN** a GET request is made to `/v3/etablissements?q=creati&sort=relevance&direction=asc`
- **THEN** results are sorted by full-text relevance score ascending (least relevant first)

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
- **AND** the response includes `suggestion` only when the result set is empty and a close term exists
- **AND** the response includes `total_capped`, `true` when `total` reached its ceiling

#### Scenario: Empty search results

- **WHEN** a search query matches no establishments
- **THEN** the response body contains an empty `etablissements` array
- **AND** `total` is 0