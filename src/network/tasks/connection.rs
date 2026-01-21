use std::sync::Arc;
use std::net::SocketAddr;
use crate::network::node::NodeManage;
use crate::network::message::NetworkMessage;

impl NodeManage{

    pub async fn start_heartbeat(self:Arc<Self>){
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop{
            interval.tick().await;
            self.broadcast(NetworkMessage::Ping).await;
            //println!("Sent Ping to all peers");
        }
    }

    //Reconnector
    pub async fn start_reconnector(self: Arc<Self>) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

        loop {
            interval.tick().await;
            
            // 접속 시도 대상 추출
            let targets: Vec<SocketAddr> = {
                let state: tokio::sync::RwLockReadGuard<'_, crate::network::node::Node> = self.state.read().await;
                state.unconnected_addrs.iter().cloned().collect()
            };

            {
                let state = self.state.read().await;
                println!("[Reconnector] 현재 연결된 피어: {}개, 대기 중인 주소: {}개", 
                    state.peers.len(), targets.len());
            }

            for addr in targets {
                // 이미 연결되었는지 다시 확인
                {
                    let state = self.state.read().await;
                    if state.peers.contains_key(&addr) { continue; }
                }

                let manager_clone = Arc::clone(&self);
                tokio::spawn(async move {
                    println!("🔌 [Reconnector] {}에 연결 시도 중...", addr);
                    let manager_for_handler = Arc::clone(&manager_clone);
                    
                    if let Ok(reader) = manager_clone.connect_to_peer(addr).await {
                        manager_for_handler.handle_peer(addr, reader).await;
                    } else {
                        println!("[Reconnector] {} 연결 실패", addr);
                    }
                });
            }
        }
    }
}