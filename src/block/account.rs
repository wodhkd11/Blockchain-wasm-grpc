use crate::block::types::{Account, Balance, TokenTicker};


impl Account{
    pub fn pay_gas(&mut self, fee: Balance, gas_tkn: &TokenTicker) -> Result<Balance, String>{
        let balance = self.balance.entry(gas_tkn.clone()).or_insert(Balance::zero());
        if *balance < fee { return Err("INSUFFICIENT_GAS".into()); }
        *balance = balance.saturating_sub(fee);
        Ok(fee)
    }
    pub fn sub_balance(&mut self, token: &TokenTicker, amount: Balance) -> Result<(), String>{
        let balance = self.balance.entry(token.clone()).or_insert(Balance::zero());
        if *balance < amount { return Err("INSUFFICIENT_BALANCE".into()); }
        *balance = balance.saturating_sub(amount);
        Ok(())
    }
    pub fn add_balance(&mut self, token: &TokenTicker, amount: Balance){
        let balance = self.balance.entry(token.clone()).or_insert(Balance::zero());
        *balance = balance.saturating_add(amount);
    }
    pub fn inc_nonce(&mut self){
        self.nonce += 1;
    }
}