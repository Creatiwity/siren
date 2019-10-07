CREATE TABLE "public"."group_metadata"
(
    "id" serial,
    "group_type" text NOT NULL,
    "insee_name" text NOT NULL,
    "file_name" text NOT NULL,
    "last_imported_timestamp" timestamptz,
    "last_file_timestamp" timestamptz,
    "staging_imported_timestamp" timestamptz,
    "staging_file_timestamp" timestamptz,
    "url" text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "group_metadata_unique_group" ON "public"."group_metadata" USING BTREE
("group_type");

SELECT diesel_manage_updated_at('group_metadata');

INSERT INTO "public"."group_metadata"
    ("group_type", "insee_name", "file_name", "url")
VALUES
    ('unites_legales', 'Unités Légales', 'StockUniteLegale_utf8', 'http://files.data.gouv.fr/insee-sirene/StockUniteLegale_utf8.zip'),
    ('etablissements', 'Établissements', 'StockEtablissement_utf8', 'http://files.data.gouv.fr/insee-sirene/StockEtablissement_utf8.zip');
