CREATE EXTENSION IF NOT EXISTS "pg_search";
CREATE EXTENSION IF NOT EXISTS "postgis";

ALTER TABLE etablissement ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' || coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '')) STORED;

ALTER TABLE etablissement ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8), 2154), 4326)::geography) END) STORED;

CREATE INDEX search_etablissement_idx ON etablissement
USING bm25 (siret, siren, date_debut, code_postal, (libelle_commune::pdb.ngram(4,5)), (search_denomination::pdb.ngram(4,5)))
WITH (key_field='siret');

CREATE INDEX etablissement_position_index
  ON etablissement
  USING GIST (position);

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

ALTER TABLE unite_legale ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' || coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, '')) STORED;

CREATE INDEX search_unite_legale_idx ON unite_legale
USING bm25 (siren, date_creation, date_debut, (search_denomination::pdb.ngram(4,5)))
WITH (key_field='siren');

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
