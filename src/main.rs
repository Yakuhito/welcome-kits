use axum::extract::State;
use axum::{routing::get, Json, Router};
use chia::protocol::Bytes32;
use std::env;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::RwLock;
use tokio::time;

#[derive(Clone, Debug)]
struct ActiveOffer {
    coin_ids: Vec<Bytes32>,
    offer: String,
    message_id: String,
}

#[derive(Debug)]
struct WalletState {
    funds: u64,
    active_offers: Vec<ActiveOffer>,
}

async fn handle_root() -> &'static str {
    "Hello, Chia!"
}

async fn handle_haz_funds(
    State(state): State<Arc<RwLock<WalletState>>>,
) -> (axum::http::StatusCode, &'static str) {
    let wallet = state.read().await;
    if wallet.funds > 4_200_000_000_000 {
        (axum::http::StatusCode::OK, "OK")
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "LOW FUNDS")
    }
}

async fn handle_offers(State(state): State<Arc<RwLock<WalletState>>>) -> Json<u64> {
    let wallet = state.read().await;
    Json(wallet.active_offers.len() as u64)
}

async fn refresh_wallet(_startup: bool, state: Arc<RwLock<WalletState>>, _mnemonic: &str) {
    println!(
        "[{}] Refreshing wallet...",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    let mut wallet = state.write().await;
    wallet.funds = 1337;
}

async fn scheduled_task(state: Arc<RwLock<WalletState>>, mnemonic: String) {
    let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes
    interval.tick().await;
    loop {
        interval.tick().await;
        refresh_wallet(false, state.clone(), &mnemonic).await;
    }
}

async fn startup_task(state: Arc<RwLock<WalletState>>, mnemonic: String) {
    println!(
        "[{}] Starting up...",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    refresh_wallet(true, state, &mnemonic).await;
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

    dotenv::dotenv().ok();

    let mnemonic = env::var("MNEMONIC").expect("MNEMONIC not set");

    let wallet_state = Arc::new(RwLock::new(WalletState {
        funds: 0,
        active_offers: Vec::new(),
    }));

    startup_task(wallet_state.clone(), mnemonic.clone()).await;

    let app = Router::new()
        .route("/", get(handle_root))
        .route("/haz-funds", get(handle_haz_funds))
        .route("/offers", get(handle_offers))
        .with_state(wallet_state.clone());

    tokio::spawn(scheduled_task(wallet_state, mnemonic));

    let addr = get_addr();

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
