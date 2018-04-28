table! {
    files (id) {
        id -> Int4,
        file_path -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    images (id) {
        id -> Int4,
        uploaded_by -> Int4,
        size_200 -> Int4,
        size_400 -> Int4,
        size_800 -> Nullable<Int4>,
        size_1200 -> Nullable<Int4>,
        size_full -> Int4,
        width -> Int4,
        height -> Int4,
        ratio -> Float4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    unprocessed_images (id) {
        id -> Int4,
        uploaded_by -> Int4,
        image_file -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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

joinable!(images -> users (uploaded_by));
joinable!(unprocessed_images -> files (image_file));
joinable!(unprocessed_images -> users (uploaded_by));

allow_tables_to_appear_in_same_query!(files, images, unprocessed_images, users,);
