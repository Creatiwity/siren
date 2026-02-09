DELETE FROM "public"."group_metadata"
    WHERE "group_type" = 'liens_succession';

DROP TABLE "public"."lien_succession" CASCADE;
DROP TABLE "public"."lien_succession_staging" CASCADE;
