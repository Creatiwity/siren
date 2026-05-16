CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "unaccent";
CREATE EXTENSION IF NOT EXISTS "postgis";

CREATE OR REPLACE FUNCTION immutable_unaccent(text)
  RETURNS text LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT AS
$$SELECT unaccent('unaccent', $1)$$;

ALTER TABLE etablissement ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (lower(immutable_unaccent(coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' || coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '') || ' ' || coalesce(libelle_commune, '')))) STORED;

ALTER TABLE etablissement ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8), 2154), 4326)::geography) END) STORED;

CREATE INDEX etablissement_search_denomination_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops);
CREATE INDEX etablissement_search_denom_a_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'A';
CREATE INDEX etablissement_search_denom_f_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'F';

CREATE INDEX etablissement_position_index
  ON etablissement
  USING GIST (position);

CREATE INDEX etablissement_filter_idx ON etablissement (etat_administratif, etablissement_siege, code_postal, code_commune, activite_principale, date_creation NULLS LAST, date_debut NULLS LAST);

CREATE INDEX unite_legale_filter_idx ON unite_legale (etat_administratif, activite_principale, categorie_juridique, categorie_entreprise, date_creation NULLS LAST, date_debut NULLS LAST);

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

ALTER TABLE unite_legale ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (lower(immutable_unaccent(coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' || coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, '')))) STORED;

CREATE INDEX unite_legale_search_denomination_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops);
CREATE INDEX unite_legale_search_denom_a_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'A';
CREATE INDEX unite_legale_search_denom_f_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'F';

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
