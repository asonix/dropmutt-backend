table! {
    files (id) {
        id -> Int4,
        file_path -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    galleries (id) {
        id -> Int4,
        name -> Varchar,
        nsfw -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    gallery_images (id) {
        id -> Int4,
        gallery_id -> Int4,
        image_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    image_files (id) {
        id -> Int4,
        image_id -> Int4,
        file_id -> Int4,
        width -> Int4,
        height -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    images (id) {
        id -> Int4,
        uploaded_by -> Int4,
        description -> Text,
        alternate_text -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    unprocessed_images (id) {
        id -> Int4,
        uploaded_by -> Int4,
        image_file -> Int4,
        description -> Text,
        alternate_text -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        gallery_id -> Int4,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

joinable!(gallery_images -> galleries (gallery_id));
joinable!(gallery_images -> images (image_id));
joinable!(image_files -> files (file_id));
joinable!(image_files -> images (image_id));
joinable!(images -> users (uploaded_by));
joinable!(unprocessed_images -> files (image_file));
joinable!(unprocessed_images -> galleries (gallery_id));
joinable!(unprocessed_images -> users (uploaded_by));

allow_tables_to_appear_in_same_query!(
    files,
    galleries,
    gallery_images,
    image_files,
    images,
    unprocessed_images,
    users,
);
