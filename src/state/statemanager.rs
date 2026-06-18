use std::{collections::HashMap, sync::Arc};

use eth_trie::{EthTrie, Trie};
use primitive_types::{H256, U256};
use alloy_primitives::B256;

use crate::{block::{db::{Storage, TrieDb}, types::{Account, AccountState, Address, GlobalBalance, Hash, PrimaryAsset, StateDiff}}, rule::config::NetworkConfig};

#[derive(Debug)]
pub enum StateError{
    TrieError(String),
    DecodeError(String),
    EncodeError(String),
    SubTrieError(String),
}

impl std::fmt::Display for StateError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        match self{
            StateError::TrieError(e) => write!(f, "Trie error: {e}"),
            StateError::DecodeError(e) => write!(f, "RLP Decode Error: {e}" ),
            StateError::EncodeError(e) => write!(f, "RLP Encode Error: {e}"),
            StateError::SubTrieError(e) => write!(f, "Sub-Trie Error: {e}"),
        }
    }
}
impl std::error::Error for StateError{}


pub struct StateManager{
    pub storage: Arc<Storage>,
    pub trie: EthTrie<TrieDb>,
}

impl StateManager{
    pub fn new(storage: Arc<Storage>, root_hash: Hash) -> Result<Self, StateError>{
        let db_adapter = TrieDb { inner: storage.db.clone() };
        let trie = if root_hash == [0u8; 32]{
            EthTrie::new(Arc::new(db_adapter))
        } else {
            let root = B256::from_slice(&root_hash);
            EthTrie::from(Arc::new(db_adapter), root).map_err(|e| StateError::TrieError(format!("{:?}", e)))?
        };
        Ok(Self{storage, trie})

    }

    pub fn apply_diff(&mut self, diff: StateDiff,
        global_state: &mut GlobalBalance
    ) -> Result<H256, StateError>{
        for (address, account) in diff.accounts{
            let existing_data = self.trie.get(&address)
                .map_err(|e| StateError::TrieError(format!("{:?}", e)))?;
            let mut mpt_account = if let Some(bytes) = existing_data{
                rlp::decode::<AccountState>(&bytes)
                    .map_err(|e| StateError::DecodeError(format!("{:?}", e)))?
            } else {
                AccountState{
                    nonce: 0,
                    primary_assets: Vec::new(),
                    asset_root: H256::zero(),
                    is_frozen: false,
                }
            };

            mpt_account.nonce = account.nonce;

            global_state.balances.insert(address, account.clone());

            if let Some(gov_balance) = account.balance.get(&global_state.config.gov_token) {
                println!("[D4], Addr: {:?} Gov balance changed to {}", address, gov_balance);
                if gov_balance.is_zero(){
                    global_state.gov_shares.remove(&address);
                } else {
                    global_state.gov_shares.insert(address, *gov_balance);
                }
            }


            let mut sub_trie_opt = None;

            for (ticker, amount) in account.balance{
                if ticker == global_state.config.gas_token || ticker == global_state.config.gov_token {
                    if let Some(pos) = mpt_account.primary_assets.iter().position(|a| a.ticker == ticker) {
                        mpt_account.primary_assets[pos].amount = amount;
                    } else { 
                        mpt_account.primary_assets.push(PrimaryAsset { ticker, amount });
                    }
                } else {
                    let sub_trie = if let Some(ref mut st) = sub_trie_opt{
                        st
                    } else {
                        let st = if mpt_account.asset_root == H256::zero(){
                            EthTrie::new(Arc::new(TrieDb {inner: self.storage.db.clone()}))
                        } else {
                            let root_b256 = B256::from_slice(mpt_account.asset_root.as_bytes());
                            EthTrie::from(Arc::new(TrieDb { inner: self.storage.db.clone() }), root_b256)
                                .map_err(|e| StateError::SubTrieError(format!("{:?}", e)))?
                        };
                        sub_trie_opt = Some(st);
                        sub_trie_opt.as_mut().unwrap()
                    };

                    let encoded_amount = rlp::encode(&amount);
                    sub_trie.insert(ticker.as_bytes(), &encoded_amount)
                        .map_err(|e| StateError::SubTrieError(format!("{:?}", e)))?;
                }
            }
            
            if let Some(mut sub_trie) = sub_trie_opt{
                let sub_root_b256 = sub_trie.root_hash()
                    .map_err(|e| StateError::SubTrieError(format!("{:?}", e)))?;
                mpt_account.asset_root = H256::from_slice(sub_root_b256.as_slice());
            }

            mpt_account.primary_assets.sort_by((|a, b| a.ticker.cmp(&b.ticker)));

            let encoded_state = rlp::encode(&mpt_account);
            self.trie.insert(&address, &encoded_state)
                .map_err(|e| StateError::TrieError(format!("{:?}", e)))?;
        }
        let main_root_b256 = self.trie.root_hash()
            .map_err(|e| StateError::TrieError(format!("{:?}", e)))?;
        Ok(H256::from_slice(main_root_b256.as_slice()))
    }






    pub fn get_account_from_mpt(&self, address: &Address, cur_height: u64) -> Result<Account, StateError>{
        let existing_data = self.trie.get(address).ok().flatten();

        if let Some(bytes) = existing_data{
            let state = rlp::decode::<AccountState>(&bytes)
                .map_err(|e| StateError::DecodeError(format!("{:?}", e)))?;
            let mut balance_map = HashMap::new();
            for asset in state.primary_assets{
                balance_map.insert(asset.ticker, asset.amount);
            }
            Ok(Account{
                balance: balance_map,
                nonce: state.nonce,
                last_seen_block: cur_height,
                asset_root: state.asset_root,
                is_frozen: state.is_frozen,
            })

        } else {
            Ok(Account{
                balance: HashMap::new(),
                nonce: 0,
                last_seen_block: cur_height,
                asset_root: H256::zero(),
                is_frozen: false,
            })
        }
    }

    pub fn get_full_state_from_trie(&self) -> HashMap<Address, Account> {
        let mut full_state = HashMap::new();

        for item in self.trie.iter(){
            if let Ok((key, value)) = item{
                let mut addr = [0u8; 20];
                addr.copy_from_slice(&key);

                if let Ok(state) = rlp::decode::<AccountState>(&value) {
                    let mut balance = HashMap::new();
                    for asset in state.primary_assets{
                        balance.insert(asset.ticker, asset.amount);
                    }

                    full_state.insert(addr, Account{
                        balance,
                        nonce: state.nonce,
                        last_seen_block: 0,
                        asset_root: state.asset_root,
                        is_frozen: state.is_frozen,
                    });
                }
            }
        }
        full_state
    }



    // pub fn update_account(&mut self, address: Address, nonce: u64, balance: U256) -> H256{
        // let existing_data = self.trie.get(&address).expect("Trie get failed");
        // let mut state = if let Some(bytes) = existing_data{
            // rlp::decode::<AccountState>(&bytes).expect("RLP Decode Failed")
        // }else{
            // AccountState { nonce: 0, primary_assets: Vec::new(), asset_root: H256::zero() }
        // };
        // state.nonce = nonce;

        // if ticker

        // let encoded = rlp::encode(&state);
        // self.trie.insert(&address, &encoded).expect("Trie Insert Failed");

        // let b256_root = self.trie.root_hash().expect("Get Root Hash Failed");
        // H256::from_slice(b256_root.as_slice())        
    // }
}


