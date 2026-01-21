use std::time::{SystemTime, UNIX_EPOCH};

use crate::{PRIVATE_KEY, block::{transaction::{ConfirmedTransaction, TransactionData}, types::*}, crypto::signature};
use sha3::{Keccak256, Digest};


impl BlockData{
    pub fn new(
        prev_block: &BlockData,
        transactions: Vec<ConfirmedTransaction>,
        valdiator: Address
    ) -> Self{
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let merkle_root = Self::calculate_merkle_root(transactions.clone());
        let header = BlockHeader{
            height: prev_block.header.height + 1,
            prev_block_hash: prev_block.hash,
            merkle_root,
            timestamp,
            valdiator,
        };

        let block_hash = Self::calculate_header_hash(&header);
        let signature = signature::sign(&block_hash).unwrap();

        BlockData {
            header,
            body: transactions,
            hash: block_hash,
            signature: signature,
        }
    }

    pub fn calculate_merkle_root(transactions: Vec<ConfirmedTransaction>) -> [u8;32]{
        if transactions.is_empty(){return [0u8;32];}
        let mut hashes: Vec<[u8;32]> = transactions
            .iter()
            .map(|tx| tx.hash)
            .collect();
        while hashes.len() > 1{
            if hashes.len() % 2 != 0{
                hashes.push(*hashes.last().unwrap());
            }
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2){
                let mut hasher = Keccak256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                let result = hasher.finalize();
                let mut node_hash = [0u8;32];
                node_hash.copy_from_slice(&result);
                next_level.push(node_hash);
            }
            hashes = next_level;
        }
        hashes[0]
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
        let genesis_signature = [0u8; 65];
        BlockData{
            header,
            body: transactions,
            hash: block_hash,
            signature: genesis_signature,
        }
    }

    pub fn calculate_header_hash(header: &BlockHeader) -> Hash{
        let mut hasher = Keccak256::new();
        hasher.update(&header.prev_block_hash);
        hasher.update(&header.merkle_root);
        hasher.update(&header.timestamp.to_be_bytes());
        hasher.update(&header.valdiator);
        hasher.finalize().into()
    }
}
