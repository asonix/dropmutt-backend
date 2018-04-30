use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::galleries;
use super::ImageWithFiles;

#[derive(Queryable)]
pub struct Gallery {
    id: i32,
    name: String,
    nsfw: bool,
}

impl Gallery {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn nsfw(&self) -> bool {
        self.nsfw
    }

    pub fn by_name(name: &str, conn: &PgConnection) -> Result<Gallery, DropmuttError> {
        use diesel::prelude::*;

        galleries::table
            .filter(galleries::dsl::name.eq(name))
            .limit(1)
            .select((
                galleries::dsl::id,
                galleries::dsl::name,
                galleries::dsl::nsfw,
            ))
            .get_result(conn)
            .map_err(From::from)
    }

    pub fn recent_by_name(
        name: &str,
        count: i64,
        conn: &PgConnection,
    ) -> Result<Vec<ImageWithFiles>, DropmuttError> {
        use schema::files;
        use schema::gallery_images;
        use schema::images;
        use schema::image_files;
        use diesel::prelude::*;

        let image_ids: Vec<i32> = galleries::table
            .inner_join(
                gallery_images::table.on(gallery_images::dsl::gallery_id.eq(galleries::dsl::id)),
            )
            .inner_join(images::table.on(gallery_images::dsl::image_id.eq(images::dsl::id)))
            .filter(galleries::dsl::name.eq(name))
            .select(images::dsl::id)
            .order(images::dsl::id.desc())
            .limit(count.min(30))
            .get_results(conn)?;

        image_files::table
            .inner_join(images::table)
            .inner_join(files::table)
            .filter(image_files::dsl::image_id.eq_any(image_ids))
            .order(image_files::dsl::width.asc())
            .select(ImageWithFiles::selection())
            .get_results(conn)
            .map(ImageWithFiles::consolidate)
            .map_err(From::from)
    }

    pub fn before_id_by_name(
        name: &str,
        count: i64,
        id: i32,
        conn: &PgConnection,
    ) -> Result<Vec<ImageWithFiles>, DropmuttError> {
        use schema::files;
        use schema::gallery_images;
        use schema::images;
        use schema::image_files;
        use diesel::prelude::*;

        let image_ids: Vec<i32> = galleries::table
            .inner_join(
                gallery_images::table.on(gallery_images::dsl::gallery_id.eq(galleries::dsl::id)),
            )
            .inner_join(images::table.on(gallery_images::dsl::image_id.eq(images::dsl::id)))
            .filter(galleries::dsl::name.eq(name))
            .filter(images::dsl::id.lt(id))
            .select(images::dsl::id)
            .order(images::dsl::id.desc())
            .limit(count.min(30))
            .get_results(conn)?;

        image_files::table
            .inner_join(images::table)
            .inner_join(files::table)
            .filter(image_files::dsl::image_id.eq_any(image_ids))
            .order(image_files::dsl::width.asc())
            .select(ImageWithFiles::selection())
            .get_results(conn)
            .map(ImageWithFiles::consolidate)
            .map_err(From::from)
    }
}

#[derive(Insertable)]
#[table_name = "galleries"]
pub struct NewGallery {
    name: String,
    nsfw: bool,
}

impl NewGallery {
    pub fn new(name: String, nsfw: bool) -> Self {
        NewGallery { name, nsfw }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<Gallery, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(galleries::table)
            .values(&self)
            .returning((
                galleries::dsl::id,
                galleries::dsl::name,
                galleries::dsl::nsfw,
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
