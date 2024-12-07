pub const GET_COIN_RECORDS_BY_PUZZLE_HASH_URL: &str =
    "https://api.coinset.org/get_coin_records_by_puzzle_hash";
pub const GET_WARP_MESSAGES_URL: &str =
    "https://watcher-api.warp.green/messages?limit=100&destination_chain=xch&status=sent&order_by=id&sort=asc";

pub const WELCOME_KIT_AMOUNT: u64 = 420_000_000; // 0.00042 XCH
pub const LOW_FUNDS_THRESHOLD: u64 = 4_200_000_000_000; // 0.42 XCH
pub const WALLET_START_HEIGHT: u64 = 6_300_000;

// for mainnet
// pub const MAX_MOJOS_IN_ELIGIBLE_WALLETS: u64 = 10_000_000_000; // 0.01 XCH
// pub const ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS: [(&str, u64); 4] = [
//     ("milliETH", 10_000),     // 10 milliETH, or 0.01 ETH
//     ("USDC", 42_000),         // 40 USDC
//     ("USDT", 42_000),         // 42 USDT
//     ("XCH", 100_000_000_000), // 0.1 XCH
// ];

// for testing
pub const MAX_MOJOS_IN_ELIGIBLE_WALLETS: u64 = 100_000_000_000_000; // 100 XCH
pub const ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS: [(&str, u64); 4] = [
    ("milliETH", 1), // 0.001 milliETH
    ("USDC", 1),     // 0.001 USDC
    ("USDT", 1),     // 0.001 USDT
    ("XCH", 1),      // 1 mojo
];
