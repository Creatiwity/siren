INSERT INTO "public"."group_metadata"
    ("group_type", "insee_name", "file_name", "url")
VALUES
    ('siren_doublons', 'Siren Doublons', 'StockDoublons_utf8', 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockDoublons_utf8.zip');

CREATE TABLE "public"."siren_doublons"
(
    "id" BIGSERIAL PRIMARY KEY,
    "siren_doublon" varchar(9) NOT NULL,
    "siren" varchar(9) NOT NULL,
    "date_dernier_traitement" date
);

CREATE INDEX "siren_doublons_lookup_index" ON "public"."siren_doublons" USING BTREE ("siren_doublon", "date_dernier_traitement" DESC NULLS LAST);

CREATE TABLE "public"."siren_doublons_staging" (LIKE "public"."siren_doublons" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES INCLUDING GENERATED);
