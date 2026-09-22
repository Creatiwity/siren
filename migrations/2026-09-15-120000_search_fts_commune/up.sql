-- ============================================================================
-- Recherche v2 : FTS natif + correction par lexique, commune en champ dedie
--
-- Aucune reecriture de table : la migration ne fait que du DDL d'index et cree
-- des tables annexes. `search_denomination` est supprimee (operation de
-- catalogue, instantanee) et remplacee par des index d'expression.
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE EXTENSION IF NOT EXISTS fuzzystrmatch;

-- ----------------------------------------------------------------------------
-- 1. immutable_unaccent : schema-qualifiee
--
-- Depuis PostgreSQL 17, les commandes de maintenance (CREATE INDEX, REINDEX,
-- VACUUM, ANALYZE, CLUSTER, REFRESH MATERIALIZED VIEW) forcent
-- search_path = pg_catalog, pg_temp. L'ancien corps `unaccent('unaccent', $1)`
-- ne resolvait plus et faisait echouer tout index fonctionnel.
-- La version qualifiee reste inlinable, donc sans surcout au plan.
-- ----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION public.immutable_unaccent(text)
  RETURNS text LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT AS
$$SELECT public.unaccent('public.unaccent'::regdictionary, $1)$$;

-- ----------------------------------------------------------------------------
-- 2. etablissement : index de recherche sur le NOM SEUL
--
-- libelle_commune sort du texte indexe : il est desormais adresse par le
-- parametre `commune` via la table de dimension commune_dim. Fusionne au nom,
-- il rendait `q=paris` equivalent a « tous les etablissements parisiens »,
-- soit 3,7 M de lignes a scorer et trier.
-- ----------------------------------------------------------------------------

DROP INDEX IF EXISTS etablissement_search_denomination_trgm_idx;
DROP INDEX IF EXISTS etablissement_search_denom_a_trgm_idx;
DROP INDEX IF EXISTS etablissement_search_denom_f_trgm_idx;
ALTER TABLE etablissement DROP COLUMN IF EXISTS search_denomination;

-- Index principal : FTS natif PostgreSQL (aucune extension requise)
CREATE INDEX etablissement_search_fts_idx ON etablissement USING GIN (
  to_tsvector('french'::regconfig, public.immutable_unaccent(
    coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' ||
    coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, '')))
);

-- Index de repli : trigrammes, utilise uniquement quand le FTS ne ramene rien
CREATE INDEX etablissement_search_trgm_idx ON etablissement USING GIN (
  lower(public.immutable_unaccent(
    coalesce(denomination_usuelle, '') || ' ' || coalesce(enseigne_1, '') || ' ' ||
    coalesce(enseigne_2, '') || ' ' || coalesce(enseigne_3, ''))) gin_trgm_ops
);

-- Filtre commune + tri chronologique en un seul parcours ordonne
CREATE INDEX etablissement_commune_date_idx
  ON etablissement (code_commune, date_creation DESC NULLS LAST);

-- ----------------------------------------------------------------------------
-- 3. unite_legale : memes index (pas de commune sur cette table)
-- ----------------------------------------------------------------------------

DROP INDEX IF EXISTS unite_legale_search_denomination_trgm_idx;
DROP INDEX IF EXISTS unite_legale_search_denom_a_trgm_idx;
DROP INDEX IF EXISTS unite_legale_search_denom_f_trgm_idx;
ALTER TABLE unite_legale DROP COLUMN IF EXISTS search_denomination;

CREATE INDEX unite_legale_search_fts_idx ON unite_legale USING GIN (
  to_tsvector('french'::regconfig, public.immutable_unaccent(
    coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' ||
    coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, '')))
);

CREATE INDEX unite_legale_search_trgm_idx ON unite_legale USING GIN (
  lower(public.immutable_unaccent(
    coalesce(denomination, '') || ' ' || coalesce(denomination_usuelle_1, '') || ' ' ||
    coalesce(denomination_usuelle_2, '') || ' ' || coalesce(denomination_usuelle_3, ''))) gin_trgm_ops
);

-- ----------------------------------------------------------------------------
-- 4. commune_dim : dimension des communes, adressee par le parametre `commune`
--
-- ~44 000 lignes (couple code_commune / libelle_commune). Assez petite pour
-- porter a la fois un index FTS (prefixe) et un index trigramme (fautes de
-- frappe) sans cout notable.
-- ----------------------------------------------------------------------------

CREATE TABLE commune_dim (
  code_commune    VARCHAR NOT NULL,
  libelle_commune TEXT    NOT NULL,
  nb              BIGINT  NOT NULL DEFAULT 0,
  search_commune  TEXT GENERATED ALWAYS AS (lower(public.immutable_unaccent(libelle_commune))) STORED,
  PRIMARY KEY (code_commune, libelle_commune)
);

CREATE INDEX commune_dim_fts_idx  ON commune_dim USING GIN (to_tsvector('simple'::regconfig, search_commune));
CREATE INDEX commune_dim_trgm_idx ON commune_dim USING GIN (search_commune gin_trgm_ops);

-- ----------------------------------------------------------------------------
-- 5. search_lexicon : vocabulaire du corpus, pour corriger la REQUETE
--
-- On corrige les quelques mots saisis contre ~1,2 M de mots, au lieu de
-- fuzzy-matcher 42 M de lignes. `ndoc` sert de prior de frequence : seuls les
-- mots suffisamment frequents sont des cibles de correction valides, sinon le
-- corpus (qui contient les fautes) legitimerait les fautes.
-- ----------------------------------------------------------------------------

CREATE TABLE search_lexicon (
  source VARCHAR NOT NULL,
  word   TEXT    NOT NULL,
  ndoc   BIGINT  NOT NULL,
  PRIMARY KEY (source, word)
);

-- Index de generation de candidats : uniquement les mots assez frequents
CREATE INDEX search_lexicon_etablissement_trgm_idx ON search_lexicon USING GIN (word gin_trgm_ops)
  WHERE source = 'etablissement' AND ndoc >= 5;
CREATE INDEX search_lexicon_unite_legale_trgm_idx ON search_lexicon USING GIN (word gin_trgm_ops)
  WHERE source = 'unite_legale' AND ndoc >= 5;

-- ----------------------------------------------------------------------------
-- 6. commune_codes(q) : texte libre -> liste de code_commune
--
-- Strategie : correspondance FTS par prefixe d'abord (PARIS -> PARIS 1..20,
-- « etienne » -> SAINT-ETIENNE), repli trigramme si aucun resultat
-- (« marseile » -> MARSEILLE).
-- ----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION public.commune_codes(q TEXT)
RETURNS TEXT[] LANGUAGE plpgsql STABLE PARALLEL SAFE
SET pg_trgm.similarity_threshold = 0.4 AS $fn$
DECLARE
  normalized TEXT;
  tsq        TEXT;
  codes      TEXT[];
BEGIN
  normalized := lower(public.immutable_unaccent(coalesce(q, '')));

  SELECT string_agg(quote_literal(w) || ':*', ' & ')
    INTO tsq
    FROM unnest(regexp_split_to_array(normalized, '[^a-z0-9]+')) AS w
   WHERE w <> '';

  IF tsq IS NULL THEN
    RETURN ARRAY[]::TEXT[];
  END IF;

  SELECT array_agg(DISTINCT c.code_commune) INTO codes
    FROM commune_dim c
   WHERE to_tsvector('simple'::regconfig, c.search_commune) @@ to_tsquery('simple', tsq);

  IF codes IS NOT NULL AND cardinality(codes) > 0 THEN
    RETURN codes;
  END IF;

  -- Repli tolerant aux fautes (44 k lignes : quelques millisecondes)
  SELECT array_agg(DISTINCT c.code_commune) INTO codes
    FROM commune_dim c
   WHERE c.search_commune % normalized;

  RETURN coalesce(codes, ARRAY[]::TEXT[]);
END $fn$;

-- ----------------------------------------------------------------------------
-- 7. search_query(q, source) : texte saisi -> tsquery augmentee + suggestion
--
-- Principe : on n'ecrase JAMAIS le mot saisi, on ajoute une alternative en OU.
-- `creatiwity` reste donc trouvable meme si le lexique propose `creativite`.
-- Declencheur de correction : il existe un voisin a distance de Levenshtein
-- <= 2 au moins 20x plus frequent que le mot saisi.
-- ----------------------------------------------------------------------------

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

-- ----------------------------------------------------------------------------
-- 8. Maintenance : reconstruction complete (apres swap du stock mensuel)
-- ----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION public.search_refresh_full(source_name TEXT)
RETURNS BIGINT LANGUAGE plpgsql
SET work_mem TO '256MB' AS $fn$
DECLARE
  expression TEXT;
  inserted   BIGINT;
BEGIN
  expression := CASE source_name
    WHEN 'etablissement' THEN
      $e$coalesce(denomination_usuelle,'')||' '||coalesce(enseigne_1,'')||' '||coalesce(enseigne_2,'')||' '||coalesce(enseigne_3,'')$e$
    WHEN 'unite_legale' THEN
      $u$coalesce(denomination,'')||' '||coalesce(denomination_usuelle_1,'')||' '||coalesce(denomination_usuelle_2,'')||' '||coalesce(denomination_usuelle_3,'')$u$
    ELSE NULL
  END;

  IF expression IS NULL THEN
    RAISE EXCEPTION 'unknown search source: %', source_name;
  END IF;

  DELETE FROM search_lexicon WHERE source = source_name;

  EXECUTE format($x$
    INSERT INTO search_lexicon (source, word, ndoc)
    SELECT %L, word, ndoc FROM ts_stat($q$
      SELECT to_tsvector('simple'::regconfig, public.immutable_unaccent(%s)) FROM %I
    $q$)
    WHERE length(word) >= 3
    ON CONFLICT (source, word) DO UPDATE SET ndoc = excluded.ndoc
  $x$, source_name, expression, source_name);

  GET DIAGNOSTICS inserted = ROW_COUNT;

  IF source_name = 'etablissement' THEN
    DELETE FROM commune_dim;
    INSERT INTO commune_dim (code_commune, libelle_commune, nb)
    SELECT code_commune, libelle_commune, count(*)
      FROM etablissement
     WHERE code_commune IS NOT NULL AND libelle_commune IS NOT NULL
     GROUP BY code_commune, libelle_commune
    ON CONFLICT (code_commune, libelle_commune) DO UPDATE SET nb = excluded.nb;
    ANALYZE commune_dim;
  END IF;

  ANALYZE search_lexicon;
  RETURN inserted;
END $fn$;

-- ----------------------------------------------------------------------------
-- 9. Maintenance : fusion incrementale (apres la synchro quotidienne Insee)
--
-- Ne parcourt que les lignes touchees depuis `since`. `ndoc` derive legerement
-- a la hausse (les mots disparus ne sont pas decrementes) ; c'est sans effet,
-- ce n'est qu'un prior de frequence, et le swap mensuel remet le compteur a
-- plat via search_refresh_full().
-- ----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION public.search_refresh_incremental(source_name TEXT, since TIMESTAMP)
RETURNS BIGINT LANGUAGE plpgsql AS $fn$
DECLARE
  expression TEXT;
  merged     BIGINT;
BEGIN
  expression := CASE source_name
    WHEN 'etablissement' THEN
      $e$coalesce(denomination_usuelle,'')||' '||coalesce(enseigne_1,'')||' '||coalesce(enseigne_2,'')||' '||coalesce(enseigne_3,'')$e$
    WHEN 'unite_legale' THEN
      $u$coalesce(denomination,'')||' '||coalesce(denomination_usuelle_1,'')||' '||coalesce(denomination_usuelle_2,'')||' '||coalesce(denomination_usuelle_3,'')$u$
    ELSE NULL
  END;

  IF expression IS NULL THEN
    RAISE EXCEPTION 'unknown search source: %', source_name;
  END IF;

  EXECUTE format($x$
    INSERT INTO search_lexicon (source, word, ndoc)
    SELECT %L, word, ndoc FROM ts_stat($q$
      SELECT to_tsvector('simple'::regconfig, public.immutable_unaccent(%s))
        FROM %I WHERE date_dernier_traitement >= %L::timestamp
    $q$)
    WHERE length(word) >= 3
    ON CONFLICT (source, word) DO UPDATE SET ndoc = search_lexicon.ndoc + excluded.ndoc
  $x$, source_name, expression, source_name, since);

  GET DIAGNOSTICS merged = ROW_COUNT;

  IF source_name = 'etablissement' THEN
    INSERT INTO commune_dim (code_commune, libelle_commune, nb)
    SELECT code_commune, libelle_commune, count(*)
      FROM etablissement
     WHERE date_dernier_traitement >= since
       AND code_commune IS NOT NULL AND libelle_commune IS NOT NULL
     GROUP BY code_commune, libelle_commune
    ON CONFLICT (code_commune, libelle_commune) DO UPDATE SET nb = excluded.nb;
  END IF;

  -- La selectivite de la correction se lit sur la distribution de ndoc, que
  -- chaque fusion decale. Le seuil de l'autoanalyze (10 % des lignes) n'est
  -- jamais atteint par un delta quotidien, d'ou cet ANALYZE explicite.
  ANALYZE search_lexicon;

  RETURN merged;
END $fn$;

-- ----------------------------------------------------------------------------
-- 10. Tables de staging : elles doivent heriter des nouveaux index
--     (LIKE ... INCLUDING INDEXES fige les index au moment de la creation,
--     d'ou la recreation systematique a chaque migration touchant aux index)
-- ----------------------------------------------------------------------------

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);

-- ----------------------------------------------------------------------------
-- 11. Premier remplissage des donnees annexes + statistiques
-- ----------------------------------------------------------------------------

SELECT public.search_refresh_full('etablissement');
SELECT public.search_refresh_full('unite_legale');

ANALYZE etablissement;
ANALYZE unite_legale;
