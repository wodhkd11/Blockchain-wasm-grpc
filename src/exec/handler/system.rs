//보안 중요함

//기존 토큰의 메타데이터에 수정자로 접근이 가능하므로, 덮어쓰기도 못하게 하는 등 여러 보안 필요

use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, Balance, GlobalBalance, StateDiff, TokenInfo, TokenTicker}}, exec::schema::RegisterTokenParams};


/**
 * param input:
 * pub name: String,
 * pub symbol: TokenTicker,
 * pub admin: Address,
 * pub initial_supply: u64, 
 * pub decimals: u8,

 */
pub fn register_token(
    state: &mut GlobalBalance,
    from: Address,
    to: Address, //None 0
    value: Balance, //None 0
    fee: Balance,
    params: serde_json::Value,
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{
    let json_params: RegisterTokenParams = serde_json::from_value(params)
        .map_err(|e| format!("INVALID_JSON: {e}"))?;
    
    let ticker = json_params.symbol.to_uppercase();
    //이거 왜 거버넌스 쉐어 없는데 작동하지;
    if !state.gov_shares.contains_key(&from){return Err("PERMISSION_DENIED".into());}
    let threshold = state.config.governance_threshold;
    if state.gov_shares.get(&from).unwrap() < &Balance::from(threshold) {return Err("THRESHOLD_ERROR".into());}
    if state.token_metadata.contains_key(&ticker){
        return Err(format!("TOKEN_ALREADY_EXISTS_{ticker}"));
    }
    if ticker.len() < 2 || ticker.len() > 10 || !ticker.chars().all(|c| c.is_alphabetic()){
        return Err("INVALID_TOKEN_TICKER_FORMAT".into());
    }
    let new_metadata = TokenInfo::new(
        &json_params.name,
        &ticker,
        json_params.decimals,
        json_params.initial_supply,
        to,
    );
    state.token_metadata.insert(ticker.clone(), new_metadata);
    let gas_tkn = state.config.gas_token.clone();
    {
        let from_acc = state.get_account_safe(&from, cur_height, db);
        from_acc.pay_gas(fee, &gas_tkn);
        from_acc.inc_nonce();
    }
    state.add_to_gas_pool(fee);
    {
        let to_acc = state.get_account_safe(&to, cur_height, db);
        to_acc.add_balance(&ticker, value);
    }

    println!("[NEW TOKEN] Registered: {ticker} by {}", hex::encode(from));

    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read_safe(&to, cur_height, db).map_err(|e| format!("{:?}", e))?);
    changed_accounts.insert(from, state.get_account_read_safe(&from, cur_height, db).map_err(|e| format!("{:?}", e))?);
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: Some(ticker),
        config_changed: false,
    })
}

