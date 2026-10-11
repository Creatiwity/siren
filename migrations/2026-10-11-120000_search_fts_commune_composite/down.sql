DROP INDEX IF EXISTS etablissement_staging_search_fts_commune_idx;
DROP INDEX IF EXISTS etablissement_search_fts_commune_idx;

-- btree_gin is left installed: other objects may depend on it.
