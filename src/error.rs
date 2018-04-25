use actix_web::{Error, HttpResponse, ResponseError, error::{ContentTypeError, MultipartError}};
use futures_fs;
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
    #[fail(display = "File upload is missing Content-Disposition header")]
    ContentDisposition,
    #[fail(display = "Request was made with bad Content-Type header")]
    ContentType,
    #[fail(display = "File uploads must have a filename")]
    Filename,
    #[fail(display = "Form too large")]
    FormSize,
    #[fail(display = "Failed to parse form")]
    UrlEncoded,
    #[fail(display = "Too many forms submitted")]
    FormCount,
    #[fail(display = "Too many files submitted")]
    FileCount,
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
            DropmuttError::ContentDisposition
            | DropmuttError::ContentType
            | DropmuttError::FormCount
            | DropmuttError::FileCount
            | DropmuttError::FormSize
            | DropmuttError::UrlEncoded
            | DropmuttError::Filename => HttpResponse::BadRequest().json(DropmutErrorResponse {
                errors: vec![format!("{}", self)],
            }),
        }
    }
}
