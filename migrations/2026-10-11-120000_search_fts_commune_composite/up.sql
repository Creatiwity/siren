-- Text search restricted to a commune, resolved inside a single index.
--
-- With `q` and `commune` together, the planner ANDs two bitmaps: the full-text
-- one (28,618 rows for "creati" -> 'creat' | 'creation') and the commune one
-- (3.9 M rows for "paris"). Building the second costs 435-460 ms in production,
-- twice per call (results, then the capped count), to keep 764 rows. Without
-- it, the same query takes 74-118 ms.
--
-- `btree_gin` lets a scalar column sit next to the tsvector in one GIN index:
-- the commune then filters the posting lists inside the index, and no bitmap
-- the size of the commune is ever built.
--
-- The full-text index stays: searches without a commune keep using it.
--
-- Plain CREATE INDEX rather than CONCURRENTLY: it only blocks writes, which come
-- from the update jobs alone, and a failure rolls back cleanly instead of
-- leaving an INVALID index behind.

CREATE EXTENSION IF NOT EXISTS btree_gin;

SET LOCAL maintenance_work_mem = '1GB';

CREATE INDEX etablissement_search_fts_commune_idx ON etablissement USING GIN (
  code_commune,
  to_tsvector('french'::regconfig, public.immutable_unaccent(
    coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' ||
    coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '')))
);

-- The staging table needs the same index, since the swap promotes it to
-- production. Created in place rather than by recreating the table: staging is
-- empty between updates, so this is instant.

CREATE INDEX etablissement_staging_search_fts_commune_idx ON etablissement_staging USING GIN (
  code_commune,
  to_tsvector('french'::regconfig, public.immutable_unaccent(
    coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' ||
    coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '')))
);
