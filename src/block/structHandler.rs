

use std::time::{SystemTime, UNIX_EPOCH};

use crate::block::modelStruct::*;
use sha2::{Sha256, Digest};

impl TransactionData{
    pub fn new(sender: [u8;20], receiver: [u8; 20], amount: u64, nonce: u64) -> Self{
        let mut tx = Self{
            sender,
            receiver,
            amount,
            nonce,
            hash: [0u8; 32],
        };
        tx.hash = tx.calculate_hash();
        tx
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.sender);
        hasher.update(&self.receiver);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());

        let result = hasher.finalize();
        let mut hash_res = [0u8; 32];
        hash_res.copy_from_slice(&result);
        hash_res
    }
}

impl BlockData{
    pub fn new(
        prev_block: &BlockData,
        transactions: Vec<TransactionData>,
        valdiator: Address
    ) -> Self{
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let merkle_root = [0u8; 32];
        let header = BlockHeader{
            height: prev_block.header.height + 1,
            prev_block_hash: prev_block.hash,
            merkle_root,
            timestamp,
            valdiator,
        };

        let block_hash = Self::calculate_header_hash(&header);
        let dummy_signature = vec![0u8; 64];

        BlockData {
            header,
            body: transactions,
            hash: block_hash,
            signature: dummy_signature,
        }
    }

    pub fn create_genesis_block(valdiator:Address) -> Self{
        let timestamp = 100;
        let prev_block_hash = [0u8; 32];
        let transactions = vec![];
        let merkle_root = [0u8; 32];
        let header = BlockHeader{
            height:0,
            prev_block_hash,
            merkle_root,
            timestamp,
            valdiator,
        };
        let block_hash = Self::calculate_header_hash(&header);
        let genesis_signature = vec![0u8; 64];
        BlockData{
            header,
            body: transactions,
            hash: block_hash,
            signature: genesis_signature,
        }
    }

    pub fn calculate_header_hash(header: &BlockHeader) -> Hash{
        let mut hasher = Sha256::new();
        hasher.update(&header.prev_block_hash);
        hasher.update(&header.merkle_root);
        hasher.update(&header.timestamp.to_be_bytes());
        hasher.update(&header.valdiator);
        hasher.finalize().into()
    }
}