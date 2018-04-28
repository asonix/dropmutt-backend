use diesel;
use diesel::pg::PgConnection;

use error::DropmuttError;
use schema::images;
use super::{File, User};

#[derive(Queryable)]
pub struct Image {
    id: i32,
    uploaded_by: i32,
    size_200: i32,
    size_400: i32,
    size_800: Option<i32>,
    size_1200: Option<i32>,
    size_full: i32,
    width: i32,
    height: i32,
    ratio: f32,
}

impl Image {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn uploaded_by(&self) -> i32 {
        self.uploaded_by
    }

    pub fn size_200(&self) -> i32 {
        self.size_200
    }

    pub fn size_400(&self) -> i32 {
        self.size_400
    }

    pub fn size_800(&self) -> Option<i32> {
        self.size_800
    }

    pub fn size_1200(&self) -> Option<i32> {
        self.size_1200
    }

    pub fn size_full(&self) -> i32 {
        self.size_full
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }
}

#[derive(Insertable)]
#[table_name = "images"]
pub struct NewImage {
    uploaded_by: i32,
    size_200: i32,
    size_400: i32,
    size_800: Option<i32>,
    size_1200: Option<i32>,
    size_full: i32,
    width: i32,
    height: i32,
    ratio: f32,
}

impl NewImage {
    pub fn new(
        user: &User,
        size_200: &File,
        size_400: &File,
        size_800: Option<&File>,
        size_1200: Option<&File>,
        size_full: &File,
        width: i32,
        height: i32,
    ) -> Self {
        NewImage {
            uploaded_by: user.id(),
            size_200: size_200.id(),
            size_400: size_400.id(),
            size_800: size_800.map(File::id),
            size_1200: size_1200.map(File::id),
            size_full: size_full.id(),
            width,
            height,
            ratio: (width as f32) / (height as f32),
        }
    }

    pub fn insert(self, conn: &PgConnection) -> Result<Image, DropmuttError> {
        use diesel::prelude::*;

        diesel::insert_into(images::table)
            .values(&self)
            .returning((
                images::dsl::id,
                images::dsl::uploaded_by,
                images::dsl::size_200,
                images::dsl::size_400,
                images::dsl::size_800,
                images::dsl::size_1200,
                images::dsl::size_full,
                images::dsl::width,
                images::dsl::height,
                images::dsl::ratio,
            ))
            .get_result(conn)
            .map_err(From::from)
    }
}
