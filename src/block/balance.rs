
use std::collections::HashMap;

use primitive_types::H256;

use crate::block::account;
use crate::block::db::Storage;
use crate::block::types::{Account, Address, Balance, GlobalBalance, TokenTicker};
use crate::rule::config::{self, NetworkConfig};
use crate::state::statemanager::{StateError, StateManager};


/// 여기서 상태 전이 함수를 정의해야함.
impl GlobalBalance{

    pub fn new() -> Self{
        let gov_shares = HashMap::new();
        let balances = HashMap::new();
        Self{
            balances,
            gov_shares,
            gas_pool: Balance::zero(),
            token_metadata: HashMap::new(),
            config: crate::rule::config::NetworkConfig::new(18),
        }
    }

    pub fn remove_from_memory(&mut self, cur_height: u64, retation: u64){
        let before_count = self.balances.len();

        self.balances.retain(|_, acc|{
            (cur_height.saturating_sub(acc.last_seen_block)) < retation
        });
        let after_count = self.balances.len();
        if before_count != after_count{
            println!("[GLOBAL STATE]: REMOVED {} account from RAM", before_count - after_count);
        }
    }

    pub fn get_account_safe(
        &mut self,
        address: &Address,
        cur_height: u64,
        db: &Storage
    ) -> &mut Account{
        self.balances.entry(*address).or_insert_with(||{
            db.get_account_flat(address).unwrap_or(Account::new(cur_height))
        })
    }
    pub fn get_account_read_safe(
        &self,
        addr: &Address,
        cur_height: u64,
        db: &Storage
    ) -> Result<Account, StateError>{
        if let Some(acc) = self.balances.get(addr){
            return Ok(acc.clone());
        }
        Ok(db.get_account_flat(addr).unwrap_or_else(|| Account::new(cur_height)))
    }

    pub fn add_to_gas_pool(&mut self, amount: Balance) {
        if !amount.is_zero(){
            self.gas_pool += amount;
        }
    }

    //methods

    pub fn get_token_balance_safe(
        &mut self,
        address: &Address,
        toekn: &TokenTicker,
        cur_height: u64,
        db: &Storage
    ) -> Result<Balance, StateError>{
        let account = self.get_account_safe(address, cur_height, db);
        Ok(*account.balance.get(toekn).unwrap_or(&Balance::zero()))
    }
    
    pub fn get_nonce_safe(
        &mut self,
        addr: &Address,
        cur_height: u64,
        db: &Storage
    ) -> Result<u64, StateError>{
        let account = self.get_account_safe(addr, cur_height, db);
        Ok(account.nonce)
    }

    pub fn check_nonce(
        &self,
        addr: &Address,
        tx_nonce: u64,
        cur_height: u64,
        db: &Storage
    ) -> Result<bool, StateError> {
        let account = self.get_account_read_safe(addr, cur_height, db)?;
        Ok(account.nonce == tx_nonce)
    }

    pub fn distribute_gas(&mut self, cur_height: u64, db: &Storage) -> Result<HashMap<Address, Account>, StateError>{
        let gas_tkn = self.config.gas_token.clone();
        println!("[D3]: {}", self.gas_pool);
        let mut rewarded_accounts = HashMap::new();
        if self.gas_pool == Balance::zero(){return Ok(rewarded_accounts);}

        let total_gas = self.gas_pool;
        self.gas_pool = Balance::zero();

        let shares: Vec<(Address, Balance)> = self.gov_shares
            .iter()
            .map(|(addr, share)| (*addr, *share))
            .collect();
        let total_shares: Balance = self.gov_shares
            .iter()
            .map(|(_, share)| *share)
            .fold(Balance::zero(), |acc, x| acc + x);
        println!("[TOTAL GAS]: {total_gas}KRW");
        for (addr, share) in shares{
            let reward = if total_shares == Balance::zero() {
                Balance::zero()
            } else {
                (total_gas * share) / total_shares
            };
            if !reward.is_zero(){
                let account = self.get_account_safe(&addr, cur_height, db);
                account.add_balance(&gas_tkn, reward);
                account.last_seen_block = cur_height;
                rewarded_accounts.insert(addr, account.clone());
            }
        }
    Ok(rewarded_accounts)
    }
}
