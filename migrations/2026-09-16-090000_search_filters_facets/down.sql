-- Back to the version without indexed chronological ordering nor phonetic correction

DROP INDEX IF EXISTS search_lexicon_etablissement_phonetic_idx;
DROP INDEX IF EXISTS search_lexicon_unite_legale_phonetic_idx;
ALTER TABLE search_lexicon DROP COLUMN IF EXISTS phonetic;
DROP FUNCTION IF EXISTS public.phonetic_fr(TEXT);

DROP INDEX IF EXISTS etablissement_date_creation_idx;
DROP INDEX IF EXISTS unite_legale_date_creation_idx;

-- search_query without the phonetic fallback
CREATE OR REPLACE FUNCTION public.search_query(
  q TEXT,
  source_name TEXT,
  OUT tsq tsquery,
  OUT suggestion TEXT
) LANGUAGE sql STABLE PARALLEL SAFE
SET pg_trgm.similarity_threshold = 0.25 AS $fn$
  WITH words AS (
    SELECT ord, word
      FROM unnest(regexp_split_to_array(lower(public.immutable_unaccent(coalesce(q, ''))), '[^a-z0-9]+'))
           WITH ORDINALITY AS t(word, ord)
     WHERE length(word) >= 3
  ), frequency AS (
    SELECT w.*,
           coalesce((SELECT l.ndoc FROM search_lexicon l
                      WHERE l.source = source_name AND l.word = w.word), 0) AS ndoc
      FROM words w
  ), corrected AS (
    SELECT f.ord, f.word, fix.word AS correction
      FROM frequency f
      LEFT JOIN LATERAL (
        SELECT l.word
          FROM search_lexicon l
         WHERE l.source = source_name
           AND l.ndoc >= 5
           AND l.ndoc >= greatest(20 * f.ndoc, 50)
           AND length(f.word) >= 4
           AND length(l.word) BETWEEN length(f.word) - 2 AND length(f.word) + 2
           AND l.word <> f.word
           AND l.word % f.word
           AND levenshtein_less_equal(l.word, f.word, 2) <= 2
         ORDER BY levenshtein_less_equal(l.word, f.word, 2), l.ndoc DESC
         LIMIT 1
      ) fix ON TRUE
  )
  SELECT to_tsquery('french', string_agg(
           CASE WHEN correction IS NULL
                THEN quote_literal(word)
                ELSE '(' || quote_literal(word) || '|' || quote_literal(correction) || ')'
           END, ' & ' ORDER BY ord)),
         CASE WHEN count(*) FILTER (WHERE correction IS NOT NULL) = 0 THEN NULL
              ELSE string_agg(coalesce(correction, word), ' ' ORDER BY ord)
         END
    FROM corrected
$fn$;

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
