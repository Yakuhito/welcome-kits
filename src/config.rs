use hex_literal::hex;

pub const GET_COIN_RECORDS_BY_PUZZLE_HASH_URL: &str =
    "https://api.coinset.org/get_coin_records_by_puzzle_hash";
pub const GET_WARP_MESSAGES_URL: &str =
    "https://watcher-api.warp.green/messages?limit=100&destination_chain=xch&status=sent&order_by=id&sort=asc";

pub const WELCOME_KIT_AMOUNT: u64 = 420_000_000; // 0.00042 XCH
pub const LOW_FUNDS_THRESHOLD: u64 = 420_000_000_000; // 0.42 XCH

// for mainnet
pub const MAX_MOJOS_IN_ELIGIBLE_WALLETS: u64 = 100_000_000_000; // 0.1 XCH
pub const ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS: [(&str, u64); 4] = [
    ("milliETH", 10_000),     // 10 milliETH, or 0.01 ETH
    ("USDC", 42_000),         // 42 USDC
    ("USDT", 42_000),         // 42 USDT
    ("XCH", 100_000_000_000), // 0.1 XCH
];

// for testing
// pub const MAX_MOJOS_IN_ELIGIBLE_WALLETS: u64 = 100_000_000_000_000; // 100 XCH
// pub const ELIGIBLE_SYMBOLS_AND_MINIMUM_AMOUNTS: [(&str, u64); 4] = [
//     ("milliETH", 1), // 0.001 milliETH
//     ("USDC", 1),     // 0.001 USDC
//     ("USDT", 1),     // 0.001 USDT
//     ("XCH", 1),      // 1 mojo
// ];

pub const PORTAL_LAUNCHER_ID: [u8; 32] =
    hex!("46e2bdbbcd1e372523ad4cd3c9cf4b372c389733c71bb23450f715ba5aa56d50");
pub const MESSAGE_COIN_MOD: [u8; 557] = hex!("ff02ffff01ff02ff16ffff04ff02ffff04ff05ffff04ff82017fffff04ff8202ffffff04ffff0bffff02ffff03ffff09ffff0dff82013f80ffff012080ffff0182013fffff01ff088080ff0180ffff02ffff03ffff09ffff0dff2f80ffff012080ffff012fffff01ff088080ff0180ff8201bf80ff80808080808080ffff04ffff01ffffff3d46ff473cffff02ffffa04bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459aa09dcf97a184f32623d11a73124ceb99a5709b083721e878a16d78f596718ba7b2ffa102a12871fee210fb8619291eaea194581cbd2531e4b23759d225f6806923f63222a102a8d5dd63fba471ebcb1f3e8f7c1e1879b7152a6e7298a91ce119a63400ade7c5ffff04ffff04ff18ffff04ff17ff808080ffff04ffff04ff14ffff04ffff0bff13ffff0bff5affff0bff12ffff0bff12ff6aff0980ffff0bff12ffff0bff7affff0bff12ffff0bff12ff6affff02ff1effff04ff02ffff04ff05ff8080808080ffff0bff12ffff0bff7affff0bff12ffff0bff12ff6aff1b80ffff0bff12ff6aff4a808080ff4a808080ff4a808080ffff010180ff808080ffff04ffff04ff1cffff04ff2fff808080ffff04ffff04ff10ffff04ffff0bff2fff1780ff808080ff8080808080ff02ffff03ffff07ff0580ffff01ff0bffff0102ffff02ff1effff04ff02ffff04ff09ff80808080ffff02ff1effff04ff02ffff04ff0dff8080808080ffff01ff0bffff0101ff058080ff0180ff018080");
