use axum::{Router, routing::get};
use axum_server::bind;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod extractors;
mod routes;
mod utils;

use routes::auth::auth_routes;
use routes::businesses::businesses_routes;
use routes::clients::client_routes;
use routes::dashboard::dashboard;
use routes::favorites::favorites_routes;
use routes::messages::messages_routes;
use routes::reviews::reviews_routes;
use routes::service_providers::service_providers_routes;
use routes::categories::category_routes;
use routes::bookings::booking_routes;
use routes::admin::admin_routes;
use utils::attachments::attachments_routes;


#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Using database URL: {}", database_url);

    // Enable CORS for all origins
    let cors_layer = CorsLayer::permissive();

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");
    println!("Database connection pool created successfully");

    let app = Router::new()
        .nest("/auth", auth_routes(pool.clone())) // Mount the auth routes
        .route("/dashboard", get(dashboard)) // Add the dashboard route
        .nest("/service_providers", service_providers_routes(pool.clone())) // Mount the service providers routes
        .nest("/businesses", businesses_routes(pool.clone())) // Mount the businesses routes
        .nest("/clients", client_routes(pool.clone())) // Mount the clients routes
        .nest("/reviews", reviews_routes(pool.clone())) // Mount the reviews routes
        .nest("/favorites", favorites_routes(pool.clone())) // Mount the favorites routes
        .nest("/messages", messages_routes(pool.clone())) // Mount the messages routes
        .nest("/categories", category_routes(pool.clone())) // Mount the categories routes
        .nest("/bookings", booking_routes(pool.clone())) // Mount the bookings routes
        .nest("/admin", admin_routes(pool.clone())) // Mount the admin routes
        .nest("/attachments", attachments_routes(pool.clone())) // Mount the attachments routes
        .nest_service("/uploads", ServeDir::new("uploads")) // Serve static files from the uploads directory
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
