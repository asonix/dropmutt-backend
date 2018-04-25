extern crate actix;
extern crate actix_web;
extern crate bytes;
#[macro_use]
extern crate diesel;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate futures_fs;
extern crate http as h;
#[macro_use]
extern crate log;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_urlencoded;

use actix_web::{fs, http, middleware, server, App, AsyncResponder, HttpMessage, HttpRequest,
                HttpResponse, State};
use futures::{Future, future::{result, Either}};
use futures_cpupool::CpuPool;
use futures_fs::FsPool;

mod error;
mod models;
mod schema;
mod upload;

use self::error::DropmuttError;
use self::upload::{do_multipart_handling, post_kind, MultipartForm, PostKind};

#[derive(Debug, Deserialize, Serialize)]
pub struct TestData {
    content: String,
}

#[derive(Clone)]
pub struct AppState {
    fs_pool: FsPool<CpuPool>,
}

fn upload(
    req: HttpRequest<AppState>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    result(post_kind(&req))
        .and_then(move |upload_kind| match upload_kind {
            PostKind::Multipart => Either::A(
                do_multipart_handling(req.multipart(), state.fs_pool.clone())
                    .map(|m: MultipartForm| {
                        info!("Responding with {:?}", m);
                        HttpResponse::Created().json(m)
                    })
                    .map_err(|e| {
                        info!("Responding with Error: {}", e);
                        e
                    }),
            ),
            PostKind::UrlEncoded => Either::B(result(Err(DropmuttError::ContentType))),
        })
        .responder()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "dropmutt_site=info");
    env_logger::init();
    let sys = actix::System::new("dropmutt-site");

    let pool = CpuPool::new(20);

    server::new(move || {
        let state = AppState {
            fs_pool: FsPool::from_executor(pool.clone()),
        };
        App::with_state(state)
            .middleware(middleware::Logger::default())
            .resource("/multipart", |r| r.method(http::Method::POST).with2(upload))
            .handler(
                "/static",
                fs::StaticFiles::with_pool("static", pool.clone()),
            )
    }).bind("127.0.0.1:8080")
        .unwrap()
        .start();

    info!("Starting server on 127.0.0.1:8080");
    sys.run();
}
