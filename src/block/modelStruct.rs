use serde::{Deserialize, Serialize};

pub type Address = [u8; 20];
pub type Hash = [u8; 32];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionData{
    pub sender: Address,
    pub receiver: Address,
    pub amount: u64,
    pub nonce: u64, // 이중지불 방지용 트랜잭션 순서
    pub hash: Hash,
}

pub type Signature = Vec<u8>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader{
    pub height: u64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u64,
    pub valdiator: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockData{
    pub header: BlockHeader,
    pub body: Vec<TransactionData>,
    pub hash: Hash,
    pub signature: Signature,
}
