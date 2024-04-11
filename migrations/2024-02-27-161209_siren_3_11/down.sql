ALTER TABLE "public"."etablissement"
DROP COLUMN IF EXISTS "dernier_numero_voie",
DROP COLUMN IF EXISTS "indice_repetition_dernier_numero_voie",
DROP COLUMN IF EXISTS "identifiant_adresse",
DROP COLUMN IF EXISTS "coordonnee_lambert_x",
DROP COLUMN IF EXISTS "coordonnee_lambert_y";

ALTER TABLE "public"."etablissement_staging"
DROP COLUMN IF EXISTS "dernier_numero_voie",
DROP COLUMN IF EXISTS "indice_repetition_dernier_numero_voie",
DROP COLUMN IF EXISTS "identifiant_adresse",
DROP COLUMN IF EXISTS "coordonnee_lambert_x",
DROP COLUMN IF EXISTS "coordonnee_lambert_y";
