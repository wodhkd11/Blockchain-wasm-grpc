use crate::{block::{db::Storage, types::{Address, GlobalBalance}}, exec::schema::*};



pub fn handle_transfer(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: u64,
    fee: u64,
    params: serde_json::Value,
    db: &Storage
) -> Result<(), String>{
    let json_params:TransferParams = serde_json::from_value(params)
        .map_err(|e| format!("JSON PARSING FAILED:{e}"))?;
    let token = &json_params.ticker;
    if !state.token_metadata.contains_key(token){
        return Err("Unsupported token".into());
    }
    let krw_balance = state.get_token_balance(&from, &"KRW".into(), db);
    if token == "KRW" {
        if krw_balance < value.saturating_add(fee){
            return Err("INSUFFICIENT_KRW".into());
        }
    }else{
        if krw_balance < fee{
            return Err("INSUFFICIENT_GAS".into());
        }
        let token_balance = state.get_token_balance(&from, token, db);
        if token_balance < value{
            return Err(format!("INSUFFICIENT_{token}_BALANCE"));
        }
    }

    state.pay_gas(&from, fee, db)?;
    match state.sub_balance(&from, token, value, db){
        Ok(()) => state.add_balance(&to, &token, value, db),
        Err(e) => return Err(e),
    }

    state.inc_nonce(&from, db);
    Ok(())
    
}
