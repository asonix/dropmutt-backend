use std::collections::BTreeMap;

use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::images;
use super::User;

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct FilesWithSizes {
    path: String,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize, Queryable, Serialize)]
pub struct ImageWithFiles {
    id: i32,
    files: Vec<FilesWithSizes>,
}

impl ImageWithFiles {
    pub fn recent(count: i64, conn: &PgConnection) -> Result<Vec<ImageWithFiles>, DropmuttError> {
        use schema::files;
        use schema::image_files;
        use diesel::prelude::*;

        let image_ids = images::table
            .select(images::dsl::id)
            .order(images::dsl::id.desc())
            .limit(count.min(30));

        image_files::table
            .inner_join(files::table)
            .filter(image_files::dsl::image_id.eq_any(image_ids))
            .order(image_files::dsl::width.asc())
            .select((
                image_files::dsl::image_id,
                image_files::dsl::width,
                image_files::dsl::height,
                files::dsl::file_path,
            ))
            .get_results(conn)
            .map(|results| {
                let mut v: Vec<_> = results
                    .into_iter()
                    .fold(BTreeMap::new(), |mut acc, (id, width, height, path)| {
                        {
                            let entry = acc.entry(id).or_insert(Vec::new());

                            entry.push(FilesWithSizes {
                                path,
                                width,
                                height,
                            });
                        }
                        acc
                    })
                    .into_iter()
                    .map(|(id, files)| ImageWithFiles { id, files })
                    .collect();

                v.reverse();
                v
            })
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

        let image_ids = images::table
            .filter(images::dsl::id.lt(id))
            .select(images::dsl::id)
            .order(images::dsl::id.desc())
            .limit(count.min(30));

        image_files::table
            .inner_join(files::table)
            .filter(image_files::dsl::image_id.eq_any(image_ids))
            .select((
                image_files::dsl::image_id,
                image_files::dsl::width,
                image_files::dsl::height,
                files::dsl::file_path,
            ))
            .get_results(conn)
            .map(|results| {
                let mut v: Vec<_> = results
                    .into_iter()
                    .fold(BTreeMap::new(), |mut acc, (id, width, height, path)| {
                        {
                            let entry = acc.entry(id).or_insert(Vec::new());

                            entry.push(FilesWithSizes {
                                path,
                                width,
                                height,
                            });
                        }
                        acc
                    })
                    .into_iter()
                    .map(|(id, files)| ImageWithFiles { id, files })
                    .collect();

                v.reverse();
                v
            })
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
}

impl NewImage {
    pub fn new(user: &User) -> Self {
        NewImage {
            uploaded_by: user.id(),
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
