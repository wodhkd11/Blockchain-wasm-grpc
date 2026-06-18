use std::{collections::{HashMap, HashSet}, sync::Arc};
use crate::{block::{transaction::ConfirmedTransaction, types::{Account, Address, Balance, BlockData, StateDiff}}, exec, network::{message::NetworkMessage, node::{self, Node, NodeManage}}};
use tokio::{sync::RwLock, time::{Duration, sleep}};

impl NodeManage{
    pub async fn start_miner(self: Arc<Self>){

        // 다른 노드들에 먼저 접속해서 최근 20개 블록 데이터? 를 받아와야함. 해시는 20개의 블록으로 하기
        println!("[MINER]BOOTING MINER");
        loop{
            sleep(Duration::from_secs(4)).await;
            let mut node_lock = self.state.write().await;
            if node_lock.port != 9000{
                println!("PERMISSION DENIED ");
                continue;
            }
            // if node_lock.mempool.is_empty(){
                // println!("NO TRANSACTIONS");
                // continue;
            // }
            // if my_gov == 0{
                // println!("PERMISSION DENIED");
                // drop(node_lock);
                // continue;
            // }// 이외에도 POS 등 로직 넣어줘야하는곳. 현재 내가 마이닝노드인지 확인 알고리즘 필요함.
            
            let mut global_state = node_lock.global_state.write().await.clone();
            let mut state_update: HashMap<Address, Account> = HashMap::new();
            let mut token_updates = HashSet::new();
            let mut config_changed: bool = false;
            let mut valid_transactions = Vec::new();


            let my_address = node_lock.wallet;
            let my_gov = global_state.gov_shares.get(&my_address).cloned().unwrap_or(Balance::zero());
            let port = node_lock.port;
 
            let keys: Vec<_>  = node_lock.mempool.keys().take(100).cloned().collect();
            let storage = node_lock.storage.clone();
            let next_height = node_lock.block_height + 1;
            let state_manager = node_lock.state_manager.clone();

            for key in keys{
                let tx = if let Some(tx) = node_lock.mempool.get(&key){
                    tx.clone()
                }else {continue;};
                
                if !tx.verify(){
                    node_lock.mempool.remove(&key);
                    println!("1");
                    continue;
                }

                match exec::apply_transaction(&mut global_state, &tx, next_height, &storage){
                    Ok(diff) => {
                        if diff.config_changed == true {
                            config_changed = true;
                        }
                        println!("[D2]: {}", global_state.gas_pool);
                        node_lock.mempool.remove(&key);
                        valid_transactions.push(ConfirmedTransaction::from(&tx));
                        for (addr, acc) in diff.accounts{
                            state_update.insert(addr, acc);
                        }
                        if let Some(ticker) = diff.token_changed{
                            token_updates.insert(ticker);
                        }
                    }
                    Err(e) => {
                        println!("3");
                        println!("[MINER]: Transaction Exec failed {e}");
                        node_lock.mempool.remove(&key);
                    }
                }
            }
            if valid_transactions.is_empty() {
                println!("[MINER]: No transaction, jump this block.");
                continue;
            }

            let rewarded_updates = match global_state.distribute_gas(next_height, &storage){
                Ok(updates) => updates,
                Err(e) => {
                    println!("[MINER]: Gas distribution failed: {:?}", e);
                    continue;
                }
            };
            
            for (addr, acc) in rewarded_updates{
                state_update.insert(addr, acc);
            }

            let final_diff = StateDiff{
                accounts: state_update.clone(),
                token_changed: None,
                config_changed,
            };

            let mut sm_lock = state_manager.write().await;

            let apply_result = sm_lock.apply_diff(final_diff, &mut global_state);
            let new_state_root = match apply_result{
                Ok(root) => root.into(),
                Err(e) => {
                    if config_changed{
                        println!("[MINER]: MPT FAILED BUT CONFIG CHANGED, FORCING GENERATION");
                        println!("[MINER]: original error: {:?}", e);
                        node_lock.last_block.header.state_root
                    } else {
                        println!("[MINER]: MPT ROOT GENERATE FAILED: {:?}", e);
                        continue;
                    }
                }
            };

            if config_changed { println!("[MINER]: Config Changed"); }

            println!("[MINER]: New block generating: {} Transactions", valid_transactions.len());
        
            let new_block = BlockData::new(
                &node_lock.last_block,
                valid_transactions,
                new_state_root.into(),
                my_address
            );

            if let Err(e) = node_lock.storage.commit_block(&new_block, &state_update, &token_updates, &global_state){
                println!("[MINER]: DB commit failed: {:?}", e);
                continue;
            }
            global_state.remove_from_memory(next_height, 20); // 이거 블록 20개가 아니라 cnofig에서 가져오는거도 해봐야됨.
            *node_lock.global_state.write().await = global_state;

            
            let block_hash = new_block.hash;
            node_lock.block_height = next_height;
            node_lock.last_block = new_block.clone();

            //global_state.balances.clear();
            println!("\n====================");
            println!("[MINER]: New block generated");
            println!("Height: {}",next_height);
            println!("State Root: {:?}",new_state_root);
            println!("Hash: {:?}",hex::encode(block_hash));
            println!("\n====================\n");
            let msg = NetworkMessage::NewBlock(new_block);
            drop(node_lock);
            self.broadcast(msg).await;

        }
    }


}