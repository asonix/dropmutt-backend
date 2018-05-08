use diesel;
use diesel::pg::PgConnection;

use super::{Gallery, Image};
use error::DropmuttError;
use schema::gallery_images;

#[derive(Queryable)]
pub struct GalleryImage {
    id: i32,
    gallery_id: i32,
    image_id: i32,
}

impl GalleryImage {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn gallery_id(&self) -> i32 {
        self.gallery_id
    }

    pub fn image_id(&self) -> i32 {
        self.image_id
    }
}

#[derive(Insertable)]
#[table_name = "gallery_images"]
pub struct NewGalleryImage {
    gallery_id: i32,
    image_id: i32,
}

impl NewGalleryImage {
    pub fn new(gallery: &Gallery, image: &Image) -> Self {
        NewGalleryImage {
            gallery_id: gallery.id(),
            image_id: image.id(),
        }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<GalleryImage, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(gallery_images::table)
            .values(&self)
            .returning((
                gallery_images::dsl::id,
                gallery_images::dsl::gallery_id,
                gallery_images::dsl::image_id,
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
