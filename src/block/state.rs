use std::{collections::HashMap, sync::Arc};
use serde::{Deserialize, Serialize};

use crate::block::{db::{self, Storage}, model_struct::BlockData};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account{
    pub balance: u64,
    pub nonce: u64,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBalance{
    pub balances: HashMap<[u8; 20], Account>,
    pub gov_shares: HashMap<[u8; 20], u64>,
    pub gas_pool: u64,
}

impl GlobalBalance{
    pub fn new() -> Self{
        let mut gov_shares = HashMap::new();
        gov_shares.insert([0x11; 20], 60);
        gov_shares.insert([0x22; 20], 40);
        Self{
            balances: HashMap::new(),
            gov_shares,
            gas_pool: 0,
        }
    }
    /**
     * This check current block(Not confirmed) first
     * 
     */
    pub fn get_account(&mut self, address: &[u8; 20], db: Arc<Storage>) -> Account{
        
        if let Some(acc) = self.balances.get(address) {
            return acc.clone();
        }
        if db.is_exist(address){
            if let Some(acc) = db.get_account(address){
                self.balances.insert(*address, acc.clone());
                return acc;
            }
        }
        Account{
            balance: 0,
            nonce: 0
        }
    }

    pub fn get_balance(&mut self, address: &[u8; 20], db: Arc<Storage>) -> u64{
        self.get_account(address, db).balance
    }
    pub fn get_nonce(&mut self, address: &[u8; 20], db: Arc<Storage>) -> u64{
        self.get_account(address, db).nonce
    }
    pub fn get_nonce_readonly(&self, address: &[u8;20]) -> u64{
        self.balances.get(address)
            .map(|acc| acc.nonce)
            .unwrap_or(0)
    }

    pub(crate) fn set_balance(&mut self, address: [u8; 20], amount: u64, db: &Storage){
        ///여기도 없으면 DB에서 가져오는 로직이 핑료함.
        let account = self.balances.entry(address).or_insert_with(||{
            db.get_account(&address).unwrap_or(Account { balance: 0, nonce: 0 })
        });
        account.balance = amount;
    }

    pub fn increase_nonce_only(&mut self, address: [u8; 20], db: Arc<Storage>){
        let account = self.balances.entry(address).or_insert_with(||{
            db.get_account(&address).unwrap_or(Account{balance:0, nonce:0})
        });
        account.nonce = account.nonce.saturating_add(1);
    }

    pub fn increase_nonce_change_balance(&mut self, address: [u8; 20], new_balance: u64, db: &Storage){
        let account = self.balances.entry(address).or_insert_with(||{
            db.get_account(&address).unwrap_or(Account { balance: 0, nonce: 0 })
        });
        account.balance = new_balance;
        account.nonce = account.nonce.saturating_add(1);
        
    }



    pub(crate) fn add_gas(&mut self, fee: u64){
        self.gas_pool = self.gas_pool.saturating_add(fee);
    }
    pub(crate) fn drain_gas_pool(&mut self) -> u64{
        let amount = self.gas_pool;
        self.gas_pool = 0;
        amount
    }

    pub fn commit_to_db(&mut self, db: &Storage, block: &BlockData){
        db.commit_block(block, &self.balances);
        self.balances.clear();
        if self.gas_pool > 0{println!("[WARN] Gas pool is not empty during comit");}
    }
}