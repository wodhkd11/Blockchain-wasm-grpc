use axum::{Json, Router, extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{post, get}};
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use serde_with::serde_as;
use sha3::{Digest, Keccak256};
use tower_http::cors::{Any, CorsLayer};
use std::sync::Arc;
use crate::{block::{types::Hash, transaction::TransactionData}, network::{message::NetworkMessage, node::NodeManage}};
use hex;

#[derive(Deserialize)]
struct RpcRequest{
    method: String,
    params: Vec<serde_json::Value>,
    id: serde_json::Value,
}

async fn handle_eth_request(
    State(manager): State<Arc<NodeManage>>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse{
    match req.method.as_str(){
        "eth_chainId" => {
            let hex_chain_id = format!("0x{:x}", 555);
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": hex_chain_id
            }))
        },
        _ => Json(json!({"jsonrpc": "2.0", "id": req.id, "error": "Method NOt FOund"})),
    }.into_response()
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TransactionRequest{
    pub sender: [u8; 20],
    pub receiver: [u8; 20],
    pub value: u64,
    pub nonce: u64,
    pub payload: Vec<u8>,
    #[serde_as(as = "[_; 65]")]
    pub signature: [u8; 65],
} // after get string, to_hex and do Transaction::New()

pub async fn start_rpc_server(manager: Arc<NodeManage>, rpc_port: u16){
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/transaction", post(handle_tx_submission))
        .route("/nonce/{address}", get(get_nonce_handler))
        .layer(cors)
        .with_state(manager);
    let addr = format!("0.0.0.0:{}", rpc_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("[RCP SERVER]: running at http://{addr}");
    axum::serve(listener, app).await.unwrap();
}


/**
 * This function gets transaction and broadcast.
 * This function returns transaction's hash value.
 */
async fn handle_tx_submission(
    State(manager): State<Arc<NodeManage>>,
    Json(payload): Json<TransactionRequest>,
) -> Result<Json<Hash>, StatusCode>{
    let mut hasher = Keccak256::new();
    hasher.update(&payload.signature);
    let sig_hash: [u8; 32] = hasher.finalize().into();
    let tx = payload.to_core_data().ok_or(StatusCode::UNAUTHORIZED)?;
    let tx_id = tx.calculate_hash();
    let sender_address = tx.sender;

    {
        let mut state = manager.state.write().await;
        let storage = state.storage.clone();
        let storage_clone = storage.clone();
        let _ = match state.global_state.check_nocne(&sender_address, tx.nonce, &storage_clone){
            true => {
                if state.mempool.contains_key(&sig_hash){ return Err(StatusCode::CONFLICT); }
                state.mempool.insert(tx_id, tx.clone());               
            },
            false => {
                println!("[RPC]: Nonce mismatch");
                return Err(StatusCode::BAD_REQUEST);
            },
        };
        //Commit 시 해야됨 state.global_state.increase_nonce_only(tx.sender, storage_clone);
        
    }
    let msg = NetworkMessage::NewTransaction(tx.clone());
    let manager_clone = manager.clone();
    

    tokio::spawn(async move{manager_clone.broadcast(msg).await;});
    Ok(Json(sig_hash))
}

async fn get_nonce_handler(
    State(manager): State<Arc<NodeManage>>,
    Path(address): Path<String>,
) -> impl IntoResponse{
    let address = hex_to_address(&address);
    let mut manager_clone = manager.state.write().await;
    let storage = manager_clone.storage.clone();
    let nonce = manager_clone.global_state.get_nonce(&address, &storage);
    Json(nonce)
}

fn hex_to_address(hex_str: &String) -> [u8;20]{
    let clean_hex = hex_str.trim_start_matches("0x");
    let decoded = hex::decode(clean_hex).expect("INVALID HEX");
    let mut address = [0u8;20];
    address.copy_from_slice(&decoded[..20]);
    address
}

impl TransactionRequest{
    pub fn to_core_data(&self) -> Option<TransactionData>{
        if !self.verify_signature(){
            println!("[RPC]: INVALID SIGNATURE");
            return None;
        }

        //is signed?
        Some(TransactionData::new(
            self.sender,
            self.receiver,
            self.value,
            self.payload.clone(),
            self.nonce,
            self.signature,
        ))
    }
    fn verify_signature(&self) -> bool{
        let mut v = Vec::new();
        v.extend_from_slice(&self.sender);
        v.extend_from_slice(&self.receiver);
        v.extend_from_slice(&self.value.to_be_bytes());
        v.extend_from_slice(&self.nonce.to_be_bytes());
        v.extend_from_slice(&self.payload);

        crate::crypto::signature::verify(self.sender, &self.signature, &v)
    }
}