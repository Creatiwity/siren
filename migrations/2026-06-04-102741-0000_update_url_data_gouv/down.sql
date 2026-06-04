UPDATE "public"."group_metadata" SET "url" = 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockEtablissement_utf8.zip' WHERE "group_type" = 'etablissements';

UPDATE "public"."group_metadata" SET "url" = 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockUniteLegale_utf8.zip' WHERE "group_type" = 'unites_legales';

UPDATE "public"."group_metadata" SET "url" = 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockEtablissementLiensSuccession_utf8.zip' WHERE "group_type" = 'liens_succession';

UPDATE "public"."group_metadata" SET "url" = 'https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockDoublons_utf8.zip' WHERE "group_type" = 'siren_doublons';
