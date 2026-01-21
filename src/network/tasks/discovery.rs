use std::{net::SocketAddr, sync::{Arc}};
use crate::network::node::{NodeManage};


pub struct DiscoveryManager {
    seed_nodes: Vec<SocketAddr>,
}

impl DiscoveryManager{
    pub fn new(seeds: Vec<&str>) -> Self{
        let mut seed_nodes = Vec::new();
        for s in seeds{
            match s.parse::<SocketAddr>(){
                Ok(addr) => seed_nodes.push(addr),
                Err(e) => println!("Failed to parse seed address {s}(error: {e})")
            }
        }
        Self{seed_nodes}
    }
    pub async fn boot(&self, manager: Arc<NodeManage>){
        println!("Start booting with {} nodes", self.seed_nodes.len());
        self.process_discovered_addrs(manager, self.seed_nodes.clone()).await;
    }

    pub async fn process_discovered_addrs(&self, manager: Arc<NodeManage>, addrs: Vec<SocketAddr>){
        let (my_addr, already_connected) = {
            let state = manager.state.read().await;
            let my_addr = state.addr;
            let connected_list: Vec<SocketAddr> = state.peers.keys().cloned().collect();
            (my_addr, connected_list)
        };

        for addr in addrs{
            if addr ==my_addr{continue;}
            if already_connected.contains(&addr) {continue;}
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move{
                let manager_for_handler = Arc::clone(&manager_clone);
                if let Ok(reader) = manager_clone.connect_to_peer(addr).await{
                    manager_for_handler.handle_peer(addr,reader).await;
                }
            });
        }


    }

    // pub async fn discover_new_peers(&self, received_addrs: Vec<SocketAddr>){
    //     let mut state = self.state.write();
    //     for addr in received_addrs{
    //         if addr != state.addr
    //     }
    // }
}