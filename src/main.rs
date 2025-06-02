use axum::{Router, routing::{get, post}};
use axum_server::bind;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use tracing_subscriber;

mod routes;
use routes::auth::register;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Using database URL: {}", database_url);

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let app = Router::new()
        .route("/", get(root))
        .route("/users/register", post(register))
        .with_state(pool); // Add the pool to the app state

    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse::<u16>().unwrap()));
    println!("listening on http://{}", addr);

    bind(addr).serve(app.into_make_service()).await.unwrap();
}

async fn root() -> &'static str {
    "mtaalink is running!"
}
