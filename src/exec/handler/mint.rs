use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, Balance, GlobalBalance, StateDiff, TokenTicker}}, exec::schema::*};

pub fn handle_mint(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: Balance,
    fee: Balance,
    params: serde_json::Value, // ticker가 들어있음. 규칙 확인후 권한 있는지 확인
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{

    let (min_gas, gas_token, gov_threshold, gov_token) = {
        let rule = &state.config;
        (rule.min_gas_price, rule.gas_token.clone(), rule.governance_threshold, rule.gov_token.clone())
    };
    //이거 왜 거버넌스 확인하는거 없냐;; 

    let json_params: MintParams = serde_json::from_value(params)
        .map_err(|e| format!("JSON PARSING ERROR: {e}"))?;
    let token = json_params.ticker.to_uppercase();

    if !state.token_metadata.contains_key(&token){ return Err("Unsupported tokena".into()); }

    let gas_token_balance = state.get_token_balance_safe(&from, &gas_token, cur_height, db).map_err(|e| format!("{:?}", e))?;
    if gas_token_balance < fee || fee < Balance::from(min_gas) { return Err("Insufficient balance for gas fee".into()); }

    let gov_balance = state.get_token_balance_safe(&from, &gov_token, cur_height, db).map_err(|e| format!("{:?}", e))?;
    
    if gov_balance < Balance::from(gov_threshold) {
        return Err("[GOVERNANCE]: Permission Denied".into());
    }
    let gas_tkn = state.config.gas_token.clone();
    {
        let from_acc = state.get_account_safe(&from, cur_height, db);
        from_acc.pay_gas(fee, &gas_tkn);
        from_acc.inc_nonce();
    }
    state.add_to_gas_pool(fee);
    {
        let to_acc = state.get_account_safe(&to, cur_height, db);
        to_acc.add_balance(&token, value - fee);
    }
    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read_safe(&to, cur_height, db).map_err(|e| format!("{:?}", e))?);
    changed_accounts.insert(from, state.get_account_read_safe(&from, cur_height, db).map_err(|e| format!("{:?}", e))?);
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: Some(token),
        config_changed: false,
    })
}
