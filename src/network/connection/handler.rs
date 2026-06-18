use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use sha3::digest::block_buffer::Block;
use sha3::{Digest, Keccak256};

use crate::block::types::BlockData;
use crate::crypto::signature::{verify, verify_for_block};
use crate::exec;
use crate::network::node::{self, NodeManage};
use crate::network::message::NetworkMessage;


impl NodeManage{
    fn needs_gossip(&self, msg: &NetworkMessage) -> bool{
        matches!(
            msg,
            NetworkMessage::NewTransaction(_) |
            NetworkMessage::NewBlock(_)
        )
    }

    pub async fn handle_message(self: Arc<Self>, from_addr: SocketAddr, msg: NetworkMessage){
        if self.needs_gossip(&msg){
            if !self.mark_seen(&msg).await{ return; }
            let self_clone = self.clone();
            let msg_clone = msg.clone();
            tokio::spawn(async move{ self_clone.relay_message(from_addr, msg_clone).await; });
            { println!("Received Message: {:?} from {}", &msg, &from_addr); }
        }

        match msg{
            NetworkMessage::NewBlock(block) => self.handle_new_block(block).await,
            NetworkMessage::NewTransaction(tx) => {
                let tx_id = tx.calculate_hash();
                let mut state = self.state.write().await;
                if !state.mempool.contains_key(&tx_id){
                    if tx.verify(){
                        state.mempool.insert(tx_id, tx.clone());
                        println!("[NEW TRANSACTION]: ID: {:?}", tx_id);
                    } else { 
                        println!("[TRANSACTION REJECTED]: Invalid transaction signature");
                    }
                }
            }
            NetworkMessage::Hello { listening_port } => {
                let real_addr = SocketAddr::new(from_addr.ip(), listening_port);
                let mut state = self.state.write().await;
                state.unconnected_addrs.remove(&real_addr);
                if let Some(peer) = state.peers.get_mut(&from_addr){
                    peer.address = Some(real_addr);
                }
                
            }
            NetworkMessage::GetPeers =>{
                let response ={
                    let state = self.state.read().await;
                    let addrs: Vec<SocketAddr> = state.peers.values()
                        .filter_map(|p| p.address)
                        .filter(|&a| a!= from_addr)
                        .take(10)
                        .collect();
                    NetworkMessage::Peers(addrs)
                };
                self.send_to(from_addr, response).await;
            }
            NetworkMessage::Peers(addrs) => {
                println!("{from_addr} Sent {} New nodes data", addrs.len());
                let mut state = self.state.write().await;
                for addr in addrs{
                    let is_already_connected = state.peers.values().any(|p| p.address == Some(addr));
                    let is_self = addr == state.addr;
                    if !is_self && !state.peers.contains_key(&addr) && !is_already_connected{
                        state.unconnected_addrs.insert(addr);
                    }
                }
            }
            /*for test */
            NetworkMessage::Ping=>{
                self.send_to(from_addr,NetworkMessage::Pong).await;
            }
            NetworkMessage::Pong => {
                //println!("Received POng from {from_addr}");
            }
            _ => {}
        }
        
    }

    async fn mark_seen(&self, msg: &NetworkMessage) -> bool{
        let msg_id = msg.get_id();
        let mut state = self.state.write().await;
        if state.recent_seen_message.contains_key(&msg_id){ return false; }
        state.recent_seen_message.insert(msg_id, Instant::now());
        true
    }



///////handlers////////
    async fn handle_new_block(&self, block:BlockData){
        if !block.verify_all(){ return; }
        let mut node_state = self.state.write().await;
        if node_state.storage.get_block(&block.hash).is_some(){ return; }
        
        let storage = node_state.storage.clone();

        let exec_result = {
            let mut global_state = node_state.global_state.write().await;
            match exec::execute_block(&mut global_state, &block, &storage){
                Ok((acc_updates,tk_updates)) => (Some((acc_updates, tk_updates))),
                Err(e) => {
                    println!("[REJECT]: {e}");
                    (None)
                }
            }
        };
        if let Some((acc_updates, tk_updates)) = exec_result{
            {
                let mut final_global = node_state.global_state.write().await;
                final_global.remove_from_memory(block.header.height, 20);// 여기도 주기 
                node_state.storage.commit_block(
                    &block,
                    &acc_updates,
                    &tk_updates,
                    &final_global
                );
            }
            node_state.last_block = block.clone();
            node_state.block_height = block.header.height;
            println!("[SUCCESS]: Block #{} accepted {} transactions",
                block.header.height, block.body.len());
        }

    }
}
