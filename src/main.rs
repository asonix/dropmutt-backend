extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate futures;
extern crate futures_fs;
extern crate http as h;
#[macro_use]
extern crate log;
extern crate mime;

use actix::*;
use actix_web::{fs, http, middleware, multipart, server, App, AsyncResponder, Error, HttpMessage,
                HttpRequest, HttpResponse, State};
use futures::{Future, Stream, future::{result, Either}};
use futures_fs::FsPool;
use h::header::CONTENT_DISPOSITION;

#[derive(Clone)]
struct AppState {
    fs_pool: FsPool,
}

fn upload(
    req: HttpRequest<AppState>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.multipart()
        .from_err()
        .and_then(move |item| match item {
            multipart::MultipartItem::Field(field) => {
                let filename = {
                    let content_disposition =
                        if let Some(cd) = field.headers().get(CONTENT_DISPOSITION) {
                            cd
                        } else {
                            return Either::B(result(Ok(())));
                        };

                    let content_disposition = if let Ok(cd) = content_disposition.to_str() {
                        cd
                    } else {
                        return Either::B(result(Ok(())));
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
                        return Either::B(result(Ok(())));
                    }
                };
                info!("CD: {}", filename);

                let path: &std::path::Path = filename.as_ref();
                let filename = path.file_name().and_then(|filename| filename.to_str());

                let filename = if let Some(filename) = filename {
                    filename
                } else {
                    return Either::B(result(Ok(())));
                };

                if field.content_type().type_() == mime::IMAGE {
                    let write = state
                        .fs_pool
                        .write(format!("uploads/{}", filename), Default::default());
                    Either::A(field.map_err(Error::from).forward(write).map(|_| ()))
                } else {
                    Either::B(result(Ok(())))
                }
            }
            multipart::MultipartItem::Nested(_) => Either::B(result(Ok(()))),
        })
        .finish()
        .map(|_| HttpResponse::Ok().into())
        .responder()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web,symon_site=info");
    env_logger::init();
    let sys = actix::System::new("symon-site");

    server::new(|| {
        let state = AppState {
            fs_pool: FsPool::default(),
        };
        App::with_state(state)
            .middleware(middleware::Logger::default())
            .resource("/multipart", |r| r.method(http::Method::POST).with2(upload))
            .handler("/static", fs::StaticFiles::new("static"))
    }).bind("127.0.0.1:8080")
        .unwrap()
        .start();

    info!("Starting server on 127.0.0.1:8080");
    sys.run();
}
