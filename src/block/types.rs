use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

// Type aliases
pub type Address = [u8; 20];
pub type Hash = [u8; 32];
pub type Signature = [u8; 65];
pub type TokenTicker = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenInfo{
    pub name: String,
    pub symbol: TokenTicker,
    pub decimals: u8,
    pub total_supply: u64,
    pub admin: Address,
}
// Block types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader{
    pub height: u64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u64,
    pub valdiator: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockData{
    pub header: BlockHeader,
    pub body: Vec<crate::block::transaction::ConfirmedTransaction>,
    pub hash: Hash,
    #[serde(with = "BigArray")]
    pub signature: Signature,
}

// Account and Balance types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account{
    pub balance: HashMap<TokenTicker, u64>, //Symbol, value
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBalance{
    pub balances: HashMap<Address, Account>,
    pub gov_shares: HashMap<Address, u64>,
    pub gas_pool: u64,
    pub token_metadata: HashMap<TokenTicker, TokenInfo>,
}
