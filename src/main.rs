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
#[macro_use]
extern crate log;
extern crate mime;
extern crate r2d2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_urlencoded;

use std::env;

use actix::prelude::*;
use actix_web::{fs, http, server, App, AsyncResponder, Form, HttpMessage, HttpRequest,
                HttpResponse, State,
                middleware::{self,
                             identity::{CookieIdentityPolicy, IdentityService, RequestIdentity}}};
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool}};
use dotenv::dotenv;
use futures::{Future, future::{result, Either}};
use futures_cpupool::CpuPool;
use futures_fs::FsPool;

mod db;
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
    db: Addr<Syn, db::DbActor>,
    pool: CpuPool,
    signup_enabled: bool,
}

fn upload(
    req: HttpRequest<AppState>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
    let id = req.identity()
        .ok_or(DropmuttError::Auth)
        .map(|s| s.to_owned());

    result(id)
        .and_then(move |_| {
            result(post_kind(&req)).and_then(move |upload_kind| match upload_kind {
                PostKind::Multipart => Either::A(
                    do_multipart_handling(
                        req.multipart(),
                        FsPool::from_executor(state.pool.clone()),
                    ).map(|m: MultipartForm| {
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
    form: Form<AuthForm>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
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

            HttpResponse::Ok().finish()
        })
        .responder()
}

fn signup(
    mut req: HttpRequest<AppState>,
    form: Form<AuthForm>,
    state: State<AppState>,
) -> Box<Future<Item = HttpResponse, Error = DropmuttError>> {
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

                HttpResponse::Ok().finish()
            })
    })
        .responder()
}

fn prepare_connection() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Please provide a database url");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder().build(manager).unwrap()
}

fn logout(mut req: HttpRequest<AppState>) -> HttpResponse {
    req.forget();

    HttpResponse::Ok().finish()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "dropmutt_site=info");
    env_logger::init();
    let sys = actix::System::new("dropmutt-site");

    let db = SyncArbiter::start(3, move || db::DbActor::new(prepare_connection()));

    let pool = CpuPool::new(20);

    server::new(move || {
        let state = AppState {
            pool: pool.clone(),
            db: db.clone(),
            signup_enabled: true,
        };
        App::with_state(state)
            .middleware(middleware::Logger::default())
            .middleware(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 80])
                    .name("auth-cookie")
                    .secure(true),
            ))
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
                r.method(http::Method::GET).with(logout)
            })
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
