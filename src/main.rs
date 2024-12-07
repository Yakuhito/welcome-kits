use axum::extract::State;
use axum::{routing::get, Json, Router};
use bip39::Mnemonic;
use chia::protocol::Coin;
use chia::puzzles::standard::StandardArgs;
use chia::puzzles::DeriveSynthetic;
use chia_bls::{master_to_wallet_unhardened, SecretKey};
use chia_wallet_sdk::{decode_puzzle_hash, encode_address, StandardLayer};
use serde::Deserialize;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::RwLock;
use tokio::time;

#[derive(Clone, Debug)]
struct ActiveOffer {
    offer: String,
    message_id: String,
}

#[derive(Debug)]
struct WalletState {
    funds: u64,
    active_offers: Vec<ActiveOffer>,
}

impl WalletState {
    fn to_json(&self) -> serde_json::Value {
        let offers: serde_json::Map<String, serde_json::Value> = self
            .active_offers
            .iter()
            .map(|offer| {
                (
                    offer.message_id.clone(),
                    serde_json::Value::String(offer.offer.clone()),
                )
            })
            .collect();

        serde_json::json!({
            "active_offers": offers
        })
    }
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

async fn handle_offers(State(state): State<Arc<RwLock<WalletState>>>) -> Json<serde_json::Value> {
    let wallet = state.read().await;
    Json(wallet.to_json())
}

#[derive(Deserialize)]
struct CoinRecord {
    coin: DeserializableCoin,
    // We don't need to define the other fields since we won't use them
    #[serde(skip)]
    _ignored: serde::de::IgnoredAny,
}

#[derive(Deserialize)]
struct CoinRecordsResponse {
    coin_records: Vec<CoinRecord>,
}

#[derive(Deserialize)]
struct DeserializableCoin {
    amount: u64,
    parent_coin_info: String,
    puzzle_hash: String,
}

async fn refresh_wallet(startup: bool, state: Arc<RwLock<WalletState>>, mnemonic: &str) {
    println!(
        "[{}] Refreshing wallet...",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    let mnemonic = Mnemonic::from_str(mnemonic).unwrap();
    let seed = mnemonic.to_seed("");
    let sk = master_to_wallet_unhardened(&SecretKey::from_seed(&seed), 0).derive_synthetic();
    let pk = sk.public_key();

    let layer = StandardLayer::new(pk);
    let wallet_puzzle_hash = StandardArgs::curry_tree_hash(pk);
    if startup {
        let address = encode_address(wallet_puzzle_hash.into(), "xch");
        println!(
            "[{}] Wallet address: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            address.unwrap()
        );
    }

    let response = reqwest::Client::new()
        .post("https://api.coinset.org/get_coin_records_by_puzzle_hash")
        .json(&serde_json::json!({
            "puzzle_hash": wallet_puzzle_hash.to_string(),
            "start_height": 63000000,
            "end_height": 0,
            "include_spent_coins": false
        }))
        .send()
        .await
        .expect("Failed to send request")
        .json::<CoinRecordsResponse>()
        .await
        .expect("Failed to parse response");

    let coins: Vec<Coin> = response
        .coin_records
        .into_iter()
        .map(|record| {
            Coin::new(
                decode_puzzle_hash(&record.coin.parent_coin_info)
                    .unwrap()
                    .into(),
                decode_puzzle_hash(&record.coin.puzzle_hash).unwrap().into(),
                record.coin.amount,
            )
        })
        .collect();

    println!("Coins: {:?}", coins);

    let mut wallet = state.write().await;
    wallet.funds = coins.iter().map(|c| c.amount).sum();
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
