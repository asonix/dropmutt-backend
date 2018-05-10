use std::path::PathBuf;

use actix::prelude::*;
use diesel::{
    pg::PgConnection, r2d2::{ConnectionManager, Pool},
};

use error::DropmuttError;
use image_processor::ProcessResponse;
use models;

pub struct DbActor {
    conn: Pool<ConnectionManager<PgConnection>>,
}

impl DbActor {
    pub fn new(conn: Pool<ConnectionManager<PgConnection>>) -> Self {
        DbActor { conn }
    }
}

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

impl Handler<CreateUser> for DbActor {
    type Result = Result<models::User, DropmuttError>;

    fn handle(&mut self, msg: CreateUser, _: &mut Self::Context) -> Self::Result {
        models::NewUser::new(msg.username.clone(), msg.password.clone())?
            .create(&*self.conn.get()?)
            .and_then(move |user| user.verify(&msg.password))
    }
}

impl Handler<LookupUser> for DbActor {
    type Result = Result<models::QueriedUser, DropmuttError>;

    fn handle(&mut self, msg: LookupUser, _: &mut Self::Context) -> Self::Result {
        models::QueriedUser::by_username(&msg.0, &*self.conn.get()?)
    }
}

impl Handler<StoreImage> for DbActor {
    type Result = Result<(models::UnprocessedImage, models::File), DropmuttError>;

    fn handle(&mut self, msg: StoreImage, _: &mut Self::Context) -> Self::Result {
        use diesel::Connection;

        let conn: &PgConnection = &*self.conn.get()?;

        conn.transaction(|| {
            let user = models::User::by_token(&msg.user_token, conn)?;
            let file = store_path(msg.file_path, conn)?;
            let gallery = models::Gallery::by_name(&msg.gallery_name, conn)?;
            let image = models::NewUnprocessedImage::new(
                &user,
                &file,
                &gallery,
                msg.alternate_text,
                msg.description,
            ).insert(conn)?;

            Ok((image, file))
        })
    }
}

fn store_path(path: PathBuf, conn: &PgConnection) -> Result<models::File, DropmuttError> {
    let path = path.to_str()
        .ok_or(DropmuttError::ImageProcessing)?
        .trim_matches('"')
        .to_owned();

    models::NewFile::new(path).insert(conn)
}

impl Handler<StoreProcessedImage> for DbActor {
    type Result = Result<models::Image, DropmuttError>;

    fn handle(&mut self, msg: StoreProcessedImage, _: &mut Self::Context) -> Self::Result {
        use diesel::Connection;

        let conn: &PgConnection = &*self.conn.get()?;

        conn.transaction(|| {
            let files = msg.1.files.into_iter().fold(
                Ok(Vec::new()) as Result<Vec<_>, DropmuttError>,
                |acc, (path, width, height)| match acc {
                    Ok(mut acc) => {
                        acc.push((store_path(path, conn)?, width, height));

                        Ok(acc)
                    }
                    Err(e) => Err(e),
                },
            )?;

            let image = models::NewImage::new(&msg.0).insert(conn)?;
            let gallery = models::Gallery::by_id(msg.0.gallery_id(), conn)?;
            models::NewGalleryImage::new(&gallery, &image).insert(conn)?;

            files.iter().fold(
                Ok(image) as Result<_, DropmuttError>,
                |acc, (file, width, height)| match acc {
                    Ok(image) => {
                        models::NewImageFile::new(&image, file, *width, *height).insert(conn)?;

                        Ok(image)
                    }
                    Err(e) => Err(e),
                },
            )
        })
    }
}

impl Handler<FetchImages> for DbActor {
    type Result = Result<Vec<models::ImageWithFiles>, DropmuttError>;

    fn handle(&mut self, msg: FetchImages, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &*self.conn.get()?;

        match msg.before_id {
            Some(id) => models::ImageWithFiles::before_id(msg.count, id, conn),
            None => models::ImageWithFiles::recent(msg.count, conn),
        }
    }
}

impl Handler<FetchGalleries> for DbActor {
    type Result = Result<Vec<String>, DropmuttError>;

    fn handle(&mut self, _: FetchGalleries, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &*self.conn.get()?;

        models::Gallery::all(conn).map(|galleries| {
            galleries
                .iter()
                .map(|gallery| gallery.name().to_owned())
                .collect()
        })
    }
}

impl Handler<FetchImagesInGallery> for DbActor {
    type Result = Result<Vec<models::ImageWithFiles>, DropmuttError>;

    fn handle(&mut self, msg: FetchImagesInGallery, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &*self.conn.get()?;

        match msg.before_id {
            Some(id) => models::Gallery::before_id_by_name(&msg.gallery, msg.count, id, conn),
            None => models::Gallery::recent_by_name(&msg.gallery, msg.count, conn),
        }
    }
}

pub struct CreateUser {
    pub username: String,
    pub password: String,
}

impl Message for CreateUser {
    type Result = Result<models::User, DropmuttError>;
}

pub struct LookupUser(pub String);

impl Message for LookupUser {
    type Result = Result<models::QueriedUser, DropmuttError>;
}

pub struct StoreImage {
    pub user_token: String,
    pub file_path: PathBuf,
    pub gallery_name: String,
    pub description: String,
    pub alternate_text: String,
}

impl Message for StoreImage {
    type Result = Result<(models::UnprocessedImage, models::File), DropmuttError>;
}

pub struct StoreProcessedImage(pub models::UnprocessedImage, pub ProcessResponse);

impl Message for StoreProcessedImage {
    type Result = Result<models::Image, DropmuttError>;
}

pub struct FetchImages {
    pub count: i64,
    pub before_id: Option<i32>,
}

impl Message for FetchImages {
    type Result = Result<Vec<models::ImageWithFiles>, DropmuttError>;
}

pub struct FetchGalleries;

impl Message for FetchGalleries {
    type Result = Result<Vec<String>, DropmuttError>;
}

pub struct FetchImagesInGallery {
    pub gallery: String,
    pub count: i64,
    pub before_id: Option<i32>,
}

impl Message for FetchImagesInGallery {
    type Result = Result<Vec<models::ImageWithFiles>, DropmuttError>;
}
