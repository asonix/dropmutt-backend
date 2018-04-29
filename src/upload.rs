use std::{fs::DirBuilder, os::unix::fs::DirBuilderExt, path::{Path, PathBuf}};

use actix_web::{multipart, HttpMessage, HttpRequest, error::PayloadError};
use bytes::{Bytes, BytesMut};
use futures::{Future, Stream, future::{result, Either}};
use futures_cpupool::CpuPool;
use futures_fs::FsPool;
use h::header::CONTENT_DISPOSITION;
use mime;
use mime_guess;

use error::DropmuttError;
use path_generator::PathGenerator;
use super::AppState;

type MultipartHash = (String, MultipartContent);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MultipartContent {
    File {
        filename: String,
        stored_as: PathBuf,
    },
    Body(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MultipartNamePart {
    Name(String),
    Map(String),
    Array,
}

pub type MultipartForm = Vec<(Vec<MultipartNamePart>, MultipartContent)>;

fn parse_multipart_name(name: String) -> Result<Vec<MultipartNamePart>, DropmuttError> {
    name.split('[')
        .map(|part| {
            if part.len() == 1 && part.ends_with(']') {
                MultipartNamePart::Array
            } else if part.ends_with(']') {
                MultipartNamePart::Map(part.trim_right_matches(']').to_owned())
            } else {
                MultipartNamePart::Name(part.to_owned())
            }
        })
        .fold(Ok(vec![]), |acc, part| match acc {
            Ok(mut v) => {
                if let MultipartNamePart::Name(_) = part {
                    if v.len() > 0 {
                        return Err(DropmuttError::ContentDisposition);
                    }
                }

                v.push(part);
                Ok(v)
            }
            Err(e) => Err(e),
        })
}

pub struct ContentDisposition {
    name: Option<String>,
    filename: Option<String>,
}

impl ContentDisposition {
    fn empty() -> Self {
        ContentDisposition {
            name: None,
            filename: None,
        }
    }
}

fn parse_content_disposition<S>(
    field: &multipart::Field<S>,
) -> Result<ContentDisposition, DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
{
    let content_disposition = if let Some(cd) = field.headers().get(CONTENT_DISPOSITION) {
        cd
    } else {
        return Err(DropmuttError::ContentDisposition);
    };

    let content_disposition = if let Ok(cd) = content_disposition.to_str() {
        cd
    } else {
        return Err(DropmuttError::ContentDisposition);
    };

    Ok(content_disposition
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

            Some((key, val.trim_matches('"')))
        })
        .fold(ContentDisposition::empty(), |mut acc, (key, val)| {
            if key == "name" {
                acc.name = Some(val.to_owned());
            } else if key == "filename" {
                acc.filename = Some(val.to_owned());
            }
            acc
        }))
}

fn handle_file_upload<S>(
    field: multipart::Field<S>,
    pool: CpuPool,
    path_generator: PathGenerator,
) -> impl Future<Item = MultipartHash, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
{
    let content_disposition = match parse_content_disposition(&field) {
        Ok(cd) => cd,
        Err(e) => return Either::B(result(Err(e))),
    };

    let filename = match content_disposition.filename {
        Some(filename) => filename,
        None => return Either::B(result(Err(DropmuttError::Filename))),
    };

    let name = match content_disposition.name {
        Some(name) => name,
        None => return Either::B(result(Err(DropmuttError::Fieldname))),
    };

    let path: &Path = filename.as_ref();
    let filename = path.file_name().and_then(|filename| filename.to_str());

    let filename = if let Some(filename) = filename {
        filename.to_owned()
    } else {
        return Either::B(result(Err(DropmuttError::Filename)));
    };

    let extension = mime_guess::get_mime_extensions(field.content_type())
        .and_then(|extensions| extensions.first().map(|r| *r))
        .unwrap_or("png");

    let stored_as = Path::new("uploads").join(path_generator.next_path(extension));
    let mut stored_dir = stored_as.clone();
    stored_dir.pop();

    Either::A(pool.spawn_fn(move || {
        DirBuilder::new()
            .recursive(true)
            .mode(0o755)
            .create(stored_dir.clone())
            .map_err(From::from)
    }).and_then(move |_| {
        let write =
            FsPool::from_executor(pool.clone()).write(stored_as.clone(), Default::default());
        field
            .map_err(DropmuttError::from)
            .forward(write)
            .map(move |_| {
                (
                    name,
                    MultipartContent::File {
                        filename,
                        stored_as,
                    },
                )
            })
    }))
}

fn handle_form_data<S>(
    field: multipart::Field<S>,
) -> impl Future<Item = MultipartHash, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
{
    let content_disposition = match parse_content_disposition(&field) {
        Ok(cd) => cd,
        Err(e) => return Either::B(result(Err(e))),
    };

    let name = match content_disposition.name {
        Some(name) => name,
        None => return Either::B(result(Err(DropmuttError::Fieldname))),
    };

    let max_body_size = 80000;

    Either::A(
        field
            .from_err()
            .fold(BytesMut::new(), move |mut acc, bytes| {
                if acc.len() + bytes.len() < max_body_size {
                    acc.extend(bytes);
                    Ok(acc)
                } else {
                    Err(DropmuttError::FormSize)
                }
            })
            .and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .map(|string| (name, MultipartContent::Body(string)))
                    .map_err(From::from)
            }),
    )
}

fn handle_multipart_field<S>(
    field: multipart::Field<S>,
    pool: CpuPool,
    path_generator: PathGenerator,
) -> impl Future<Item = MultipartHash, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError>,
{
    let content_type = field.content_type().clone();

    if content_type.type_() == mime::IMAGE {
        Either::A(Either::A(handle_file_upload(field, pool, path_generator)))
    } else if content_type == mime::APPLICATION_OCTET_STREAM {
        Either::A(Either::B(handle_form_data(field)))
    } else {
        warn!("Bad Content-Type header: {}", content_type);
        Either::B(result(Err(DropmuttError::ContentType)))
    }
}

pub fn handle_multipart<S>(
    m: multipart::Multipart<S>,
    pool: CpuPool,
    path_generator: PathGenerator,
) -> Box<Stream<Item = MultipartHash, Error = DropmuttError>>
where
    S: Stream<Item = Bytes, Error = PayloadError> + 'static,
{
    Box::new(
        m.map_err(DropmuttError::from)
            .map(move |item| match item {
                multipart::MultipartItem::Field(field) => {
                    info!("Field: {:?}", field);
                    Box::new(
                        handle_multipart_field(field, pool.clone(), path_generator.clone())
                            .map(From::from)
                            .into_stream(),
                    )
                        as Box<Stream<Item = MultipartHash, Error = DropmuttError>>
                }
                multipart::MultipartItem::Nested(m) => {
                    info!("Nested");
                    Box::new(handle_multipart(m, pool.clone(), path_generator.clone()))
                        as Box<Stream<Item = MultipartHash, Error = DropmuttError>>
                }
            })
            .flatten(),
    )
}

pub fn do_multipart_handling<S>(
    m: multipart::Multipart<S>,
    pool: CpuPool,
    path_generator: PathGenerator,
) -> impl Future<Item = MultipartForm, Error = DropmuttError>
where
    S: Stream<Item = Bytes, Error = PayloadError> + 'static,
{
    let max_files = 10;
    let max_fields = 100;

    handle_multipart(m, pool, path_generator)
        .fold(
            (Vec::new(), 0, 0),
            move |(mut acc, file_count, field_count), (name, content)| match content {
                MultipartContent::File {
                    filename,
                    stored_as,
                } => {
                    let file_count = file_count + 1;

                    if file_count < max_files {
                        parse_multipart_name(name).map(|name| {
                            acc.push((
                                name,
                                MultipartContent::File {
                                    filename,
                                    stored_as,
                                },
                            ));

                            (acc, file_count, field_count)
                        })
                    } else {
                        Err(DropmuttError::FileCount)
                    }
                }
                b @ MultipartContent::Body(_) => {
                    let field_count = field_count + 1;

                    if field_count < max_fields {
                        parse_multipart_name(name).map(|name| {
                            acc.push((name, b));

                            (acc, file_count, field_count)
                        })
                    } else {
                        Err(DropmuttError::FormCount)
                    }
                }
            },
        )
        .map(|(multipart_form, _, _)| multipart_form)
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
