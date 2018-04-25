use std::path::{Path, PathBuf};

use actix_web::{multipart, HttpMessage, HttpRequest, error::PayloadError};
use bytes::{Bytes, BytesMut};
use futures::{Future, Stream, future::{result, Either}};
use futures_cpupool::CpuPool;
use futures_fs::FsPool;
use h::header::CONTENT_DISPOSITION;
use mime;
use serde::de::DeserializeOwned;
use serde_urlencoded;

use error::DropmuttError;
use super::AppState;

pub enum MultipartForm<T>
where
    T: DeserializeOwned,
{
    Form(T),
    File(PathBuf),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultipartData<T> {
    forms: Vec<T>,
    files: Vec<PathBuf>,
}

impl<T> MultipartData<T>
where
    T: DeserializeOwned,
{
    pub fn empty() -> Self {
        MultipartData {
            forms: vec![],
            files: vec![],
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.forms.extend(other.forms);
        self.files.extend(other.files);
    }
}

impl<T> From<MultipartForm<T>> for MultipartData<T>
where
    T: DeserializeOwned,
{
    fn from(m: MultipartForm<T>) -> Self {
        match m {
            MultipartForm::Form(form) => MultipartData {
                forms: vec![form],
                files: vec![],
            },
            MultipartForm::File(path) => MultipartData {
                forms: vec![],
                files: vec![path],
            },
        }
    }
}

fn handle_file_upload<S, T>(
    field: multipart::Field<S>,
    pool: FsPool<CpuPool>,
) -> impl Future<Item = MultipartForm<T>, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
    T: DeserializeOwned,
{
    info!("File upload: {:?}", field);
    let filename = {
        let content_disposition = if let Some(cd) = field.headers().get(CONTENT_DISPOSITION) {
            cd
        } else {
            return Either::B(result(Err(DropmuttError::ContentDisposition)));
        };

        let content_disposition = if let Ok(cd) = content_disposition.to_str() {
            cd
        } else {
            return Either::B(result(Err(DropmuttError::ContentDisposition)));
        };

        let filename = content_disposition
            .split(';')
            .skip(1)
            .filter_map(|section| {
                let mut parts = section.splitn(2, '=');

                let key = if let Some(key) = parts.next() {
                    key.trim()
                } else {
                    return None;
                };

                let val = if let Some(val) = parts.next() {
                    val.trim()
                } else {
                    return None;
                };

                if key == "filename" {
                    Some(val)
                } else {
                    None
                }
            })
            .next();

        if let Some(filename) = filename {
            filename.trim_matches('"').to_owned()
        } else {
            return Either::B(result(Err(DropmuttError::Filename)));
        }
    };

    let path: &Path = filename.as_ref();
    let filename = path.file_name().and_then(|filename| filename.to_str());

    let filename = if let Some(filename) = filename {
        filename.to_owned()
    } else {
        return Either::B(result(Err(DropmuttError::Filename)));
    };

    let write = match pool.write(format!("uploads/{}", filename), Default::default()) {
        Ok(writer) => writer,
        Err(e) => return Either::B(result(Err(e.into()))),
    };

    Either::A(
        field
            .map_err(DropmuttError::from)
            .forward(write)
            .map(move |_| MultipartForm::File(filename.into())),
    )
}

fn handle_form<S, T>(
    field: multipart::Field<S>,
) -> impl Future<Item = MultipartForm<T>, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
    T: DeserializeOwned,
{
    let form_size_limit = 80000;
    field
        .from_err()
        .fold((BytesMut::new(), 0), move |(mut acc, count), bytes| {
            let count = count + bytes.len();

            if count < form_size_limit {
                acc.extend_from_slice(&bytes);
                Ok((acc, count))
            } else {
                Err(DropmuttError::FormSize)
            }
        })
        .and_then(|(full_bytes, _)| {
            serde_urlencoded::from_bytes(&full_bytes)
                .map_err(From::from)
                .map(MultipartForm::Form)
        })
}

fn handle_multipart_field<S, T>(
    field: multipart::Field<S>,
    pool: FsPool<CpuPool>,
) -> impl Future<Item = MultipartForm<T>, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
    T: DeserializeOwned,
{
    let content_type = field.content_type().clone();

    if content_type == mime::APPLICATION_OCTET_STREAM || content_type.type_() == mime::IMAGE {
        Either::A(Either::A(handle_file_upload(field, pool)))
    } else if content_type == mime::APPLICATION_WWW_FORM_URLENCODED {
        Either::A(Either::B(handle_form(field)))
    } else {
        Either::B(result(Err(DropmuttError::ContentType)))
    }
}

pub fn handle_multipart<S, T>(
    m: multipart::Multipart<S>,
    pool: FsPool<CpuPool>,
) -> Box<Future<Item = MultipartData<T>, Error = DropmuttError>>
where
    S: Stream<Item = Bytes, Error = PayloadError> + 'static,
    T: DeserializeOwned + 'static,
{
    let max_files = 10;
    let max_forms = 100;

    Box::new(
        m.from_err()
            .and_then(move |item| match item {
                multipart::MultipartItem::Field(field) => {
                    Either::A(handle_multipart_field(field, pool.clone()).map(From::from))
                }
                multipart::MultipartItem::Nested(m) => Either::B(handle_multipart(m, pool.clone())),
            })
            .fold(
                (MultipartData::empty(), 0, 0),
                move |(mut acc, file_count, form_count), data| {
                    let file_count = file_count + data.files.len();
                    let form_count = form_count + data.forms.len();

                    if file_count < max_files && form_count < max_forms {
                        acc.merge(data);
                        Ok((acc, file_count, form_count))
                    } else if file_count > max_files {
                        Err(DropmuttError::FileCount)
                    } else {
                        Err(DropmuttError::FormCount)
                    }
                },
            )
            .map(|(data, _, _)| data),
    )
}

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
                Err(DropmuttError::ContentType)
            }
        }
        None => Err(DropmuttError::ContentType),
    }
}
