ALTER TABLE "public"."etablissement"
ADD COLUMN "dernier_numero_voie" text DEFAULT NULL,
ADD COLUMN "indice_repetition_dernier_numero_voie" text DEFAULT NULL,
ADD COLUMN "identifiant_adresse" text DEFAULT NULL,
ADD COLUMN "coordonnee_lambert_x" text DEFAULT NULL,
ADD COLUMN "coordonnee_lambert_y" text DEFAULT NULL;

ALTER TABLE "public"."etablissement_staging"
ADD COLUMN "dernier_numero_voie" text DEFAULT NULL,
ADD COLUMN "indice_repetition_dernier_numero_voie" text DEFAULT NULL,
ADD COLUMN "identifiant_adresse" text DEFAULT NULL,
ADD COLUMN "coordonnee_lambert_x" text DEFAULT NULL,
ADD COLUMN "coordonnee_lambert_y" text DEFAULT NULL;
