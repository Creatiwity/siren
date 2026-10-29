-- `coordonnee_lambert_x/y` are only Lambert 93 in metropolitan France: INSEE
-- gives overseas establishments in their territory's legal UTM projection.
-- Reading everything as EPSG:2154 sent the Antilles to the Gulf of Guinea and
-- La Réunion to the North Sea. The projection is now picked from the commune:
--
--   971 Guadeloupe, 972 Martinique,
--   977 Saint-Barthélemy, 978 Saint-Martin  RGAF09 / UTM 20N    5490
--   973 Guyane                              RGFG95 / UTM 22N    2972
--   974 La Réunion                          RGR92 / UTM 40S     2975
--   975 Saint-Pierre-et-Miquelon            RGSPM06 / UTM 21N   4467
--   976 Mayotte                             RGM04 / UTM 38S     4471
--
-- PostgreSQL < 17 cannot change a generated expression in place, so the column
-- is dropped and re-added. This rewrites `etablissement` under an exclusive
-- lock; staging is empty between updates, so its rewrite is instant. Dropping
-- the column drops its GiST index, rebuilt below on both tables.

SET LOCAL maintenance_work_mem = '1GB';

ALTER TABLE etablissement DROP COLUMN position;
ALTER TABLE etablissement_staging DROP COLUMN position;

ALTER TABLE etablissement ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8),
  CASE left(code_commune, 3)
    WHEN '971' THEN 5490 WHEN '972' THEN 5490 WHEN '977' THEN 5490 WHEN '978' THEN 5490
    WHEN '973' THEN 2972 WHEN '974' THEN 2975 WHEN '975' THEN 4467 WHEN '976' THEN 4471
    ELSE 2154
  END), 4326)::geography) END) STORED;

ALTER TABLE etablissement_staging ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8),
  CASE left(code_commune, 3)
    WHEN '971' THEN 5490 WHEN '972' THEN 5490 WHEN '977' THEN 5490 WHEN '978' THEN 5490
    WHEN '973' THEN 2972 WHEN '974' THEN 2975 WHEN '975' THEN 4467 WHEN '976' THEN 4471
    ELSE 2154
  END), 4326)::geography) END) STORED;

CREATE INDEX etablissement_position_index ON etablissement USING GIST (position);
CREATE INDEX etablissement_staging_position_idx ON etablissement_staging USING GIST (position);
