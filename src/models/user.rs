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
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn token_str(&self) -> String {
        format!("{}#{}", self.username, self.id)
    }

    pub fn by_token(token: &str, conn: &PgConnection) -> Result<Self, DropmuttError> {
        let index = token.rfind('#').ok_or(DropmuttError::Login)?;

        let (uname, id) = token.split_at(index);
        let id: i32 = id.trim_left_matches('#')
            .parse()
            .map_err(|_| DropmuttError::Login)?;

        let user = QueriedUser::by_username(uname, conn)?;

        if user.id == id {
            Ok(User {
                id: user.id,
                username: user.username,
            })
        } else {
            Err(DropmuttError::Login)
        }
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
