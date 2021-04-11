use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use anyhow::*;
use bb8::*;
use bb8_tiberius::*;
use serde::Deserialize;
use std::env;
use tiberius::{AuthMethod, Config};
extern crate config;

#[derive(Deserialize)]
pub struct IndexQuery {
    number: i32,
}

async fn db_example(
    pool: &bb8::Pool<bb8_tiberius::ConnectionManager>,
    roundtrip_number: &i32,
) -> Result<Option<i32>, RunError<tiberius::error::Error>> {
    let mut client = match pool.get().await {
        Ok(client) => client,
        Err(e) => panic!("DB connection timeout {:?}", e),
    };
    let stream = client.query("SELECT @P1", &[roundtrip_number]).await?;
    let row = stream.into_row().await?;
    Ok(row.unwrap().get(0))
}

/// Async request handler. Ddb pool is stored in application state.
async fn index(
    web::Query(query): web::Query<IndexQuery>,
    pool: web::Data<Pool<ConnectionManager>>,
) -> Result<HttpResponse, Error> {
    match db_example(&pool, &query.number).await {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(e) => panic!("db op error: {:?}", e),
    }
}

#[derive(Deserialize, Debug)]
pub struct DBSettings {
    pub host: String,
    pub port: u16,
    pub trust_cert: bool,
    pub database: String,
    pub user: String,
    pub pw: String,
}

// inspired from https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs
impl DBSettings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name("config/default"))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(config::File::with_name(&format!("config/{}", env)).required(false))?;

        s.try_into()
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    let settings = DBSettings::new()?;
    println!("{:?}", settings);

    let mut config = Config::new();
    config.host(settings.host);
    config.port(settings.port);
    if settings.trust_cert {
        config.trust_cert();
    };
    config.database(settings.database);

    // Using SQL Server authentication.
    config.authentication(AuthMethod::sql_server(settings.user, settings.pw));

    let mgr = bb8_tiberius::ConnectionManager::new(config);
    let pool: bb8::Pool<bb8_tiberius::ConnectionManager> =
        bb8::Pool::builder().max_size(2 as u32).build(mgr).await?;

    // start http server
    HttpServer::new(move || {
        App::new()
            .data(pool.clone()) // <- store db pool in app state
            .wrap(middleware::Logger::default())
            .service(web::resource("/").route(web::get().to(index)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;
    Ok(())
}
