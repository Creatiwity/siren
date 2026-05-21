DELETE FROM "public"."group_metadata"
    WHERE "group_type" = 'siren_doublons';

DROP TABLE "public"."siren_doublons" CASCADE;
DROP TABLE "public"."siren_doublons_staging" CASCADE;
