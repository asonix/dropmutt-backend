use std::path::Path;

use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::files;

#[derive(Queryable)]
pub struct File {
    id: i32,
    file_path: String,
}

impl File {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn path(&self) -> &Path {
        self.file_path.as_ref()
    }
}

impl AsRef<Path> for File {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

#[derive(Insertable)]
#[table_name = "files"]
pub struct NewFile {
    file_path: String,
}

impl NewFile {
    pub fn new(file_path: String) -> Self {
        NewFile { file_path }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<File, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(files::table)
            .values(&self)
            .returning((files::dsl::id, files::dsl::file_path))
            .get_result(conn)
            .map_err(From::from)
    }
}
