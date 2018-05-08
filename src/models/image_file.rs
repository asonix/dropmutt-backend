use diesel;
use diesel::pg::PgConnection;

use super::{File, Image};
use error::DropmuttError;
use schema::image_files;

#[derive(Queryable)]
pub struct ImageFile {
    id: i32,
    image_id: i32,
    file_id: i32,
    width: i32,
    height: i32,
}

impl ImageFile {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn image_id(&self) -> i32 {
        self.image_id
    }

    pub fn file_id(&self) -> i32 {
        self.file_id
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }
}

#[derive(Insertable)]
#[table_name = "image_files"]
pub struct NewImageFile {
    image_id: i32,
    file_id: i32,
    width: i32,
    height: i32,
}

impl NewImageFile {
    pub fn new(image: &Image, file: &File, width: i32, height: i32) -> Self {
        NewImageFile {
            image_id: image.id(),
            file_id: file.id(),
            width,
            height,
        }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<ImageFile, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(image_files::table)
            .values(&self)
            .returning((
                image_files::dsl::id,
                image_files::dsl::image_id,
                image_files::dsl::file_id,
                image_files::dsl::width,
                image_files::dsl::height,
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
