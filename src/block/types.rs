use std::collections::HashMap;

use primitive_types::{H256, U256};
use rlp::{Decodable, Encodable};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::rule::config::NetworkConfig;


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
    pub total_supply: Balance,
    pub admin: Address,
}
// Block types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader{
    pub height: u64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,
    pub state_root: Hash,
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
    pub balance: HashMap<TokenTicker, Balance>, //Symbol, value
    pub nonce: u64,
    pub last_seen_block: u64,
    pub asset_root: H256,
    pub is_frozen: bool,
}
impl Account{
    pub fn new(cur_block: u64) -> Self{
       Self { balance: HashMap::new(), nonce: 0, last_seen_block: cur_block, asset_root: H256::zero(), is_frozen: false}
    }
}
// impl Encodable for Account{
//     fn rlp_append(&self, s: &mut rlp::RlpStream) {
//         s.begin_list(3);
//         s.append(&self.nonce);
//         let balance_vec: Vec<(String, U256)> = self.balance.iter().map(|(k,v)| (k.clone(), *v)).collect();
//         s.append_list(&balance_vec);
//         s.append(&self.is_frozen);
//     }
// }


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBalance{
    pub balances: HashMap<Address, Account>,
    pub gov_shares: HashMap<Address, Balance>,
    pub gas_pool: Balance,
    pub token_metadata: HashMap<TokenTicker, TokenInfo>,
    pub config: NetworkConfig,
}

pub struct StateDiff{
    pub accounts: HashMap<Address, Account>,
    pub token_changed: Option<TokenTicker>,
    pub config_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionForDB{
    pub hash: Hash,
    pub block_height: u64,
    pub block_hash: Hash,
    pub index: u32,
    pub status: u8,
}

pub type Balance = primitive_types::U256;
pub type Nonce = u64;

#[derive(Debug, Clone)]
pub struct PrimaryAsset{
    pub ticker: TokenTicker,
    pub amount: Balance,
}

#[derive(Debug, Clone)]
pub struct AccountState{
    pub nonce: u64,
    pub primary_assets: Vec<PrimaryAsset>,
    pub asset_root: H256,
    pub is_frozen: bool,
}

impl Encodable for PrimaryAsset{
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.ticker);

        let buf = self.amount.to_big_endian();
        let start = buf.iter().position(|&x| x != 0).unwrap_or(31);
        s.append(&&buf[start..]);
    }
}
impl Decodable for PrimaryAsset{
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let ticker: TokenTicker =  rlp.val_at(0)?;
        let amount_bytes: Vec<u8> = rlp.val_at(1)?;
        let amount = U256::from_big_endian(&amount_bytes);
        Ok(Self{ticker, amount})
    }
}

impl Encodable for AccountState{
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(3);
        s.append(&self.nonce);
        let mut assets = self.primary_assets.clone();
        assets.sort_by(|a, b| a.ticker.cmp(&b.ticker));
        s.append_list(&assets);
        s.append(&self.asset_root);
        s.append(&self.is_frozen);
    }
}
impl Decodable for AccountState{
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let nonce: u64 = rlp.val_at(0)?;
        let primary_assets: Vec<PrimaryAsset> = rlp.list_at(1)?;
        let asset_root_bytes: Vec<u8> = rlp.val_at(2)?;
        let is_frozen: bool = rlp.val_at(3)?;
        Ok(Self{
            nonce,
            primary_assets,
            asset_root: H256::from_slice(&asset_root_bytes),
            is_frozen,
        })
    }
}