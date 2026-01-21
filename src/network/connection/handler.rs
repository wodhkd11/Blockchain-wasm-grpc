use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use sha3::{Digest, Keccak256};

use crate::block::types::BlockData;
use crate::crypto::signature::{verify, verify_for_block};
use crate::exec;
use crate::network::node::NodeManage;
use crate::network::message::NetworkMessage;


impl NodeManage{

    pub async fn handle_message(self: Arc<Self>, from_addr: SocketAddr, msg: NetworkMessage){
        let needs_gossip = match &msg{
            NetworkMessage::NewTransaction(_) | NetworkMessage::NewBlock(_) => true,
            _ => false,
        };
        if needs_gossip{
            let msg_id = msg.get_id();
            {
                let mut state = self.state.write().await;
                if state.recent_seen_message.contains_key(&msg_id){return;}
                state.recent_seen_message.insert(msg_id, Instant::now());
            }
            self.relay_message(from_addr, msg.clone()).await;
        }

        if needs_gossip{ println!("Received Message: {:?} from {}", msg, from_addr); }
        match msg{
            NetworkMessage::NewBlock(block) =>{
                //check block is already exists
                { 
                    let state = self.state.read().await;
                    if state.storage.get_block(&block.hash).is_some() { return; }
                }
                //check block is valid
                let calculated_hash = BlockData::calculate_header_hash(&block.header);
                if calculated_hash != block.hash{
                    println!("[WARN]: Hash missmatched");
                    return;
                }
                //check signature is valid
                let sig: [u8;65] = block.signature.clone();
                if !verify_for_block(block.header.valdiator, &sig, &block.hash){
                    println!("[WARN]: Block Signature invalid");
                    return;
                }
                let mut node_state = self.state.write().await;
                let mut temp_state = node_state.global_state.clone();

                match exec::execute_block(&mut temp_state, &block, &node_state.storage){
                    Ok(state_updates) =>{
                        node_state.global_state = temp_state;
                        node_state.storage.commit_block(&block, &state_updates, &node_state.global_state);
                        node_state.last_block = block.clone();
                        node_state.block_height = block.header.height;
                        node_state.global_state.balances.clear();
                        println!("[SUCCESS]: Blcok #{} accepted.", block.header.height);
                    }
                    Err(e) => {
                        println!("[REJECT] Block #{} rejected: {}", block.header.height, e);
                    }
                }
            }
            NetworkMessage::NewTransaction(tx) => {
                let sig_hash:[u8;32] = {
                    let mut hasher = Keccak256::new();
                    hasher.update(&tx.signature);
                    hasher.finalize().into()
                };
                let mut state = self.state.write().await;
                if !state.mempool.contains_key(&sig_hash){
                    state.mempool.insert(sig_hash, tx.clone());
                    println!("[New transaction Added], Total: {}",state.mempool.len());
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
}
