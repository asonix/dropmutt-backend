use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::unprocessed_images;
use super::{File, User};

#[derive(Queryable)]
pub struct UnprocessedImage {
    id: i32,
    uploaded_by: i32,
    image_file: i32,
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
}

#[derive(Insertable)]
#[table_name = "unprocessed_images"]
pub struct NewUnprocessedImage {
    uploaded_by: i32,
    image_file: i32,
}

impl NewUnprocessedImage {
    pub fn new(user: &User, file: &File) -> Self {
        NewUnprocessedImage {
            uploaded_by: user.id(),
            image_file: file.id(),
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
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
