use std::net::SocketAddr;

use tokio::io::AsyncWriteExt;

use crate::network::{message::NetworkMessage, node::NodeManage};

impl NodeManage{

    pub async fn relay_message(&self, exclude_addr: SocketAddr, msg:NetworkMessage){
        let state=self.state.read().await;
        for &addr in state.peers.keys(){
            if addr == exclude_addr{continue;}
            let manager_clone = self.clone();
            let msg_clone = msg.clone();
            tokio::spawn(async move{
                manager_clone.send_to(addr, msg_clone).await;
            });
        }
    }

    /**
     * This function use when Your node is Miner
     * Or you got data from out of BlockChain Nodes
     */
    pub async fn broadcast(&self, msg:NetworkMessage){
        let bin = msg.encode();
        let mut state = self.state.write().await;
        for (_, peer) in state.peers.iter_mut(){
            if let Some(ref mut writer) = peer.writer{
                let _ = writer.write_all(&bin).await;
            }
        }
    }

}
