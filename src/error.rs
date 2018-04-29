use std::io;

use actix::MailboxError;
use actix_multipart;
use actix_web::{Error, HttpResponse, ResponseError, error::{ContentTypeError, MultipartError}};
use bcrypt::BcryptError;
use diesel;
use image;
use r2d2;

#[derive(Debug, Deserialize, Serialize)]
pub struct DropmutErrorResponse {
    errors: Vec<String>,
}

#[derive(Debug, Fail)]
pub enum DropmuttError {
    #[fail(display = "Error in actix, {}", _0)]
    Actix(Error),
    #[fail(display = "Error in diesel, {}", _0)]
    Diesel(#[cause] diesel::result::Error),
    #[fail(display = "Error in r2d2, {}", _0)]
    R2d2(#[cause] r2d2::Error),
    #[fail(display = "Error serving file, {}", _0)]
    IO(#[cause] io::Error),
    #[fail(display = "Error processing image, {}", _0)]
    Image(#[cause] image::ImageError),
    #[fail(display = "Problem in upload, {}", _0)]
    Upload(#[cause] actix_multipart::Error),
    #[fail(display = "Error processing image")]
    ImageProcessing,
    #[fail(display = "Request was made with bad Content-Type header")]
    ContentType,
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

impl From<io::Error> for DropmuttError {
    fn from(e: io::Error) -> Self {
        DropmuttError::IO(e)
    }
}

impl From<image::ImageError> for DropmuttError {
    fn from(e: image::ImageError) -> Self {
        DropmuttError::Image(e)
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
            DropmuttError::Diesel(ref e) => {
                let body = DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                };

                match *e {
                    diesel::result::Error::NotFound => HttpResponse::NotFound().json(body),
                    _ => HttpResponse::InternalServerError().json(body),
                }
            }
            DropmuttError::Upload(ref e) => HttpResponse::BadRequest().json(DropmutErrorResponse {
                errors: vec![format!("{}", e)],
            }),
            DropmuttError::Image(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::R2d2(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::IO(ref e) => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", e)],
                })
            }
            DropmuttError::Mailbox | DropmuttError::Bcrypt | DropmuttError::ImageProcessing => {
                HttpResponse::InternalServerError().json(DropmutErrorResponse {
                    errors: vec![format!("{}", self)],
                })
            }
            DropmuttError::Auth => HttpResponse::Unauthorized().json(DropmutErrorResponse {
                errors: vec![format!("{}", self)],
            }),
            | DropmuttError::ContentType | DropmuttError::Login | DropmuttError::SignupClosed => {
                HttpResponse::BadRequest().json(DropmutErrorResponse {
                    errors: vec![format!("{}", self)],
                })
            }
        }
    }
}
