-- Retour a la recherche trigramme sur search_denomination (commune incluse)

DROP FUNCTION IF EXISTS public.search_refresh_incremental(TEXT, TIMESTAMP);
DROP FUNCTION IF EXISTS public.search_refresh_full(TEXT);
DROP FUNCTION IF EXISTS public.search_query(TEXT, TEXT);
DROP FUNCTION IF EXISTS public.commune_codes(TEXT);

DROP TABLE IF EXISTS search_lexicon;
DROP TABLE IF EXISTS commune_dim;

DROP INDEX IF EXISTS etablissement_search_fts_idx;
DROP INDEX IF EXISTS etablissement_search_trgm_idx;
DROP INDEX IF EXISTS etablissement_commune_date_idx;
DROP INDEX IF EXISTS unite_legale_search_fts_idx;
DROP INDEX IF EXISTS unite_legale_search_trgm_idx;

ALTER TABLE etablissement ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (lower(immutable_unaccent(coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' || coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '') || ' ' || coalesce(libelle_commune, '')))) STORED;

CREATE INDEX etablissement_search_denomination_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops);
CREATE INDEX etablissement_search_denom_a_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'A';
CREATE INDEX etablissement_search_denom_f_trgm_idx ON etablissement USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'F';

ALTER TABLE unite_legale ADD COLUMN search_denomination TEXT GENERATED ALWAYS AS (lower(immutable_unaccent(coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' || coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, '')))) STORED;

CREATE INDEX unite_legale_search_denomination_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops);
CREATE INDEX unite_legale_search_denom_a_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'A';
CREATE INDEX unite_legale_search_denom_f_trgm_idx ON unite_legale USING GIN (search_denomination gin_trgm_ops) WHERE etat_administratif = 'F';

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
