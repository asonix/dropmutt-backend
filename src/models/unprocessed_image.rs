use diesel;
use diesel::pg::PgConnection;

use super::{File, Gallery, User};
use error::DropmuttError;
use schema::unprocessed_images;

#[derive(Queryable)]
pub struct UnprocessedImage {
    id: i32,
    uploaded_by: i32,
    image_file: i32,
    gallery_id: i32,
    alternate_text: String,
    description: String,
}

impl UnprocessedImage {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn uploaded_by(&self) -> i32 {
        self.uploaded_by
    }

    pub fn image_file(&self) -> i32 {
        self.image_file
    }

    pub fn gallery_id(&self) -> i32 {
        self.gallery_id
    }

    pub fn alternate_text(&self) -> &str {
        &self.alternate_text
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Insertable)]
#[table_name = "unprocessed_images"]
pub struct NewUnprocessedImage {
    uploaded_by: i32,
    image_file: i32,
    gallery_id: i32,
    alternate_text: String,
    description: String,
}

impl NewUnprocessedImage {
    pub fn new(
        user: &User,
        file: &File,
        gallery: &Gallery,
        alternate_text: String,
        description: String,
    ) -> Self {
        NewUnprocessedImage {
            uploaded_by: user.id(),
            image_file: file.id(),
            gallery_id: gallery.id(),
            alternate_text,
            description,
        }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<UnprocessedImage, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(unprocessed_images::table)
            .values(&self)
            .returning((
                unprocessed_images::dsl::id,
                unprocessed_images::dsl::uploaded_by,
                unprocessed_images::dsl::image_file,
                unprocessed_images::dsl::gallery_id,
                unprocessed_images::dsl::alternate_text,
                unprocessed_images::dsl::description,
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
