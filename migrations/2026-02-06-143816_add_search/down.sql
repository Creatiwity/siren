DROP INDEX etablissement_search_denomination_trgm_idx;
DROP INDEX etablissement_search_denom_a_trgm_idx;
DROP INDEX etablissement_search_denom_f_trgm_idx;
DROP INDEX etablissement_position_index;
DROP INDEX etablissement_filter_idx;

ALTER TABLE etablissement DROP COLUMN search_denomination;
ALTER TABLE etablissement DROP COLUMN position;

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES);

DROP INDEX unite_legale_search_denomination_trgm_idx;
DROP INDEX unite_legale_search_denom_a_trgm_idx;
DROP INDEX unite_legale_search_denom_f_trgm_idx;
DROP INDEX unite_legale_filter_idx;

ALTER TABLE unite_legale DROP COLUMN search_denomination;

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES);

DROP FUNCTION IF EXISTS immutable_unaccent(text);
DROP EXTENSION IF EXISTS "unaccent";
DROP EXTENSION IF EXISTS "pg_trgm";
DROP EXTENSION IF EXISTS "postgis";
