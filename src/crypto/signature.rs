use k256::{ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner}};
use sha3::{Keccak256, Digest};

use crate::{PRIVATE_KEY, block::types::Address};

//pub fn sign(block_hash: &Hash) -> Result<Vec<u8>, String>{
    //let signing_key = SigningKey::from_bytes(private_key_bytes.into()).expect("INVALID PRIVATE KEY");

//}


pub fn sign(hash: &[u8;32] ) -> Result<[u8;65], String>{
    let priv_bytes = PRIVATE_KEY.get().expect("");
    let signing_key = SigningKey::from_bytes(priv_bytes.into()).expect("");

    let (signature, recoveryid) = signing_key
        .sign_prehash(hash)
        .expect("");
    let mut sig_bytes = [0u8;65];
    sig_bytes[..64].copy_from_slice(&signature.to_bytes());
    sig_bytes[64] = recoveryid.to_byte() + 27;
    Ok(sig_bytes)

}

pub fn verify(
    sender: Address,
    signature_bytes: &[u8; 65],
    message: &[u8],
) -> bool{
    let prefix = format!("\x19Ethereum Signed Message:\n{}",message.len());
    
    let mut eth_message = Vec::new();
    eth_message.extend_from_slice(prefix.as_bytes());
    eth_message.extend_from_slice(message);


    let msg_hash = Keccak256::digest(&eth_message);
    let sig = match Signature::from_slice(&signature_bytes[..64]){
        Ok(s) => s,
        Err(_) => return false,
    };

    let v = signature_bytes[64];
    let rec_id = if v >= 27 { v - 27} else {v};

    let recovery_id = match RecoveryId::from_byte(rec_id){
        Some(id) => id,
        None => return false,
    };

    let recovered_key = match VerifyingKey::recover_from_prehash(&msg_hash, &sig, recovery_id){
        Ok(key) => key,
        Err(_) => return false,
    };
    let recovered_address = public_key_to_address(&recovered_key);
    recovered_address == sender

}

pub fn verify_for_block(
    validator: Address,
    signature_bytes: &[u8; 65],
    block_hash: &[u8;32],
) -> bool{
    let sig = match Signature::from_slice(&signature_bytes[..64]){
        Ok(s) => s,
        Err(_) => return false,
    };
    let v = signature_bytes[64];
    let rec_id_byte = if v >= 27 { v - 27 } else {v};
    let recoveryid = match RecoveryId::from_byte(rec_id_byte){
        Some(id) => id,
        None => return false,
    };
    let recovered_key = match VerifyingKey::recover_from_prehash(block_hash, &sig, recoveryid){
        Ok(key) => key,
        Err(_) => return false,
    };
    let recovered_addr = public_key_to_address(&recovered_key);
    recovered_addr == validator
}


fn public_key_to_address(verifying_key: &VerifyingKey) -> [u8; 20]{
    let encoded = verifying_key.to_encoded_point(false);
    let public_key_bytes = &encoded.as_bytes()[1..];
    let hash = Keccak256::digest(public_key_bytes);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    address
}