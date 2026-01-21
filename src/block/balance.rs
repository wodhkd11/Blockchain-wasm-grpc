
use std::collections::HashMap;

use crate::block::db::Storage;
use crate::block::types::{Account, Address, GlobalBalance, TokenTicker};


/// 여기서 상태 전이 함수를 정의해야함.
impl GlobalBalance{

    pub fn new() -> Self{
        let gov_shares = HashMap::new();
        let balances = HashMap::new();
        Self{
            balances,
            gov_shares,
            gas_pool: 0,
            token_metadata: HashMap::new(),
        }
    }



    //methods
    fn get_account_mut(&mut self, address: &Address, db:&Storage) -> &mut Account{
        self.balances.entry(*address).or_insert_with(||{
            db.get_account(address)
        })
    }
    pub fn get_account_read(&self, address: &Address, db: &Storage) -> Account{
        if let Some(acc) = self.balances.get(address){ return acc.clone(); }
        db.get_account(address)
    }

    pub fn get_changed_accounts(&self) -> HashMap<Address, Account>{
        self.balances.clone()
    }
    pub fn get_token_balance(&mut self, address: &Address, token: &TokenTicker, db: &Storage) -> u64{
        let account = self.get_account_mut(address, db);
        *account.balance.get(token).unwrap_or(&0)
    }

    pub fn get_nonce(&mut self, addr: &Address, db: &Storage) -> u64{
        let account = self.get_account_mut(addr, db);
        account.nonce
    }

    pub fn check_nocne(&mut self, addr: &Address, tx_nonce: u64, db: &Storage) -> bool{
        let account = self.get_account_mut(addr, db);
        account.nonce == tx_nonce
    }

    pub fn inc_nonce(&mut self, addr: &Address, db: &Storage) {
        let account = self.get_account_mut(addr, db);
        account.nonce += 1;
    }

    pub fn add_balance(&mut self, addr: &Address, token: &TokenTicker, amount: u64, db: &Storage) {
        let account = self.get_account_mut(addr, db);
        let balance = account.balance.entry(token.clone()).or_insert(0);
        *balance = balance.saturating_add(amount);
    }
    pub fn sub_balance(&mut self, addr: &Address, token: &TokenTicker, amount: u64, db: &Storage) -> Result<(), String>{
        let account = self.get_account_mut(addr, db);
        let balance = account.balance.entry(token.clone()).or_insert(0);
        if *balance < amount {return Err("INSUFFICIENT BALANCE".into());}
        *balance = balance.saturating_sub(amount);
        Ok(())
    }
    
    pub fn pay_gas(&mut self, addr: &Address, fee: u64, db: &Storage) -> Result<(), String>{
        let account = self.get_account_mut(addr, db);
        let balance = account.balance.entry("KRW".into()).or_insert(0);
        if *balance < fee {return Err("INSUFFICIENT GAS FEE".into());}
        self.sub_balance(addr, &"KRW".into(), fee, db)?;
        self.gas_pool = self.gas_pool.saturating_add(fee);
        Ok(())
    }

    pub fn distribute_gas(&mut self, db: &Storage){
        if self.gas_pool == 0{return;}
        let total_gas = self.gas_pool;
        self.gas_pool = 0;
        let shares: Vec<(Address, u64)> = self.gov_shares
            .iter()
            .map(|(addr, share)| (*addr, *share))
            .collect();
        let total_shares: u64 = self.gov_shares
            .iter()
            .map(|(_, share)| *share)
            .sum();
        println!("[TOTAL GAS]: {total_gas}KRW");
        for (addr, share) in shares{
            let reward = match total_shares{
                0 => 0,
                _ => (total_gas * share) / total_shares,
            };
            if reward > 0{
                self.add_balance(&addr, &"KRW".to_string(), reward, db);
            }
        }
    }
}
