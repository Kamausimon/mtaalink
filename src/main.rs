use axum::{
    Router,
    routing::{get,},
};
use axum_server::bind;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower_http::trace::TraceLayer;

mod extractors;
mod utils;
mod routes;
use routes::auth::auth_routes;
use routes::dashboard::dashboard;



#[tokio::main]
async fn main() {
    dotenv().ok();
       tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Using database URL: {}", database_url);

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");
    println!("Database connection pool created successfully");

    let app = Router::new()
        .nest("/auth", auth_routes(pool.clone())). // Mount the auth routes
        route("/dashboard", get(dashboard)) // Add the dashboard route
        .layer(TraceLayer::new_for_http()) // ✅ This logs all requests
        .route("/", get(root));
     

    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse::<u16>().unwrap()));
    println!("listening on http://{}", addr);

    bind(addr).serve(app.into_make_service()).await.unwrap();
}

async fn root() -> &'static str {
    "mtaalink is running!"
}
