-- Migration manuelle : de pg_search (ParadeDB BM25) vers pg_trgm
--
-- A executer sur les instances ayant deja applique la migration
-- 2026-02-06-143816_add_search avec pg_search.
--
-- Les nouvelles installations n'ont pas besoin de ce script :
-- la migration Diesel utilise directement pg_trgm.

-- 1. Supprimer les anciens index BM25 ParadeDB
DROP INDEX IF EXISTS search_etablissement_idx;
DROP INDEX IF EXISTS search_unite_legale_idx;

-- 2. Activer les nouvelles extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

-- 3. Creer les nouveaux index GIN trigramme
CREATE INDEX IF NOT EXISTS etablissement_search_denomination_trgm_idx
    ON etablissement USING GIN (search_denomination gin_trgm_ops);
CREATE INDEX IF NOT EXISTS etablissement_libelle_commune_trgm_idx
    ON etablissement USING GIN (libelle_commune gin_trgm_ops);
CREATE INDEX IF NOT EXISTS unite_legale_search_denomination_trgm_idx
    ON unite_legale USING GIN (search_denomination gin_trgm_ops);

-- 4. Recreer les tables de staging pour qu'elles heritent des nouveaux index GIN
--    (LIKE ... INCLUDING INDEXES capture les index au moment de la creation)
DROP TABLE IF EXISTS "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

DROP TABLE IF EXISTS "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

-- 5. Supprimer l'extension pg_search (necessite d'etre superuser ou owner)
DROP EXTENSION IF EXISTS pg_search;
