mod db;
mod domain;
mod entities;
mod memory;
mod openai;
mod report_card;
mod request_logging;
mod routes;
mod voiceover_store;

use openai::OpenAiClient;
use poem::{EndpointExt, Server, listener::TcpListener, middleware::Cors};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub openai: OpenAiClient,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    load_env();

    let db = db::connect_database()
        .await
        .expect("failed to connect to Primer Postgres; run `docker compose up -d db`");
    db::init_database(&db)
        .await
        .expect("failed to initialize Primer database schema and extensions");

    let state = AppState {
        db: Arc::new(db),
        openai: OpenAiClient::from_env(),
    };
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    let app = routes::api_routes()
        .with(Cors::new())
        .around(request_logging::log_request_to_stdout)
        .data(state);

    println!("primerlab-api listening on http://{bind_addr}");
    println!("primerlab-api request logging enabled on stdout");
    Server::new(TcpListener::bind(bind_addr)).run(app).await
}

fn load_env() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env"));
    let _ = dotenvy::dotenv();
}
