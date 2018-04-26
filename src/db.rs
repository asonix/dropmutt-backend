use actix::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

use error::DropmuttError;
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
