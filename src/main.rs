use axum::{routing::get, Json, Router};
use std::{env, net::SocketAddr, time::Duration};
use tokio::time;

async fn handle_root() -> &'static str {
    "Hello, Chia!"
}

async fn handle_haz_funds() -> Json<&'static str> {
    Json("Haz funds :)")
}

async fn handle_offers() -> Json<&'static str> {
    Json("Offers anyone?")
}

async fn scheduled_task() {
    let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes
    loop {
        interval.tick().await;
        println!("Running scheduled task...");
    }
}

async fn startup_task() {
    println!("Running startup task...");
}

fn get_addr() -> SocketAddr {
    let host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3333);

    format!("{}:{}", host, port).parse().unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    startup_task().await;

    let app = Router::new()
        .route("/", get(handle_root))
        .route("/haz-funds", get(handle_haz_funds))
        .route("/offers", get(handle_offers));

    tokio::spawn(scheduled_task());

    let addr = get_addr();

    println!("Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
