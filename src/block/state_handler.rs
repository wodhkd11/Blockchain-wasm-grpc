use std::sync::Arc;

use tokio::sync::RwLock;

use crate::block::{db::Storage, state::GlobalBalance};

#[derive(Clone)]
pub struct BalanceReader{
    inner: Arc<RwLock<GlobalBalance>>,
    storage: Arc<Storage>
}
#[derive(Clone)]
pub struct BalanceWriter{
    inner: Arc<RwLock<GlobalBalance>>,
    storage: Arc<Storage>
}

//balanceReader
impl BalanceReader{
    pub fn new(inner: Arc<RwLock<GlobalBalance>>, storage: Arc<Storage>) -> Self{
        Self{inner, storage}
    }
    pub async fn get_balance(&self, address: [u8; 20]) -> u64{
        let mut state = self.inner.write().await;
        state.get_balance(&address, self.storage.clone())
    }
    pub async fn get_pending_gas(&self) -> u64{
        let state = self.inner.read().await;
        state.gas_pool
    }
}

impl BalanceWriter{
    pub fn new(inner: Arc<RwLock<GlobalBalance>>, storage: Arc<Storage>) -> Self{
        Self{inner, storage}
    }

    pub async fn exc_committed_transaction(&self, from:[u8;20], to:[u8;20], amount:u64, fee:u64) -> bool{
        let mut state = self.inner.write().await;

        let sender_balance = state.get_balance(&from, self.storage.clone());
        if (sender_balance < amount) && (fee < amount) {return false;}
        state.set_balance(from, sender_balance-amount, &self.storage);

        let net_amount = amount.saturating_sub(fee);
        let receiver_cur = state.get_balance(&to, self.storage.clone());
        state.set_balance(to, receiver_cur + net_amount, &self.storage);
        state.add_gas(fee);
        true
    }

    pub async fn distribute_gas(&self){
        let mut state = self.inner.write().await;
        let total_gas = state.drain_gas_pool();
        if total_gas == 0 {return;}
        let shares: Vec<([u8;20], u64)> = state.gov_shares.iter()
            .map(|(addr, share)| (*addr, *share))
            .collect();
        for (addr, share) in shares{
            let reward = (total_gas * share)/100;
            let current = state.get_balance(&addr, self.storage.clone());
            state.set_balance(addr,current+reward, &self.storage);
        }
        ("[GAS]:가스비 분배 완료");
    }

}