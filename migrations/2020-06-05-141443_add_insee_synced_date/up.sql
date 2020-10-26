ALTER TABLE "public"."group_metadata" ADD COLUMN "last_insee_synced_timestamp" timestamptz DEFAULT NULL;

CREATE INDEX "unite_legale_staging_date_dernier_traitement_idx" ON "public"."unite_legale_staging" USING BTREE ("date_dernier_traitement");
CREATE INDEX "unite_legale_date_dernier_traitement_idx" ON "public"."unite_legale" USING BTREE ("date_dernier_traitement");

CREATE INDEX "etablissement_staging_date_dernier_traitement_idx" ON "public"."etablissement_staging" USING BTREE ("date_dernier_traitement");
CREATE INDEX "etablissement_date_dernier_traitement_idx" ON "public"."etablissement" USING BTREE ("date_dernier_traitement");
