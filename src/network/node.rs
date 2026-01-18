


use tokio::net::{TcpListener};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr};
use std::time::{Instant};
//use crate::block::block_tester::run_block_tester;
use crate::block::db::Storage;
use crate::block::model_struct::{Hash};
use crate::block::state::GlobalBalance;
use crate::block::transaction::TransactionData;
use crate::network::{discovery, rpc};
use crate::network::peer::{Peer};

//현재 노드가 어떤 상태인지를 담고 있다.
// 현재 개방중인 포트, 내 IP주소, 나와 연결된 노드들의 정보를 담고 있다.
pub struct Node{ 
    pub port:u16,
    pub addr: SocketAddr,
    pub wallet: [u8; 20],
    pub chain_id: u64,
    pub peers: HashMap<SocketAddr, Peer>,
    pub unconnected_addrs: HashSet<SocketAddr>,
    pub max_peers: usize ,
    pub recent_seen_message: HashMap<Vec<u8>, Instant>,
    pub mempool: HashMap<Hash, TransactionData>, // 다음 블록이 생기기 이전까지 트랜잭션 저장 캐시 역할을 맡음. 
    pub global_state: GlobalBalance, //블록 생기기 전까지 잔액을 관리함
    pub storage: Arc<Storage>,
}

#[derive(Clone)]
pub struct NodeManage{
    pub state: Arc<RwLock<Node>>,
}
impl NodeManage{


    pub fn new(port:u16, addr: &str, wallet: [u8;20], path: &str) -> Self{
        let node_addr = addr.parse().expect("INVALID ADDR");
        Self { 
            state: Arc::new(RwLock::new(Node{
                port,
                addr: node_addr,
                wallet: wallet,
                chain_id: 6699,
                peers: HashMap::new(),
                unconnected_addrs: HashSet::new(),
                max_peers: 100, // Default: 10, need to change
                recent_seen_message: HashMap::new(),
                mempool: HashMap::new(),
                global_state: GlobalBalance::new(),
                storage: Arc::new(Storage::new(path)),
            })),
         }
    }
    pub async fn start(self: Arc<Self>, seeds:Vec<&str>) {
        let addr = {self.state.read().await.addr};
        let listener = TcpListener::bind(addr).await.unwrap();
        let my_addr = self.state.read().await.wallet;

        //Make Ping - Pong for runtime
        let manager = Arc::clone(&self);

        let hb = Arc::clone(&manager);
        tokio::spawn(async move {hb.start_heartbeat().await;});

        let gc = Arc::clone(&manager);
        tokio::spawn(async move {gc.recent_message_collecter().await;});

        let rc = Arc::clone(&manager);
        tokio::spawn(async move{rc.start_reconnector().await;});

        let mn = Arc::clone(&manager);
        tokio::spawn(async move{mn.start_miner().await;});
        //let tester = Arc::clone(&self);
        //tokio::spawn(async move{run_block_tester(tester).await;});

        //let txgen = Arc::clone(&manager);
        //tokio::spawn(async move {txgen.start_transaction_generator().await;});
        
        
        let my_port = manager.state.read().await.port;
        let rpcport = my_port + 1000;
        let rpcmanager = Arc::clone(&self);
        tokio::spawn(async move{rpc::start_rpc_server(rpcmanager, rpcport).await;});

        //최초 부팅 시 동기화 진행
        //discovery::Discov
        let discovery = discovery::DiscoveryManager::new(seeds);
        let booter = Arc::clone(&manager);
        tokio::spawn(async move{
            discovery.boot(booter).await;
        });

        ///////// Initialized Comeplete //////////
        

        //discovery 이후 노드들과 소통하는 과정
        loop{
            let (socket, peer_addr) = match listener.accept().await{
                Ok(conn) => conn,
                Err(e) =>{
                    println!("Binding Error: {}", e);
                    continue;
                }
            };
            // 신규 노드 연결 시 Random하게 기존 노드를 삭제하는 로직. 구현 필요함.
            // {
                // let mut state = self.state.write().await;
                // if state.peers.len() >= state.max_peers{
                    // let rmaddr = state.peers.keys().next().cloned();
                    // if let Some(addr) = rmaddr{
                        // println!("Peer limit overed. removed {addr}");
                        // state.peers.remove(&addr);
                    // }
                // }
            // }
            let (reader, writer) = socket.into_split();
            { // Add Found node
                let mut state_guard = self.state.write().await;
                let mut peer = Peer::new(peer_addr);
                peer.writer = Some(writer);
                state_guard.peers.insert(peer_addr, peer);
            }
            let manager_clone = Arc::clone(&self);
            tokio::spawn(async move{
                manager_clone.handle_peer(peer_addr, reader).await;
            });
            
        }

    }
}


