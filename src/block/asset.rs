use crate::block::types::{Address, TokenInfo};


impl TokenInfo{
    pub fn new(name: &str, symbol: &str, decimals: u8, total_supply: u64, admin: Address) -> Self{
        Self{
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply,
            admin,
        }
    }
}