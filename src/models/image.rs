use std::collections::BTreeMap;

use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::{self, images};
use super::UnprocessedImage;

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct FilesWithSizes {
    path: String,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct ImageWithFiles {
    id: i32,
    description: String,
    alternate_text: String,
    files: Vec<FilesWithSizes>,
}

impl ImageWithFiles {
    pub fn consolidate(
        results: Vec<(i32, String, String, i32, i32, String)>,
    ) -> Vec<ImageWithFiles> {
        let mut v: Vec<_> = results
            .into_iter()
            .fold(
                BTreeMap::new(),
                |mut acc, (id, desc, alt, width, height, path)| {
                    {
                        let entry = acc.entry((id, desc, alt)).or_insert(Vec::new());

                        entry.push(FilesWithSizes {
                            path,
                            width,
                            height,
                        });
                    }
                    acc
                },
            )
            .into_iter()
            .map(
                |((id, description, alternate_text), files)| ImageWithFiles {
                    id,
                    description,
                    alternate_text,
                    files,
                },
            )
            .collect();

        v.reverse();
        v
    }

    pub fn selection() -> (
        schema::image_files::columns::image_id,
        schema::images::description,
        schema::images::alternate_text,
        schema::image_files::columns::width,
        schema::image_files::columns::height,
        schema::files::columns::file_path,
    ) {
        use schema::files;
        use schema::image_files;

        (
            image_files::dsl::image_id,
            images::dsl::description,
            images::dsl::alternate_text,
            image_files::dsl::width,
            image_files::dsl::height,
            files::dsl::file_path,
        )
    }

    pub fn recent(count: i64, conn: &PgConnection) -> Result<Vec<ImageWithFiles>, DropmuttError> {
        use schema::files;
        use schema::image_files;
        use diesel::prelude::*;

        let image_ids: Vec<i32> = images::table
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

    pub fn before_id(
        count: i64,
        id: i32,
        conn: &PgConnection,
    ) -> Result<Vec<ImageWithFiles>, DropmuttError> {
        use schema::files;
        use schema::image_files;
        use diesel::prelude::*;

        let image_ids: Vec<i32> = images::table
            .filter(images::dsl::id.lt(id))
            .select(images::dsl::id)
            .order(images::dsl::id.desc())
            .limit(count.min(30))
            .get_results(conn)?;

        image_files::table
            .inner_join(images::table)
            .inner_join(files::table)
            .filter(image_files::dsl::image_id.eq_any(image_ids))
            .select(ImageWithFiles::selection())
            .get_results(conn)
            .map(ImageWithFiles::consolidate)
            .map_err(From::from)
    }
}

#[derive(Queryable)]
pub struct Image {
    id: i32,
    uploaded_by: i32,
}

impl Image {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn uploaded_by(&self) -> i32 {
        self.uploaded_by
    }
}

#[derive(Insertable)]
#[table_name = "images"]
pub struct NewImage {
    uploaded_by: i32,
    description: String,
    alternate_text: String,
}

impl NewImage {
    pub fn new(ui: &UnprocessedImage) -> Self {
        NewImage {
            uploaded_by: ui.uploaded_by(),
            description: ui.description().to_owned(),
            alternate_text: ui.description().to_owned(),
        }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<Image, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(images::table)
            .values(&self)
            .returning((images::dsl::id, images::dsl::uploaded_by))
            .get_result(conn)
            .map_err(From::from)
    }
}
