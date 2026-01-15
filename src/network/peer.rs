use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::net::tcp::OwnedWriteHalf;
use std::net::SocketAddr;
use std::time::SystemTime;
use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, PartialEq)]
pub enum PeerStatus{
    Connecting,
    Connected,
    Disconnected,
}

pub struct Peer{
    pub address: Option<SocketAddr>,
    pub status: PeerStatus,
    pub last_seen: SystemTime,
    pub writer: Option<OwnedWriteHalf>,
}

impl Peer{
    pub fn new(address: SocketAddr) -> Self{
        Self{
            address: Some(address),
            status: PeerStatus::Disconnected,
            last_seen: SystemTime::now(),
            writer: None,
        }
    }

    pub async fn send_data(&mut self, data: &[u8]) -> std::io::Result<()>{
        if let Some(ref mut w) = self.writer{
            w.write_all(data).await?;
            w.flush().await?;
            self.last_seen = SystemTime::now();
            Ok(())
        }else{
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "NOT CONNECTED WITH PEER"))
        }
    }

//    pub async fn read_data(&mut self) -> std::io::Result<Vec<u8>> {
//        if let Some(ref mut s) = self.stream{
//            let mut buffer = [0u8; 1024];
//            let n = s.read(&mut buffer).await?;
//            self.last_seen = SystemTime::now();
//            Ok(buffer[..n].to_vec())
//        }else{Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "NOT CONNECTED WITH PEER"))}
//    }

    pub async fn disconnect(&mut self){
        self.writer= None;
        self.status = PeerStatus::Disconnected;
    }
}