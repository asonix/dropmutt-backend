extern crate actix;
extern crate actix_multipart;
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

use std::{env, path::PathBuf};

use actix::prelude::*;
use actix_web::{fs, http, server, App, AsyncResponder, HttpMessage, HttpRequest, HttpResponse,
                Json, Path, Query, State,
                middleware::{self, cors::Cors,
                             identity::{CookieIdentityPolicy, IdentityService, RequestIdentity}}};
use actix_multipart::*;
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool}};
use dotenv::dotenv;
use futures::{Future, future::{result, Either}};
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
use self::upload::{post_kind, PostKind};

#[derive(Debug, Deserialize, Serialize)]
pub struct Success {
    message: String,
}

#[derive(Clone)]
pub struct AppState {
    app_path: String,
    db: Addr<Syn, db::DbActor>,
    form: Form,
    img_processor: Addr<Syn, image_processor::ImageProcessor>,
    pool: CpuPool,
    signup_enabled: bool,
}

pub struct ImageForm {
    file_upload: (String, PathBuf),
    description: String,
    alternate_text: String,
    gallery_name: String,
}

impl ImageForm {
    fn from_value(v: Value) -> Option<Self> {
        let mut v = match v {
            Value::Map(hm) => hm,
            _ => return None,
        };

        let file_upload = v.remove("file-upload").and_then(|v| match v {
            Value::File(filename, path) => Some((filename, path)),
            _ => None,
        })?;

        let description = v.remove("description").and_then(|v| match v {
            Value::Text(string) => Some(string),
            _ => None,
        })?;

        let alternate_text = v.remove("alternate-text").and_then(|v| match v {
            Value::Text(string) => Some(string),
            _ => None,
        })?;

        let gallery_name = v.remove("gallery-name").and_then(|v| match v {
            Value::Text(string) => Some(string),
            _ => None,
        })?;

        Some(ImageForm {
            file_upload,
            description,
            alternate_text,
            gallery_name,
        })
    }
}

fn process_image(
    db: Addr<Syn, db::DbActor>,
    ip: Addr<Syn, image_processor::ImageProcessor>,
    unprocessed_image: models::UnprocessedImage,
    file: models::File,
) -> impl Future<Item = models::Image, Error = DropmuttError> {
    ip.send(image_processor::ProcessImage(file))
        .then(|res| match res {
            Ok(res) => res,
            Err(e) => Err(e.into()),
        })
        .and_then(move |proc_res| {
            db.clone()
                .send(db::StoreProcessedImage(unprocessed_image, proc_res))
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
                    handle_upload(req.multipart(), state.form.clone())
                        .map_err(DropmuttError::Upload)
                        .and_then(move |m: MultipartForm| {
                            let ImageForm {
                                file_upload,
                                description,
                                alternate_text,
                                gallery_name,
                            } = match ImageForm::from_value(consolidate(m)) {
                                Some(imgform) => imgform,
                                None => return Either::B(result(Err(DropmuttError::MissingFields))),
                            };

                            let (_, file_path) = file_upload;
                            let img_p = state.img_processor.clone();
                            let db = state.db.clone();

                            let fut = state
                                .db
                                .clone()
                                .send(db::StoreImage {
                                    user_token: token.clone(),
                                    file_path,
                                    gallery_name,
                                    description,
                                    alternate_text,
                                })
                                .then(|res| match res {
                                    Ok(res) => res,
                                    Err(e) => Err(e.into()),
                                })
                                .and_then(move |(unprocessed_image, file)| {
                                    process_image(db, img_p, unprocessed_image, file)
                                })
                                .map(move |_| HttpResponse::Created().json(r#"{"msg":"success"}"#));

                            Either::A(fut)
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

fn fetch_images_by_gallery(
    path: Path<String>,
    query: Query<ImageQuery>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    let gallery = path.into_inner();
    let q = query.into_inner();

    state
        .db
        .send(db::FetchImagesInGallery {
            gallery,
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
    ::std::env::set_var("RUST_LOG", "actix_multipart,dropmutt_site=info");
    env_logger::init();
    let sys = actix::System::new("dropmutt-site");

    let db = SyncArbiter::start(3, move || db::DbActor::new(prepare_connection()));

    let img_processor = SyncArbiter::start(3, move || image_processor::ImageProcessor);

    let pool = CpuPool::new(20);

    let form = Form::from_executor(pool.clone())
        .field("file-upload", Field::file(PathGenerator::new("uploads", 0)))
        .field("description", Field::text())
        .field("alternate-text", Field::text())
        .field("gallery-name", Field::text());

    server::new(move || {
        let state = AppState {
            app_path: "static/index.html".to_owned(),
            db: db.clone(),
            form: form.clone(),
            img_processor: img_processor.clone(),
            pool: pool.clone(),
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
                    .resource("/api/v1/images/{gallery}", |r| {
                        r.method(http::Method::GET).with3(fetch_images_by_gallery)
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
