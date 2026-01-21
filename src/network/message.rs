use serde::{Serialize, Deserialize};
use crate::block::{types::BlockData, transaction::TransactionData};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkMessage{
    Hello {listening_port: u16},
    Status{chain_id: u64, best_height: u64, best_hash: [u8;32],},
    NewTransaction(TransactionData), //트랜잭션 전파
    NewBlock(BlockData), // 블록 전파
    GetPeers, //외부의 노드들에 대해 피어 공유 요청
    Peers(Vec<SocketAddr>), //보유한 노드들의 주소 공유
    Ping,
    Pong,
    GetBlocks{request_id: u64, start_height: Option<u64>, count: Option<u64>,},
    Inventory(Vec<BlockData>),
}

impl NetworkMessage{
    pub fn serialize(&self) -> Vec<u8>{
        postcard::to_allocvec(self).expect("Serialization failed")
    }

    pub fn encode(&self) -> Vec<u8>{
        let body = self.serialize();
        let len = body.len() as u32;

        let mut packet = Vec::with_capacity(4+body.len());;
        packet.extend_from_slice(&len.to_le_bytes());
        packet.extend_from_slice(&body);
        packet
    }

    pub fn decode(data: &[u8]) -> Option<Self>{
        postcard::from_bytes(data).ok()
    }

    pub fn decode_with_bytes(src: &[u8]) -> Option<(Self, usize)>{
        if src.len() < 4 { return None; }
        let len_bytes = src[..4].try_into().ok()?;
        let message_len = u32::from_le_bytes(len_bytes) as usize;
        if message_len > 10*1024*1024{
            return None;
        }
        if src.len() < 4 + message_len{return None;}
        let actual_data = &src[4..4+message_len];
        match NetworkMessage::decode(actual_data){
            Some(msg) => Some((msg, 4+message_len)),
            None => {
                println!("Can't decode");
                None
            }
        }
    }

    pub fn get_id(&self) -> Vec<u8>{
        use sha3::{Keccak256, Digest};
        let mut hasher = Keccak256::new();
        hasher.update(self.serialize());
        hasher.finalize().to_vec()
    }
}