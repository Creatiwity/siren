table! {
    group_metadata (id) {
        id -> Int4,
        group_type -> Text,
        insee_name -> Text,
        file_name -> Text,
        last_imported_timestamp -> Nullable<Timestamptz>,
        last_file_timestamp -> Nullable<Timestamptz>,
        staging_imported_timestamp -> Nullable<Timestamptz>,
        staging_file_timestamp -> Nullable<Timestamptz>,
        url -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
