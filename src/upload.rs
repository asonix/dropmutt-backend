use actix_web::{HttpMessage, HttpRequest};
use mime;

use super::AppState;
use error::DropmuttError;

pub enum PostKind {
    Multipart,
    UrlEncoded,
}

pub fn post_kind(req: &HttpRequest<AppState>) -> Result<PostKind, DropmuttError> {
    match req.mime_type()? {
        Some(mime_type) => {
            if mime_type.type_() == mime::MULTIPART {
                Ok(PostKind::Multipart)
            } else if mime_type == mime::APPLICATION_WWW_FORM_URLENCODED {
                Ok(PostKind::UrlEncoded)
            } else {
                warn!("Bad post kind: {}", mime_type);
                Err(DropmuttError::ContentType)
            }
        }
        None => {
            warn!("No mime_type on request");
            Err(DropmuttError::ContentType)
        }
    }
}
