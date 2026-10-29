SET LOCAL maintenance_work_mem = '1GB';

ALTER TABLE etablissement DROP COLUMN position;
ALTER TABLE etablissement_staging DROP COLUMN position;

ALTER TABLE etablissement ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8), 2154), 4326)::geography) END) STORED;
ALTER TABLE etablissement_staging ADD COLUMN position geography(Point,4326) GENERATED ALWAYS AS (CASE WHEN coordonnee_lambert_x = '[ND]' THEN NULL ELSE (ST_Transform(ST_SetSRID(ST_MakePoint(coordonnee_lambert_x::float8, coordonnee_lambert_y::float8), 2154), 4326)::geography) END) STORED;

CREATE INDEX etablissement_position_index ON etablissement USING GIST (position);
CREATE INDEX etablissement_staging_position_idx ON etablissement_staging USING GIST (position);
