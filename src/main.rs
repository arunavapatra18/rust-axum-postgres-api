mod handler;
mod model;
mod route;
mod schema;

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue,
    Method
};

use std::sync::Arc;
use dotenv::dotenv;

use sqlx::{
    postgres::PgPoolOptions,
    Pool,
    Postgres
};

use route::create_router;
use tower_http::cors::{CorsLayer};

// Struct containing DB Pool
pub struct AppState {
    db: Pool<Postgres>,
}

// main()
#[tokio::main]
async fn main() {
    dotenv().ok(); // Load the .env

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("❌ Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    // CORS Middleware
    let cors = CorsLayer::new()
    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
    .allow_credentials(true)
    .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    // Create the Router and add the CORS layer
    let app = create_router(Arc::new(AppState { db: pool.clone() })).layer(cors);

    println!("🚀 Server started successfully at 127.0.0.1:8000");
    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
