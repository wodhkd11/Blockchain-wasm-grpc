use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Bytes};
use sha3::{Digest, Keccak256};
use crate::{block::types::{Address, Balance, Hash}, crypto::signature};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionData{
    pub sender: Address,
    pub receiver: Address,
    pub value: Balance,
    pub nonce: u64, // 이중지불 방지용 트랜잭션 순서
    pub payload: Vec<u8>,
    #[serde_as(as = "Vec<Bytes>")]
    pub signature: Vec<[u8; 65]>,
}

impl TransactionData{
    pub fn new(sender: [u8;20], receiver: [u8; 20], value: Balance, payload: Vec<u8>, nonce: u64, signature: Vec<[u8; 65]> ) -> Self{
        let tx = Self{
            sender,
            receiver,
            value,
            nonce,
            payload,
            signature,
        };
        tx
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = Keccak256::new();
        hasher.update(&self.sender);
        hasher.update(&self.receiver);
        hasher.update(&self.value.to_big_endian());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(&self.payload);
        //hasher.update(&self.signature);

        let result = hasher.finalize();
        let mut hash_res = [0u8; 32];
        hash_res.copy_from_slice(&result);
        hash_res
    }

    pub fn generate_payload_to_bytes(&self) -> Vec<u8>{
        let mut v = Vec::new();
        v.extend_from_slice(&self.sender);
        v.extend_from_slice(&self.receiver);
        v.extend_from_slice(&self.value.to_big_endian());
        v.extend_from_slice(&self.nonce.to_be_bytes());
        v.extend_from_slice(&self.payload);
        v
    }
    
    pub fn verify(&self) -> bool{
        let message = self.generate_payload_to_bytes();
        println!("{:?}", message);

        for sig in &self.signature{
            if !signature::verify(self.sender, sig, &message){
                return false;
            }
        }
        true
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfirmedTransaction{
    pub tx_info: TransactionData,
    pub hash: Hash,
}

impl ConfirmedTransaction{
    /**
     * This function gets TransactionData 
     * and returns ConfirmedTransaction
     */
    pub fn from(tx: &TransactionData) -> Self{
        let tx_hash = tx.calculate_hash();
        Self{
            tx_info: tx.clone(),
            hash: tx_hash,
        }
    }

}

