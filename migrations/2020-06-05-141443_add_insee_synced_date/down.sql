ALTER TABLE "public"."group_metadata" DROP COLUMN "last_insee_synced_timestamp";

DROP INDEX "public"."unite_legale_staging_date_dernier_traitement_idx";

DROP INDEX "public"."etablissement_staging_date_dernier_traitement_idx";

DROP INDEX "public"."unite_legale_date_dernier_traitement_idx";

DROP INDEX "public"."etablissement_date_dernier_traitement_idx";
