//Config Data를 수정할 수 있는 프로그램

use primitive_types::U256;

use crate::{block::{db::Storage, types::{Address, Balance, GlobalBalance, StateDiff}}, exec::schema::ChangeConfig};

//This transaction don't need gas fee
pub fn config_update(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: Balance,
    fee: Balance,
    params: serde_json::Value, // 
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{
    let threshold = state.config.governance_threshold;
    let user_share = state.gov_shares.get(&from).cloned().unwrap_or(Balance::zero());
    if user_share < Balance::from(threshold) { return Err("YOU ARE NOT SUDOER: INSUFFICIENT GOV SHARES".into()); }
    let update: ChangeConfig = serde_json::from_value(params)
        .map_err(|e| format!("INVALID_JSON {e}"))?;

    let gas_tkn = state.config.gas_token.clone();
    let gas_fee = state.config.min_gas_price.clone();

    let mut changed_accounts = std::collections::HashMap::new();
    {
        let from_acc = state.get_account_safe(&from, cur_height, db);
        if from_acc.balance.get(&gas_tkn).unwrap_or(&Balance::zero()) < &Balance::from(gas_fee) { return Err("INSUFFICIENT_GAS_FEE".into()); }
        from_acc.pay_gas(Balance::from(gas_fee), &gas_tkn)?;
        from_acc.inc_nonce();
        changed_accounts.insert(from, from_acc.clone());
    }

    state.add_to_gas_pool(fee);
    let min_gas = match update.min_gas_price{
        Some(v) => {
            if v == U256::zero() { return Err("[CONFIG]: GAS_FEE_CANNOT_BE_0".into()); }
            v
        }
        None => {
            gas_fee
        }
    };
    let gas_tkn = match update.gas_token{
        Some(v) => {
            if state.token_metadata.contains_key(&v){
                v
            } else { return Err("[CONFIG]: UNSUPPORTED TOKEN".into()); }
        }
        None => {
            gas_tkn
        }
    };
    let gov_threshold = match update.governance_threshold{
        Some(v) => {
            if v == U256::zero() { return Err("[CONFIG]: GOVERNANCE_THRESHOLD CANNOT BE 0".into()); }
            v
        }
        None => {
            state.config.governance_threshold
        }
    };
    
    state.config.min_gas_price = min_gas;
    state.config.gas_token = gas_tkn;
    state.config.governance_threshold = gov_threshold;
    state.config.last_updated_height = cur_height;

    println!("[CONFIG_UPDATE] Height: {}, Updated by: {}", cur_height, hex::encode(from));
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: None,
        config_changed: true,
    })
}
