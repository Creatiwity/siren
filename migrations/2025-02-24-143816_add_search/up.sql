CREATE EXTENSION IF NOT EXISTS "pg_search";
CREATE EXTENSION IF NOT EXISTS "postgis";

ALTER TABLE etablissement ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' || coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '')) STORED;

ALTER TABLE etablissement ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8), 2154), 4326)::geography) END) STORED;

CREATE INDEX search_etablissement_idx ON etablissement
USING bm25 (siret, siren, date_creation, date_debut, code_postal, libelle_commune, search_denomination)
WITH (
	key_field='siret',
	text_fields='{
		"code_postal": {
          "fast": true
        },
		"libelle_commune": {
          "fast": true,
          "tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 5, "prefix_only": false}
        },
        "search_denomination": {
          "fast": true,
          "tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 5, "prefix_only": false}
        }
    }'
);

CREATE INDEX etablissement_position_index
  ON etablissement
  USING GIST (position);

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

ALTER TABLE unite_legale ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' || coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, '')) STORED;

CREATE INDEX search_unite_legale_idx ON unite_legale
USING bm25 (siren, date_creation, date_debut, search_denomination)
WITH (
	key_field='siren',
	text_fields='{
        "search_denomination": {
          "fast": true,
          "tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 5, "prefix_only": false}
        }
    }'
);

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
