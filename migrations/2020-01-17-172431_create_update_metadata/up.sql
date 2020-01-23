CREATE TABLE "public"."update_metadata"
(
    "id" serial,
    "synthetic_group_type" text NOT NULL,
    "force" boolean NOT NULL,
    "data_only" boolean NOT NULL,
    "status" text NOT NULL DEFAULT 'launched',
    "summary" jsonb,
    "error" text,
    "launched_timestamp" timestamptz NOT NULL,
    "finished_timestamp" timestamptz,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id")
);

SELECT diesel_manage_updated_at('update_metadata');
