
pub mod decoder;
mod opcodes;
pub mod schema;
pub mod handler;



use std::collections::HashMap;

use serde::Deserialize;
use crate::{block::{db::Storage, transaction::TransactionData, types::{Account, Address, BlockData, GlobalBalance}}, exec::{handler::{system::register_token, token::handle_transfer}, opcodes::*}};

// pub enum Instruction{
    // RegisterToken(RegisterTokenParams),
    // Transfer(TransferParams),
    // Mint,
    // Burn,

// }

/*
Transaction format
json{
sender
receiver
value
nonce
Payload{
opcode
fee
data
}
}
*/

/*
pub const OP_SYSTEM_REGISTER_TOKEN: u8 = 0x00;
pub const OP_TOKEN_MINT: u8 = 0x01;
pub const OP_TOKEN_TRANSFER: u8 = 0x02;
pub const OP_TOKEN_BURN: u8 = 0x03;
pub const OP_PAY_PURCHASE: u8 = 0x04;
 */
#[derive(Deserialize)]
pub struct RawPayload{
    pub opcode: u8,
    pub fee: u64,
    pub data: serde_json::Value,
}

pub fn apply_transaction(state: &mut GlobalBalance, tx: &TransactionData, db:&Storage) -> Result<(), String>{
    let raw_payload: RawPayload = serde_json::from_slice(&tx.payload)
        .map_err(|_| "Invalid Payload JSON")?;
    let opcode = raw_payload.opcode;
    let fee = raw_payload.fee;
    match opcode{
        OP_SYSTEM_REGISTER_TOKEN => {
            register_token(state, tx.sender, tx.receiver, tx.value, fee, raw_payload.data, &db)
        },
        OP_TOKEN_TRANSFER => {
            handle_transfer(state, tx.sender, tx.receiver, tx.value, fee, raw_payload.data, &db)
        }
        // OP_TOKEN_MINT =>{
        // }

        _ => Err("OP NOT FOUND".to_string())
    }
}

pub fn execute_block(state: &mut GlobalBalance, block: &BlockData, db: &Storage) -> Result<HashMap<Address, Account>, String>{
    
    for tx in &block.body{
        apply_transaction(state, &tx.tx_info, db)?;
    }
    state.distribute_gas(db);
    let state_update = state.get_changed_accounts();
    Ok(state_update)
}