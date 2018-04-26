use std::string::FromUtf8Error;

use actix::MailboxError;
use actix_web::{Error, HttpResponse, ResponseError, error::{ContentTypeError, MultipartError}};
use bcrypt::BcryptError;
use diesel;
use futures_fs;
use r2d2;
use serde_urlencoded;

#[derive(Debug, Deserialize, Serialize)]
pub struct DropmutErrorResponse {
    errors: Vec<String>,
}

#[derive(Debug, Fail)]
pub enum DropmuttError {
    #[fail(display = "Error in actix, {}", _0)]
    Actix(Error),
    #[fail(display = "Error in futures fs, {}", _0)]
    FuturesFs(#[cause] futures_fs::Error),
    #[fail(display = "Error in diesel, {}", _0)]
    Diesel(#[cause] diesel::result::Error),
    #[fail(display = "Error in r2d2, {}", _0)]
    R2d2(#[cause] r2d2::Error),
    #[fail(display = "File upload is missing Content-Disposition header")]
    ContentDisposition,
    #[fail(display = "Request was made with bad Content-Type header")]
    ContentType,
    #[fail(display = "File uploads must have a filename")]
    Filename,
    #[fail(display = "Multipart forms must have field names")]
    Fieldname,
    #[fail(display = "Form too large")]
    FormSize,
    #[fail(display = "Failed to parse form")]
    UrlEncoded,
    #[fail(display = "Too many forms submitted")]
    FormCount,
    #[fail(display = "Too many files submitted")]
    FileCount,
    #[fail(display = "Field name contained invalid utf8")]
    Utf8,
    #[fail(display = "Failed to log in user")]
    Login,
    #[fail(display = "Failed to talk to actor")]
    Mailbox,
    #[fail(display = "Signup is closed")]
    SignupClosed,
    #[fail(display = "Must be authorized to perform this action")]
    Auth,
    #[fail(display = "Error in bcrypt")]
    Bcrypt,
}

impl From<Error> for DropmuttError {
    fn from(e: Error) -> Self {
        DropmuttError::Actix(e)
    }
}

impl From<MultipartError> for DropmuttError {
    fn from(e: MultipartError) -> Self {
        DropmuttError::Actix(e.into())
    }
}

impl From<ContentTypeError> for DropmuttError {
    fn from(e: ContentTypeError) -> Self {
        DropmuttError::Actix(e.into())
    }
}

impl From<futures_fs::Error> for DropmuttError {
    fn from(e: futures_fs::Error) -> Self {
        DropmuttError::FuturesFs(e)
    }
}

impl From<serde_urlencoded::de::Error> for DropmuttError {
    fn from(_: serde_urlencoded::de::Error) -> Self {
        DropmuttError::UrlEncoded
    }
}

impl From<FromUtf8Error> for DropmuttError {
    fn from(_: FromUtf8Error) -> Self {
        DropmuttError::Utf8
    }
}

impl From<MailboxError> for DropmuttError {
    fn from(_: MailboxError) -> Self {
        DropmuttError::Mailbox
    }
}

impl From<BcryptError> for DropmuttError {
    fn from(_: BcryptError) -> Self {
        DropmuttError::Bcrypt
    }
}

impl From<diesel::result::Error> for DropmuttError {
    fn from(e: diesel::result::Error) -> Self {
        DropmuttError::Diesel(e)
    }
}

impl From<r2d2::Error> for DropmuttError {
    fn from(e: r2d2::Error) -> Self {
        DropmuttError::R2d2(e)
    }
}

impl ResponseError for DropmuttError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            DropmuttError::Actix(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::FuturesFs(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::Diesel(ref e) => {
                let body = DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                };

                match *e {
                    diesel::result::Error::NotFound => HttpResponse::NotFound().json(body),
                    _ => HttpResponse::InternalServerError().json(body),
                }
            }
            DropmuttError::R2d2(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::Mailbox | DropmuttError::Bcrypt => HttpResponse::InternalServerError()
                .json(DropmutErrorResponse {
                    errors: vec![format!("{}", self)],
                }),
            DropmuttError::Auth => HttpResponse::Unauthorized().json(DropmutErrorResponse {
                errors: vec![format!("{}", self)],
            }),
            DropmuttError::ContentDisposition
            | DropmuttError::ContentType
            | DropmuttError::FormCount
            | DropmuttError::FileCount
            | DropmuttError::FormSize
            | DropmuttError::UrlEncoded
            | DropmuttError::Fieldname
            | DropmuttError::Utf8
            | DropmuttError::Login
            | DropmuttError::SignupClosed
            | DropmuttError::Filename => HttpResponse::BadRequest().json(DropmutErrorResponse {
                errors: vec![format!("{}", self)],
            }),
        }
    }
}
