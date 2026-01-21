use std::{net::SocketAddr, sync::Arc};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpStream, tcp::OwnedReadHalf}};
use crate::network::{message::NetworkMessage, node::NodeManage, peer::Peer};


impl NodeManage{
    pub async fn connect_to_peer(self: Arc<Self>, addr: SocketAddr) -> Result<OwnedReadHalf, String>{
        {
            let state = self.state.read().await;
            if state.peers.len() >= state.max_peers{
                return Err("Max peers reached".to_string());
            }
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



    pub async fn send_to(&self, addr:SocketAddr, msg: NetworkMessage){
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
}
