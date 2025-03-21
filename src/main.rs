use axum::extract::State;
use axum::{routing::get, Json, Router};
use bip39::Mnemonic;
use chia::clvm_traits::{FromClvm, ToClvm};
use chia::clvm_utils::{CurriedProgram, ToTreeHash};
use chia::protocol::{Bytes, Bytes32, Coin, SpendBundle};
use chia::puzzles::offer::SETTLEMENT_PAYMENTS_PUZZLE_HASH;
use chia::puzzles::singleton::SingletonStruct;
use chia::puzzles::standard::StandardArgs;
use chia::puzzles::DeriveSynthetic;
use chia_bls::{master_to_wallet_unhardened, sign, SecretKey, Signature};
use chia_wallet_sdk::{
    decode_address, decode_puzzle_hash, encode_address, select_coins, AggSigConstants, Conditions,
    Offer, RequiredSignature, SpendContext, StandardLayer, MAINNET_CONSTANTS,
};
use clvmr::serde::node_from_bytes;
use config::{
    ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS, GET_COIN_RECORDS_BY_PUZZLE_HASH_URL,
    GET_WARP_MESSAGES_URL, LOW_FUNDS_THRESHOLD, MAX_MOJOS_IN_ELIGIBLE_WALLETS, MESSAGE_COIN_MOD,
    PORTAL_LAUNCHER_ID, WELCOME_KIT_AMOUNT,
};
use serde::Deserialize;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::RwLock;
use tokio::time;

mod config;

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
    if wallet.funds > LOW_FUNDS_THRESHOLD {
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
    spent: bool,
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

async fn get_unspent_coins(puzzle_hash: Bytes32) -> Vec<Coin> {
    let response = reqwest::Client::new()
        .post(GET_COIN_RECORDS_BY_PUZZLE_HASH_URL)
        .json(&serde_json::json!({
            "puzzle_hash": puzzle_hash.to_string(),
            "include_spent_coins": false
        }))
        .send()
        .await
        .expect("Failed to send request")
        .json::<CoinRecordsResponse>()
        .await
        .expect("Coinset down");

    response
        .coin_records
        .into_iter()
        .filter(|record| !record.spent)
        .map(|record| {
            Coin::new(
                decode_puzzle_hash(&record.coin.parent_coin_info)
                    .unwrap()
                    .into(),
                decode_puzzle_hash(&record.coin.puzzle_hash).unwrap().into(),
                record.coin.amount,
            )
        })
        .collect()
}

#[derive(Deserialize)]
struct ParsedMessage {
    token_symbol: String,
    amount_mojo: u64,
    receiver: String,
}

#[derive(Deserialize)]
struct PendingMessage {
    nonce: String,
    source_chain: String,
    source: String,
    destination: String,
    contents: Vec<String>,
    parsed: ParsedMessage,
}

async fn get_pending_messages() -> Vec<PendingMessage> {
    reqwest::Client::new()
        .get(GET_WARP_MESSAGES_URL)
        .send()
        .await
        .expect("Failed to fetch pending messages")
        .json::<Vec<PendingMessage>>()
        .await
        .expect("Failed to parse pending messages")
}

#[derive(Debug, Clone, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(list)]
pub struct UniqueMessageInfo {
    pub source_chain: Bytes,
    #[clvm(rest)]
    pub nonce: Bytes32,
}

#[derive(Debug, Clone, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct MessageCoinFirstCurryArgs {
    pub portal_singleton_struct: SingletonStruct,
}

#[derive(Debug, Clone, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct MessageCoinSecondCurryArgs {
    pub unique_info: UniqueMessageInfo,
    pub source: Bytes,
    pub destination: Bytes32,
    pub message_hash: Bytes32,
}

fn generate_offer(msg: PendingMessage, selected_coins: Vec<Coin>, sk: &SecretKey) -> String {
    let ctx = &mut SpendContext::new();

    // first, determine the puzzle hash of the message coin
    let message_mod = node_from_bytes(&mut ctx.allocator, &MESSAGE_COIN_MOD).unwrap();
    let first_curry = CurriedProgram {
        program: message_mod,
        args: MessageCoinFirstCurryArgs {
            portal_singleton_struct: SingletonStruct::new(PORTAL_LAUNCHER_ID.into()),
        },
    }
    .to_clvm(&mut ctx.allocator)
    .unwrap();
    let contents: Vec<Bytes32> = msg
        .contents
        .iter()
        .map(|c| decode_puzzle_hash(c).unwrap().into())
        .collect();
    let second_curry = CurriedProgram {
        program: first_curry,
        args: MessageCoinSecondCurryArgs {
            unique_info: UniqueMessageInfo {
                source_chain: msg.source_chain.as_bytes().to_vec().into(),
                nonce: decode_puzzle_hash(&msg.nonce).unwrap().into(),
            },
            source: hex::decode(&msg.source).unwrap().into(),
            destination: decode_puzzle_hash(&msg.destination).unwrap().into(),
            message_hash: contents.tree_hash().into(),
        },
    }
    .to_clvm(&mut ctx.allocator)
    .unwrap();
    let message_coin_puzzle_hash = ctx.tree_hash(second_curry);

    let total_coin_amount = selected_coins.iter().map(|c| c.amount).sum::<u64>();
    let offer_amount = if msg.parsed.token_symbol == "XCH" {
        1
    } else {
        msg.parsed.amount_mojo
    };

    let pk = sk.public_key();
    let layer = StandardLayer::new(pk);

    let lead_coin = selected_coins[0];
    for other_coin in selected_coins[1..].iter() {
        layer
            .clone()
            .spend(
                ctx,
                *other_coin,
                Conditions::new().assert_concurrent_spend(lead_coin.coin_id()),
            )
            .unwrap();
    }

    let mut lead_coin_conditions = Conditions::new()
        .create_coin(SETTLEMENT_PAYMENTS_PUZZLE_HASH.into(), offer_amount, vec![])
        .create_coin(
            decode_address(&msg.parsed.receiver).unwrap().0.into(),
            WELCOME_KIT_AMOUNT,
            vec![],
        )
        .assert_concurrent_puzzle(message_coin_puzzle_hash.into());

    let change = total_coin_amount - offer_amount - WELCOME_KIT_AMOUNT;
    if change > 42 * WELCOME_KIT_AMOUNT {
        let change1 = change / 2 - 4;
        let change2 = change - change1;
        lead_coin_conditions = lead_coin_conditions
            .create_coin(lead_coin.puzzle_hash, change1, vec![])
            .create_coin(lead_coin.puzzle_hash, change2, vec![]);
    } else {
        lead_coin_conditions =
            lead_coin_conditions.create_coin(lead_coin.puzzle_hash, change, vec![]);
    }

    layer.spend(ctx, lead_coin, lead_coin_conditions).unwrap();

    let coin_spends = ctx.take();
    let required_sig = RequiredSignature::from_coin_spends(
        &mut ctx.allocator,
        &coin_spends,
        &AggSigConstants::new(MAINNET_CONSTANTS.agg_sig_me_additional_data),
    )
    .unwrap();
    let mut sigs: Vec<Signature> = vec![];
    for req in required_sig {
        sigs.push(sign(sk, req.final_message()));
    }

    let offer = Offer::new(SpendBundle {
        coin_spends,
        aggregated_signature: sigs.into_iter().reduce(|a, b| a + &b).unwrap(),
    });

    offer.encode().unwrap()
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

    let wallet_puzzle_hash = StandardArgs::curry_tree_hash(pk);
    if startup {
        let address = encode_address(wallet_puzzle_hash.into(), "xch");
        println!(
            "[{}] Wallet address: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            address.unwrap()
        );
    }

    let wallet_coins = get_unspent_coins(wallet_puzzle_hash.into()).await;

    let mut pending_messages_to_process: Vec<PendingMessage> = Vec::new();
    let pending_messages = get_pending_messages().await;

    for message in pending_messages {
        let mut min_amount: Option<u64> = None;
        for (symbol, assoc_min_amount) in ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS {
            if message.parsed.token_symbol == symbol {
                min_amount = Some(assoc_min_amount);
                break;
            }
        }

        if let Some(min_amount) = min_amount {
            if message.parsed.amount_mojo < min_amount {
                continue;
            }
        } else {
            continue;
        }

        // We know:
        //  - The message is to XCH (from URL)
        //  - The message is pending (relay not completed - also URL)
        //  - An eligible token is being transferred
        //  - The amount is greater than the minimum amount for that token
        // So check that the wallet is not super well-funded and that's it.

        let recipient: Bytes32 = decode_address(&message.parsed.receiver).unwrap().0.into();
        let recipient_coins = get_unspent_coins(recipient).await;
        let recipient_funds = recipient_coins.iter().map(|c| c.amount).sum::<u64>();
        if recipient_funds > MAX_MOJOS_IN_ELIGIBLE_WALLETS {
            continue;
        }

        pending_messages_to_process.push(message);
    }

    let mut new_offers: Vec<ActiveOffer> = Vec::new();
    let mut coins_to_select_from: Vec<Coin> = wallet_coins.clone();

    for pending_message in pending_messages_to_process {
        let Ok(selected_coins) = select_coins(
            coins_to_select_from.clone(),
            (if pending_message.parsed.token_symbol == "XCH" {
                1 + WELCOME_KIT_AMOUNT
            } else {
                pending_message.parsed.amount_mojo + WELCOME_KIT_AMOUNT
            })
            .into(),
        ) else {
            println!(
                "[{}] Not enough funds to generate offer {}-{}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                pending_message.source_chain,
                pending_message.nonce
            );
            break;
        };
        coins_to_select_from.retain(|c| !selected_coins.contains(c));

        new_offers.push(ActiveOffer {
            message_id: pending_message.source_chain.clone() + "-" + &pending_message.nonce,
            offer: generate_offer(pending_message, selected_coins, &sk),
        });
    }

    let mut wallet = state.write().await;
    wallet.funds = wallet_coins.iter().map(|c| c.amount).sum();
    wallet.active_offers = new_offers;

    println!(
        "[{}] Done refreshing wallet with {} coins - total funds: {} XCH; active offers: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        wallet_coins.len(),
        wallet.funds as f64 / 1_000_000_000_000.0,
        wallet.active_offers.len()
    );
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
