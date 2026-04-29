mod db;
mod domain;
mod entities;
mod memory;
mod openai;
mod routes;

use openai::OpenAiClient;
use poem::{EndpointExt, Server, listener::TcpListener, middleware::Cors};
use sea_orm::DatabaseConnection;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub openai: OpenAiClient,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    load_env();

    let db = db::connect_database()
        .await
        .expect("failed to connect to PrimerLab Postgres; run `docker compose up -d db`");
    db::init_database(&db)
        .await
        .expect("failed to initialize PrimerLab database schema and extensions");
    db::seed_demo_student(&db)
        .await
        .expect("failed to seed demo student");

    let state = AppState {
        db,
        openai: OpenAiClient::from_env(),
    };
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    let app = routes::api_routes().with(Cors::new()).data(state);

    println!("primerlab-api listening on http://{bind_addr}");
    Server::new(TcpListener::bind(bind_addr)).run(app).await
}

fn load_env() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env"));
    let _ = dotenvy::dotenv();
}
