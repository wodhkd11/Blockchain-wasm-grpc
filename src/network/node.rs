use tokio::io::AsyncReadExt;
use tokio::net::tcp::{OwnedReadHalf};
use tokio::net::{TcpListener,TcpStream};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr};
use crate::network::discovery;
use crate::network::peer::{Peer};
use crate::network::message::NetworkMessage;
use tokio::io::AsyncWriteExt;


//현재 노드가 어떤 상태인지를 담고 있다.
// 현재 개방중인 포트, 내 IP주소, 나와 연결된 노드들의 정보를 담고 있다.
pub struct Node{ 
    pub port:u16,
    pub addr: SocketAddr,
    pub peers: HashMap<SocketAddr, Peer>,
    pub unconnected_addrs: HashSet<SocketAddr>,
}

#[derive(Clone)]
pub struct NodeManage{
    pub state: Arc<RwLock<Node>>,
}
impl NodeManage{
    pub fn new(port:u16, addr: &str) -> Self{
        let node_addr = addr.parse().expect("INVALID ADDR");
        Self { 
            state: Arc::new(RwLock::new(Node{
                port,
                addr: node_addr,
                peers: HashMap::new(),
                unconnected_addrs: HashSet::new(),
            })),
         }
    }
    /*
    pub async fn booting(self: Arc<Self>, seeds: Vec<SocketAddr>){
        for addr in seeds{
            let my_addr = {self.state.read().await.addr};
            if addr == my_addr {continue;}
            println!("Connecting to seed node {addr}");
            let manager_clone = Arc::clone(&self);
            tokio::spawn(async move{
                manager_clone.connect_to_peer(addr).await;
            });
        }
    }*/
    pub async fn start(self: Arc<Self>, seeds:Vec<&str>) {
        let addr = {self.state.read().await.addr};
        let listener = TcpListener::bind(addr).await.unwrap();

        //For test(pingpong)
        let hb_manager = Arc::clone(&self);
        tokio::spawn(async move{
            hb_manager.start_heartbeat().await;
        });
        let reconnector_manager = Arc::clone(&self);
        tokio::spawn(async move{
            reconnector_manager.start_reconnector().await;
        });
        //최초 부팅 시 동기화 진행
        //discovery::Discov
        let discovery = discovery::DiscoveryManager::new(seeds);
        let manager_clone = Arc::clone(&self);
        tokio::spawn(async move{
            discovery.boot(manager_clone).await;
        });


        //discovery 이후 노드들과 소통하는 과정
        loop{
            let (socket, peer_addr) = match listener.accept().await{
                Ok(conn) => conn,
                Err(e) =>{
                    println!("Binding Error: {}", e);
                    continue;
                }
            };
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


    pub async fn connect_to_peer(self: Arc<Self>, addr: SocketAddr) -> Result<OwnedReadHalf, String>{
        {
            let state = self.state.read().await;
            if state.peers.contains_key(&addr) {return Err(format!("Already connected to: {addr}"));}
        }
        match TcpStream::connect(addr).await{
            Ok(socket) =>{
                println!("Connected to: {addr}");
                let (reader, mut writer) = socket.into_split();
                //let my_port = self.state.read().await.port;
                
                //let hello_bin = NetworkMessage::Hello{listening_port:my_port}.encode();
                //self.send_to(addr,NetworkMessage::Hello { listening_port: my_port }).await;

                //let _ = writer.write_all(&hello_bin).await;
                //tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = writer.write_all(&NetworkMessage::GetPeers.encode()).await;
                
                {
                    let mut state = self.state.write().await;
                    let mut peer = Peer::new(addr);
                    peer.writer = Some(writer);
                    state.peers.insert(addr, peer);
                }
                Ok(reader)
                // more needed
            }
            Err(e) => Err(format!("Failed to connect to : {addr} (error: {e})")),
        }
        
    }

    pub async fn handle_peer(self: Arc<Self>, addr: SocketAddr, mut reader: OwnedReadHalf){
        let mut temp_buffer = [0u8; 1024];
        let mut buffer = Vec::new();
        let my_port = self.state.read().await.port;
        self.send_to(addr, NetworkMessage::Hello{listening_port:my_port}).await;
        loop{
            match reader.read(&mut temp_buffer).await{
                Ok(0) => break,
                Ok(n) =>{
                    buffer.extend_from_slice(&temp_buffer[..n]);
                    while !buffer.is_empty(){
                        if let Some((msg, bytes)) = NetworkMessage::decode_with_bytes(&buffer){
                            if bytes == 0{
                                break;
                            }
                            let manager_clone = Arc::clone(&self);
                            tokio::spawn(async move{
                                manager_clone.handle_message(addr, msg).await;
                            });
                            buffer.drain(..bytes);
                        }else{break;}
                    }
                }
                Err(_) => break,
                
            }
        }
        let mut state = self.state.write().await;
        state.peers.remove(&addr);
        drop(state);
        println!("Connection closed with {addr}");
    }

    async fn handle_message(self: Arc<Self>, from_addr: SocketAddr, msg: NetworkMessage){
        println!("Received Message: {:?} from {}", msg, from_addr);
        match msg{
            NetworkMessage::Hello { listening_port } => {
                let real_addr = SocketAddr::new(from_addr.ip(), listening_port);
                let mut state = self.state.write().await;
                state.unconnected_addrs.remove(&real_addr);
                if let Some(peer) = state.peers.get_mut(&from_addr){
                    peer.address = Some(real_addr);
                    println!("✅ {}의 실제 주소 {}를 장부에 기록했습니다.", from_addr, real_addr);
                }
                
            }
            NetworkMessage::GetPeers =>{
                let response ={
                    let state = self.state.read().await;
                    let addrs: Vec<SocketAddr> = state.peers.values()
                        .filter_map(|p| p.address)
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
                println!("Received POng from {from_addr}");
            }
            _ => {}
        }
    }

    async fn send_to(&self, addr:SocketAddr, msg: NetworkMessage){
        let bin = msg.encode();
        let mut state = self.state.write().await;
        if let Some(peer) = state.peers.get_mut(&addr){
            if let Some(ref mut writer) = peer.writer{
                if let Err(e) = writer.write_all(&bin).await{
                    println!("Failed to send message to {addr}\nMessage: {msg:?}\nError code: {e} \n\n");
                }
                let _ = writer.flush().await;
            }else{
                println!("No writer found for: {addr}");
            }
        }else{
            println!("Peer{addr} not found in state");
        }
    }

    pub async fn start_reconnector(self: Arc<Self>) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

        loop{
            interval.tick().await;
            let targets: Vec<SocketAddr> = {
                let state = self.state.read().await;
                state.unconnected_addrs.iter().cloned().collect()
            };
            {   
                let state = self.state.read().await;
                println!("🔄 [Reconnector] 현재 연결된 피어 수: {}", state.peers.len());
                println!("📋 [Reconnector] 접속 시도 예정 목록: {:?}", targets);
            }
            for addr in targets{
                {
                    let state = self.state.read().await;
                    if state.peers.contains_key(&addr){continue;};
                }
                let manager_clone = Arc::clone(&self);
                tokio::spawn(async move{
                    println!("Connect to : {addr}");
                    let manager_for_handler = Arc::clone(&manager_clone);
                    if let Ok(reader) = manager_clone.connect_to_peer(addr).await{
                        manager_for_handler.handle_peer(addr, reader).await;
                    }else{
                    println!("{addr}: Connect failed");
                    }
                });
            }
        }
    }

    //For test
    pub async fn broadcast(&self, msg:NetworkMessage){
        let bin = msg.encode();
        let mut state = self.state.write().await;
        for (_, peer) in state.peers.iter_mut(){
            if let Some(ref mut writer) = peer.writer{
                let _ = writer.write_all(&bin).await;
            }
        }
    }
    pub async fn start_heartbeat(self:Arc<Self>){
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop{
            interval.tick().await;
            self.broadcast(NetworkMessage::Ping).await;
            println!("Sent Ping to all peers");
        }
    }
}



// impl Node{
//     pub async fn start(self: Arc<Self>){
//         let listener: TcpListener = TcpListener::bind(self.addr).await.expect("PORT BINDING FAILED");
//         println!("Node running at {}", self.addr);
//         loop{
//             let (socket, addr) = match listener.accept().await{
//                 Ok(conn) => conn,
//                 Err(e) => {
//                     println!("Failed to connect: {}",e);
//                     continue;
//                 }
//             };
//             let (reader, writer) = socket.into_split();
//             println!("New peer connected: {}", addr);
//             let mut peer = Peer::new(addr);
//             peer.writer = Some(writer);
//             peer.status = PeerStatus::Connected;

//             let peers_clone = Arc::clone(&self.peers);
//             {
//                 let mut guard = peers_clone.write().await;
//                 guard.insert(addr, peer);
//             }
//             let peers_for_handler = Arc::clone(&self.peers);
//             let self_clone = Arc::clone(&self);
//             tokio::spawn(async move{
//                 self_clone.handle_peer(addr, reader, peers_for_handler).await;
//             });

//         }

//     }

//     async fn handle_peer(self: &Arc<Self>, addr:SocketAddr, mut reader: tokio::net::tcp::OwnedReadHalf, peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>){
//         let mut buffer = [0u8;1024];
//         loop{
//             match reader.read(&mut buffer).await{
//                 Ok(0) => break,
//                 Ok(n) =>{
//                     let data = &buffer[..n];
//                     if let Some(msg) = NetworkMessage::decode(data){
//                         println!("[{}] sent: {:?}", addr, msg);
//                         let self_for_msg = Arc::clone(&self);

//                         let mut msg_handled = false;
//                         {
//                             let mut guard = self.peers.write().await;
//                             if let Some(peer) = guard.get_mut(&addr){
//                                 if let Some(ref mut writer) = peer.writer{
//                                     self_for_msg.handle_message(msg, writer).await;
//                                     msg_handled = true;
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 Err(_) => break,
//             }
//         }
//         peers.write().await.remove(&addr);
//     }

//     pub async fn broadcast(&self, msg:NetworkMessage){
//         let data = msg.encode();
//         let mut peers = self.peers.write().await;
//         for peer in peers.values_mut(){
//             let _ = peer.send_data(&data).await;
//         }
//     }

//     pub async fn connect_to_peer(self: Arc<Self>, addr: SocketAddr){
//         println!("TRYING CONNECT TO OUTER NODE: {addr}");
//         match TcpStream::connect(addr).await{
//             Ok(socket) =>{
//                 println!("CONNECTED: {}", addr);
//                 let (reader, mut writer) = socket.into_split();
//                 let get_peers_msg = NetworkMessage::GetPeers;
//                 Self::send_message(&mut writer, &get_peers_msg).await;
//                 let mut peer = Peer::new(addr);
//                 peer.writer = Some(writer);
//                 peer.status = PeerStatus::Connected;

//                 let peers_clone = Arc::clone(&self.peers);
//                 peers_clone.write().await.insert(addr, peer);

//                 let peers_for_handler = Arc::clone(&self.peers);
//                 let self_clone = Arc::clone(&self);
//                 tokio::spawn(async move{
//                     self_clone.handle_peer(addr,reader, peers_for_handler).await;
//                 });
//             }
//             Err(e) => {println!("CONNECTION FAILED [{}]: {}", addr, e);}
//         }
//     }

//     async fn send_message(writer: &mut OwnedWriteHalf, msg: &NetworkMessage){
//         let bin = msg.encode();
//         let _ = writer.write_all(&bin).await;
//         let _ = writer.flush().await;
//     }

//     async fn handle_message(self: Arc<Self>, msg: NetworkMessage, writer: &mut OwnedWriteHalf){
//         match msg{
//             NetworkMessage::GetPeers =>{
//                 let mut rng = rand::rng();
//                 let guard = self.peers.read().await;              
//                 let mut all_addrs: Vec<SocketAddr> = guard.keys().cloned().collect();
//                 drop(guard);

//                 all_addrs.shuffle(&mut rng);
//                 let selected_peers = all_addrs.into_iter().take(20).collect();
                
//                 let response = NetworkMessage::Peers(selected_peers);
//                 Self::send_message(writer, &response).await;
//                 println!("신규 접속 노드에게 주소 보냄");
//             },
//             NetworkMessage::Peers(received_addrs) => {
//                 let known_peers = self.peers.read().await;
//                 for addr in received_addrs{
//                     if !known_peers.contains_key(&addr) && addr.port() != self.port{
//                         println!("New peer found, connect into: {}", addr);
//                         let node_clone = Arc::clone(&self);
//                         tokio::spawn(async move{
//                             node_clone.connect_to_peer(addr).await;
//                         });
//                     }
//                 }
//             },
//             _ => return
//         }
//     }

// }
