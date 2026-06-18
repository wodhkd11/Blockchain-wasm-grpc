




//작성 필요

use serde::Deserialize;

use crate::{block::types::Balance, exec::schema::RawPayload};

#[derive(Debug)]
pub enum DecodeError{
    JsonError(serde_json::Error),
    INvalidFormat,
}

impl From<serde_json::Error> for DecodeError{
    fn from(err: serde_json::Error) -> Self{
        DecodeError::JsonError(err)
    }
}

pub fn decode_payload(payload: &[u8]) -> Result<RawPayload, String>{
    if payload.is_empty(){
        return Err("EMPTY_PAYLOAD".to_string());
    }
    //is this JSON?
    if payload[0] == 0x7b { // 0x7b:: { => 0x7b means bracket, it means json
        #[derive(serde::Deserialize)]
        struct JsonPayload{
            opcode: u8,
            fee: Option<Balance>,
            data: serde_json::Value,
        }
        let j: JsonPayload = serde_json::from_slice(payload)
            .map_err(|e| format!("JSON_PARSING_ERR: {:?}", e))?;
        return Ok(RawPayload{
            opcode: j.opcode,
            fee: j.fee,
            data: j.data.to_string().into_bytes(),
        });
    }    
    rlp::decode::<RawPayload>(payload)
        .map_err(|e| format!("RLP_DECODE_FAILED: {:?}", e))

    //RLP 판별
}