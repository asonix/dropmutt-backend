use actix::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use error::DropmuttError;
use models;
use schema;

pub struct DbActor {
    conn: Pool<ConnectionManager<PgConnection>>,
}

impl Actor for DbActor {
    type Context = SyncContext<Self>;
}

pub struct CreateUser {
    username: String,
    password: String,
}

impl Message for CreateUser {
    type Result = Result<models::QueriedUser, DropmuttError>;
}
