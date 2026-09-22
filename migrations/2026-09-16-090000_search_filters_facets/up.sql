-- Indexed chronological ordering, and phonetic correction.
--
-- Like the previous search migration, this one does not rewrite the large
-- tables: index DDL only, plus a generated column on the small search_lexicon.

-- No index started with date_creation, so the default ordering and any sorted
-- date range degenerated into a full scan: 10,885 ms down to 0.68 ms on a
-- sorted year, 3,899 ms down to 0.80 ms on the default listing. 287 MB.

CREATE INDEX etablissement_date_creation_idx
  ON etablissement (date_creation DESC NULLS LAST);

CREATE INDEX unite_legale_date_creation_idx
  ON unite_legale (date_creation DESC NULLS LAST);

-- Trigram plus Levenshtein catches typing mistakes but misses hearing ones:
-- "filipe" never led to "philippe".
--
-- `metaphone` covers most of it, but keeps the leading h, so "herbusse" and
-- "erbusse" do not meet. Hence stripping it, silent in French. Length 8 rather
-- than the short default: on 137k target words a longer code cuts collisions
-- without losing useful grouping.
--
-- Everything is schema-qualified: since PostgreSQL 17 maintenance commands
-- force search_path = pg_catalog, pg_temp.

CREATE OR REPLACE FUNCTION public.phonetic_fr(word TEXT)
  RETURNS TEXT LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT AS
$$SELECT public.metaphone(regexp_replace(lower(public.immutable_unaccent(word)), '^h', ''), 8)$$;

ALTER TABLE search_lexicon
  ADD COLUMN phonetic TEXT GENERATED ALWAYS AS (public.phonetic_fr(word)) STORED;

CREATE INDEX search_lexicon_etablissement_phonetic_idx
  ON search_lexicon (phonetic) WHERE source = 'etablissement' AND ndoc >= 5;
CREATE INDEX search_lexicon_unite_legale_phonetic_idx
  ON search_lexicon (phonetic) WHERE source = 'unite_legale' AND ndoc >= 5;

-- Candidate sources, from safest to most permissive: trigram plus Levenshtein
-- for typing mistakes, then same phonetic code for hearing ones. The second is
-- consulted only when the first yields nothing, so existing quality cannot
-- degrade; it tolerates a wider edit distance since "filipe" is 3 away from
-- "philippe".
--
-- The principle is unchanged: the typed word is never replaced, only completed
-- with an alternative.

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
    SELECT f.ord, f.word, coalesce(typo.word, sound.word) AS correction
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
      ) typo ON TRUE
      LEFT JOIN LATERAL (
        SELECT l.word
          FROM search_lexicon l
         WHERE typo.word IS NULL
           AND l.source = source_name
           AND l.ndoc >= 5
           AND l.ndoc >= greatest(20 * f.ndoc, 50)
           AND length(f.word) >= 4
           AND l.phonetic = public.phonetic_fr(f.word)
           AND l.word <> f.word
           AND levenshtein_less_equal(l.word, f.word, 4) <= 4
         ORDER BY levenshtein_less_equal(l.word, f.word, 4), l.ndoc DESC
         LIMIT 1
      ) sound ON TRUE
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

-- Staging tables must inherit the new indexes.

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

ANALYZE search_lexicon;
