use serde::{Deserialize, Serialize};

use crate::block::types::{Address, TokenTicker};



#[derive(Deserialize, Serialize)]
pub struct RegisterTokenParams{
    pub name: String,
    pub symbol: TokenTicker,
    pub admin: Address,
    pub initial_supply: u64, //
    pub decimals: u8,
}

#[derive(Deserialize, Serialize)]
pub struct TransferParams{ //전이할 데이터: 토큰과 값, from과 to는 transaction에 존재
    pub ticker: TokenTicker,
}

//pub fn MintParams{}
//pub fn BurnParams{}
