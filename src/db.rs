use std::path::PathBuf;

use actix::prelude::*;
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool}};

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
            let user = models::User::by_token(&msg.0, conn)?;
            let file = store_path(msg.1, conn)?;
            let image = models::NewUnprocessedImage::new(&user, &file).insert(conn)?;

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
    type Result = Result<(models::Image, Vec<models::File>), DropmuttError>;

    fn handle(&mut self, msg: StoreProcessedImage, _: &mut Self::Context) -> Self::Result {
        use diesel::Connection;

        let conn: &PgConnection = &*self.conn.get()?;

        conn.transaction(|| {
            let user = models::User::by_token(&msg.0, conn)?;
            let file_200 = store_path(msg.1.path_200, conn)?;
            let file_400 = store_path(msg.1.path_400, conn)?;
            let file_800 = if let Some(path_800) = msg.1.path_800 {
                Some(store_path(path_800, conn)?)
            } else {
                None
            };
            let file_1200 = if let Some(path_1200) = msg.1.path_1200 {
                Some(store_path(path_1200, conn)?)
            } else {
                None
            };
            let file_full = store_path(msg.1.path_full, conn)?;

            models::NewImage::new(
                &user,
                &file_200,
                &file_400,
                file_800.as_ref(),
                file_1200.as_ref(),
                &file_full,
                msg.1.width,
                msg.1.height,
            ).insert(conn)
                .map(|image| {
                    let mut v = vec![file_200, file_400, file_full];
                    v.extend(file_800);
                    v.extend(file_1200);

                    (image, v)
                })
        })
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

pub struct StoreImage(pub String, pub PathBuf);

impl Message for StoreImage {
    type Result = Result<(models::UnprocessedImage, models::File), DropmuttError>;
}

pub struct StoreProcessedImage(pub String, pub ProcessResponse);

impl Message for StoreProcessedImage {
    type Result = Result<(models::Image, Vec<models::File>), DropmuttError>;
}
