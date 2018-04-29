extern crate actix;
extern crate actix_web;
extern crate bcrypt;
extern crate bytes;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate futures_fs;
extern crate http as h;
extern crate image;
#[macro_use]
extern crate log;
extern crate mime;
extern crate mime_guess;
extern crate r2d2;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_urlencoded;

use std::env;

use actix::prelude::*;
use actix_web::{fs, http, server, App, AsyncResponder, HttpMessage, HttpRequest, HttpResponse,
                Json, Query, State,
                middleware::{self, cors::Cors,
                             identity::{CookieIdentityPolicy, IdentityService, RequestIdentity}}};
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool}};
use dotenv::dotenv;
use futures::{Future, Stream, future::{result, Either}, stream::futures_unordered};
use futures_cpupool::CpuPool;

mod db;
mod error;
mod image_processor;
mod path_generator;
mod models;
mod schema;
mod upload;

use self::error::DropmuttError;
use self::path_generator::PathGenerator;
use self::upload::{do_multipart_handling, post_kind, MultipartContent, MultipartForm, PostKind};

#[derive(Debug, Deserialize, Serialize)]
pub struct Success {
    message: String,
}

#[derive(Clone)]
pub struct AppState {
    app_path: String,
    path_generator: PathGenerator,
    img_processor: Addr<Syn, image_processor::ImageProcessor>,
    db: Addr<Syn, db::DbActor>,
    pool: CpuPool,
    signup_enabled: bool,
}

fn process_image(
    db: Addr<Syn, db::DbActor>,
    ip: Addr<Syn, image_processor::ImageProcessor>,
    token: String,
    file: models::File,
) -> impl Future<Item = models::Image, Error = DropmuttError> {
    ip.send(image_processor::ProcessImage(file))
        .then(|res| match res {
            Ok(res) => res,
            Err(e) => Err(e.into()),
        })
        .and_then(move |proc_res| {
            db.clone()
                .send(db::StoreProcessedImage(token.clone(), proc_res))
                .then(|res| match res {
                    Ok(res) => res,
                    Err(e) => Err(e.into()),
                })
        })
}

fn upload(
    req: HttpRequest<AppState>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    let id = req.identity()
        .ok_or(DropmuttError::Auth)
        .map(|s| s.to_owned());

    result(id)
        .and_then(move |token| {
            result(post_kind(&req)).and_then(move |upload_kind| match upload_kind {
                PostKind::Multipart => Either::A(
                    do_multipart_handling(
                        req.multipart(),
                        state.pool.clone(),
                        state.path_generator.clone(),
                    ).and_then(move |m: MultipartForm| {
                        futures_unordered(m.iter().filter_map(|(_, v)| match *v {
                            MultipartContent::File {
                                ref filename,
                                ref stored_as,
                            } => {
                                let _ = filename;
                                let img_p = state.img_processor.clone();
                                let db = state.db.clone();
                                let token = token.clone();

                                Some(
                                    state
                                        .db
                                        .clone()
                                        .send(db::StoreImage(token.clone(), stored_as.to_owned()))
                                        .then(|res| match res {
                                            Ok(res) => res,
                                            Err(e) => Err(e.into()),
                                        })
                                        .and_then(move |(_, file)| {
                                            process_image(db, img_p, token, file)
                                        }),
                                )
                            }
                            _ => None,
                        })).fold(0, |acc, _| Ok(acc + 1) as Result<_, DropmuttError>)
                            .map(|total| {
                                info!("Stored {} files", total);
                                info!("Responding with {:?}", m);
                                HttpResponse::Created().json(m)
                            })
                    })
                        .map_err(|e| {
                            info!("Responding with Error: {}", e);
                            e
                        }),
                ),
                PostKind::UrlEncoded => Either::B(result(Err(DropmuttError::ContentType))),
            })
        })
        .responder()
}

#[derive(Deserialize, Serialize)]
struct AuthForm {
    username: String,
    password: String,
}

fn login(
    mut req: HttpRequest<AppState>,
    form: Json<AuthForm>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    info!("login");
    let form = form.into_inner();

    let pool = state.pool.clone();

    state
        .db
        .send(db::LookupUser(form.username.clone()))
        .then(|res| match res {
            Ok(res) => res,
            Err(e) => Err(e.into()),
        })
        .and_then(move |queried_user| {
            pool.spawn_fn(move || queried_user.verify(&form.password))
                .from_err()
        })
        .map(move |user| {
            req.remember(user.token_str());

            HttpResponse::Ok().json(Success {
                message: "Logged in!".to_owned(),
            })
        })
        .responder()
}

fn signup(
    mut req: HttpRequest<AppState>,
    form: Json<AuthForm>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    info!("signup");
    let form = form.into_inner();

    result(if state.signup_enabled {
        Ok(())
    } else {
        Err(DropmuttError::SignupClosed)
    }).and_then(move |_| {
        state
            .db
            .send(db::CreateUser {
                username: form.username,
                password: form.password,
            })
            .then(|res| match res {
                Ok(res) => res,
                Err(e) => Err(e.into()),
            })
            .map(move |user| {
                req.remember(user.token_str());

                HttpResponse::Created().json(Success {
                    message: "Created account!".to_owned(),
                })
            })
    })
        .responder()
}

fn logout(mut req: HttpRequest<AppState>) -> HttpResponse {
    info!("logout user {:?}", req.identity());
    req.forget();

    HttpResponse::Ok().json(Success {
        message: "Logged out!".to_owned(),
    })
}

fn check_auth(req: HttpRequest<AppState>) -> Result<HttpResponse, DropmuttError> {
    req.identity()
        .map(|_| {
            HttpResponse::Ok().json(Success {
                message: "Yup!".to_owned(),
            })
        })
        .ok_or(DropmuttError::Auth)
}

fn serve_app(state: State<AppState>) -> Result<fs::NamedFile, DropmuttError> {
    fs::NamedFile::open(&state.app_path)
        .map(|nf| nf.set_cpu_pool(state.pool.clone()))
        .map_err(From::from)
}

#[derive(Debug, Deserialize, Serialize)]
struct ImageQuery {
    count: i64,
    id: Option<i32>,
}

fn fetch_images(
    query: Query<ImageQuery>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    let q = query.into_inner();
    state
        .db
        .send(db::FetchImages {
            count: q.count,
            before_id: q.id,
        })
        .then(|res| match res {
            Ok(res) => res,
            Err(e) => Err(e.into()),
        })
        .map(|images| HttpResponse::Ok().json(images))
        .from_err()
        .responder()
}

fn prepare_connection() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Please provide a database url");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder().build(manager).unwrap()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "dropmutt_site=info");
    env_logger::init();
    let sys = actix::System::new("dropmutt-site");

    let db = SyncArbiter::start(3, move || db::DbActor::new(prepare_connection()));

    let img_processor = SyncArbiter::start(3, move || image_processor::ImageProcessor);

    let pool = CpuPool::new(20);

    server::new(move || {
        let state = AppState {
            app_path: "static/index.html".to_owned(),
            path_generator: PathGenerator::with_start_position(0),
            pool: pool.clone(),
            img_processor: img_processor.clone(),
            db: db.clone(),
            signup_enabled: true,
        };
        App::with_state(state)
            .middleware(middleware::Logger::default())
            .middleware(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 80])
                    .name("auth-cookie")
                    .secure(false),
            ))
            .configure(|app| {
                Cors::for_app(app)
                    .allowed_origin("http://localhost:8000")
                    .supports_credentials()
                    .resource("/api/v1/upload", |r| {
                        r.method(http::Method::POST).with2(upload)
                    })
                    .resource("/api/v1/login", |r| {
                        r.method(http::Method::POST).with3(login)
                    })
                    .resource("/api/v1/signup", |r| {
                        r.method(http::Method::POST).with3(signup)
                    })
                    .resource("/api/v1/logout", |r| {
                        r.method(http::Method::DELETE).with(logout)
                    })
                    .resource("/api/v1/check-auth", |r| {
                        r.method(http::Method::GET).with(check_auth)
                    })
                    .resource("/api/v1/images", |r| {
                        r.method(http::Method::GET).with2(fetch_images)
                    })
                    .register()
            })
            .resource("/", |r| r.method(http::Method::GET).with(serve_app))
            .handler(
                "/static",
                fs::StaticFiles::with_pool("static", pool.clone()),
            )
            .handler(
                "/uploads",
                fs::StaticFiles::with_pool("uploads", pool.clone()),
            )
    }).bind("127.0.0.1:8080")
        .unwrap()
        .start();

    info!("Starting server on 127.0.0.1:8080");
    sys.run();
}
