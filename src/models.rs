use std::fmt;

use bcrypt;
use diesel;
use diesel::pg::PgConnection;

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
    pub fn verify(self, password: &str) -> Result<User, DropmuttError> {
        bcrypt::verify(&password, &self.password)
            .map_err(From::from)
            .and_then(|verified| {
                if verified {
                    Ok(User {
                        id: self.id,
                        username: self.username,
                    })
                } else {
                    Err(DropmuttError::Login)
                }
            })
    }

    pub fn by_username(username: &str, conn: &PgConnection) -> Result<Self, DropmuttError> {
        use diesel::*;

        users::table
            .select((users::dsl::id, users::dsl::username, users::dsl::password))
            .filter(users::dsl::username.eq(username))
            .get_result(conn)
            .map_err(From::from)
    }
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    username: String,
    password: String,
}

impl NewUser {
    pub fn new(username: String, password: String) -> Result<Self, DropmuttError> {
        Ok(NewUser {
            username,
            password: bcrypt::hash(&password, bcrypt::DEFAULT_COST)?,
        })
    }

    pub fn create(self, conn: &PgConnection) -> Result<QueriedUser, DropmuttError> {
        use diesel::*;

        diesel::insert_into(users::table)
            .values(&self)
            .returning((users::dsl::id, users::dsl::username, users::dsl::password))
            .get_result(conn)
            .map_err(From::from)
    }
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
