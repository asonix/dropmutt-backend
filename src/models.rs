use std::fmt;

use error::DropmuttError;
use schema::users;

pub struct User {
    id: i32,
    username: String,
}

impl User {
    pub fn token_str(&self) -> String {
        format!("{}#{}", self.username, self.id)
    }
}

#[derive(Queryable)]
pub struct QueriedUser {
    id: i32,
    username: String,
    password: String,
}

impl fmt::Debug for QueriedUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "QueriedUser ( id: {}, username: {}, password: #[redacted] )",
            self.id, self.username
        )
    }
}

impl fmt::Display for QueriedUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "QueriedUser ( id: {}, username: {}, password: #[redacted] )",
            self.id, self.username
        )
    }
}

impl QueriedUser {
    fn verify(self, _password: &str) -> Result<User, DropmuttError> {
        Err(DropmuttError::Login)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    username: String,
    password: String,
}

impl fmt::Debug for NewUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NewUser ( username: {}, password: #[redacted] )",
            self.username
        )
    }
}

impl fmt::Display for NewUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NewUser ( username: {}, password: #[redacted] )",
            self.username
        )
    }
}
