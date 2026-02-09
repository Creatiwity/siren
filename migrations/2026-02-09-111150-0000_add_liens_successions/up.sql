INSERT INTO "public"."group_metadata"
    ("group_type", "insee_name", "file_name", "url")
VALUES
    ('liens_succession', 'Liens Succession', 'StockEtablissementLiensSuccession_utf8', 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockEtablissementLiensSuccession_utf8.zip');

CREATE TABLE "public"."lien_succession"
(
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "siret_etablissement_predecesseur" varchar(14) NOT NULL,
    "siret_etablissement_successeur" varchar(14) NOT NULL,
    "date_lien_succession" date NOT NULL,
    "transfert_siege" bool NOT NULL,
    "continuite_economique" bool NOT NULL,
    "date_dernier_traitement_lien_succession" timestamp
);

CREATE INDEX "lien_succession_predecesseur_index" ON "public"."lien_succession" USING BTREE
("siret_etablissement_predecesseur");

CREATE INDEX "lien_succession_successeur_index" ON "public"."lien_succession" USING BTREE
("siret_etablissement_successeur");

CREATE INDEX "lien_succession_date_index" ON "public"."lien_succession" USING BTREE
("date_lien_succession");

CREATE INDEX "lien_succession_date_dernier_traitement_index" ON "public"."lien_succession" USING BTREE
("date_dernier_traitement_lien_succession");

CREATE TABLE "public"."lien_succession_staging" (LIKE "public"."lien_succession" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
