use axum::{Json, Router, extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{post, get}};
use primitive_types::U256;
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use serde_with::{serde_as, hex::Hex};
use sha3::{Digest, Keccak256};
use tower_http::cors::{Any, CorsLayer};
use std::{collections::HashMap, sync::Arc};
use crate::{block::{types::{Balance, Hash}, transaction::TransactionData}, network::{message::NetworkMessage, node::NodeManage}};
use hex;

#[derive(Deserialize, Debug)]
struct RpcRequest{
    method: String,
    params: Vec<serde_json::Value>,
    id: serde_json::Value,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TransactionRequest{
    #[serde_as(as = "serde_with::hex::Hex")]
    pub sender: [u8; 20],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub receiver: [u8; 20],
    pub value: Balance,
    pub nonce: u64,
    pub payload: Vec<u8>,
    #[serde_as(as = "Vec<serde_with::hex::Hex>")]
    pub signature: Vec<[u8; 65]>,
} // after get string, to_hex and do Transaction::New()

pub async fn start_rpc_server(manager: Arc<NodeManage>, rpc_port: u16){
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", post(handle_eth_request))
        .route("/transaction", post(handle_tx_submission))
        .route("/nonce/{address}", get(get_nonce_handler))
        .route("/dashboard/state", get(get_all_state_handler))
        .route("/admin", post(update_network_config))
        .layer(cors)
        .with_state(manager);
    let addr = format!("0.0.0.0:{}", rpc_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("[RCP SERVER]: running at http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_eth_request(
    State(manager): State<Arc<NodeManage>>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    println!("{:?}", req);
    let ret = match req.method.as_str() {
        // 1. 체인 ID 응답 (메타마스크 네트워크 연결용)
        "eth_chainId" => {
            let chain_id = { manager.state.read().await.chain_id };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", chain_id)
            })).into_response()
        },

        // 2. 네트워크 버전 응답
        "net_version" => {
            let chain_id = { manager.state.read().await.chain_id };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": chain_id.to_string()
            })).into_response()
        },

        // 3. 최신 블록 높이 응답 (이게 응답되어야 잔액 조회가 시작됨)
        "eth_blockNumber" => {
            let height = { manager.state.read().await.block_height };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", height)
            })).into_response()
        },

        // 4. 기본 잔액(Native Token, 예: GOV) 응답
        "eth_getBalance" => {
            let addr_str = req.params.get(0).and_then(|v| v.as_str()).unwrap_or("");
            let address = hex_to_address(&addr_str.to_string());
            
            let state = manager.state.read().await;
            let balance = state.global_state.read().await.balances.get(&address)
                .and_then(|acc| acc.balance.get(&"GOV".to_string()))
                .cloned()
                .unwrap_or(Balance::zero());

            // [핵심] 메타마스크 18자리 소수점 보정 (1 GOV -> 10^18 wei)
            let display_balance = balance * Balance::from(10).pow(Balance::from(18));
            let test_balance = Balance::from(777) * Balance::from(10).pow(Balance::from(18));

            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", test_balance)
            })).into_response()
        },

        // 5. 커스텀 토큰(예: KRW) 잔액 및 기타 호출 응답
        "eth_call" => {
            let Some(params) = req.params.get(0) else {
                return Json(json!({"jsonrpc": "2.0", "id": req.id, "error": "no params"})).into_response();
            };
            let to_address = params.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let data = params.get("data").and_then(|v| v.as_str()).unwrap_or("");

            // balanceOf(address) 요청 파싱 (시그니처: 0x70a08231)
            if data.starts_with("0x70a08231") {
                let user_addr_str = &data[34..];
                let user_addr = hex_to_address(&user_addr_str.to_string());
                
                let state = manager.state.read().await;
                let token_symbol = match to_address.to_lowercase().as_str() {
                    "0x0000000000000000000000000000000000000001" => "KRW",
                    _ => "GOV",
                };

                let balance = state.global_state.read().await.balances.get(&user_addr)
                    .and_then(|acc| acc.balance.get(&token_symbol.to_string()))
                    .cloned()
                    .unwrap_or(Balance::zero());

                // [핵심] 토큰도 18자리 보정 + 32바이트 패딩 응답
                let display_balance = balance * Balance::from(10).pow(Balance::from(18));
                return Json(json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": format!("0x{:0>64x}", display_balance)
                })).into_response();
            }

            // 그 외 알 수 없는 eth_call에 대한 기본 응답 (에러 방지용)
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": "0x0000000000000000000000000000000000000000000000000000000000000000"
            })).into_response()
        },
        "eth_getBlockByNumber" => {
            // params: [block_height_hex, full_tx_obj_bool]
            let height_hex = req.params.get(0).and_then(|v| v.as_str()).unwrap_or("0x0");
            let height = u64::from_str_radix(height_hex.trim_start_matches("0x"), 16).unwrap_or(0);

            let state = manager.state.read().await;
            // 실제 저장된 블록이 있으면 가져오고, 없으면 마지막 블록이나 제네시스를 임시로 반환
            let block = &state.last_block; 

            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "number": format!("0x{:x}", height),
                    "hash": format!("0x{}", hex::encode(block.hash)),
                    "parentHash": format!("0x{}", hex::encode(block.header.prev_block_hash)),
                    "timestamp": "0x65ada600", // 임시 타임스탬프
                    "transactions": [], // 일단 빈 배열로 응답해도 무방
                    "gasLimit": "0xffffff",
                    "gasUsed": "0x0"
                }
            })).into_response()
        },
        _ => {
            Json(json!({
                "jsonrpc": "2.0", 
                "id": req.id, 
                "error": {"code": -32601, "message": "Method not found"}
            })).into_response()
        }
    };
    println!("{:?}",ret);
    ret
}

/**
 * This function gets transaction and broadcast.
 * This function returns transaction's hash value.
 */
async fn handle_tx_submission(
    State(manager): State<Arc<NodeManage>>,
    Json(payload): Json<TransactionRequest>,
) -> Result<Json<Hash>, StatusCode>{
    let tx = payload.to_core_data().ok_or(StatusCode::UNAUTHORIZED)?;
    let tx_id = tx.calculate_hash();
    
    {
        let mut node_state = manager.state.write().await;
        let storage = &node_state.storage;
        
        let is_nonce_valid = {
            let global_state= node_state.global_state.read().await;
            global_state.check_nonce(&tx.sender, tx.nonce, node_state.block_height + 1, storage)
        }.map_err(|e| StatusCode::BAD_REQUEST)?;
        if !is_nonce_valid{ return Err(StatusCode::BAD_REQUEST); }
        if node_state.mempool.contains_key(&tx_id){ return Err(StatusCode::CONFLICT); }
        node_state.mempool.insert(tx_id, tx.clone());
    }
    let manager_clone = manager.clone();
    let msg = NetworkMessage::NewTransaction(tx.clone());
    tokio::spawn(async move{manager_clone.broadcast(msg).await;});
    Ok(Json(tx_id))
}

async fn get_nonce_handler(
    State(manager): State<Arc<NodeManage>>,
    Path(address): Path<String>,
) -> impl IntoResponse{
    let address = hex_to_address(&address);
    let node_state = manager.state.read().await;
    let storage = &node_state.storage;

    let mut global_state = node_state.global_state.write().await;
    match global_state.get_nonce_safe(&address, node_state.block_height+1, storage){
        Ok(nonce) => {
            Json(json!({
                "address": address,
                "nonce": nonce,
                "nocne_hex": format!("0x{:?}", nonce)
            })).into_response()
        },
        Err(e) => {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to get nonce",
                    "details": format!("{:?}", e)
                }))
            ).into_response()
        }
    }
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
            self.signature.clone(),
        ))
    }
    fn verify_signature(&self) -> bool{
        let mut v = Vec::new();
        v.extend_from_slice(&self.sender);
        v.extend_from_slice(&self.receiver);
        let value_bytes: [u8; 32] = self.value.to_big_endian();
        v.extend_from_slice(&value_bytes);
        v.extend_from_slice(&self.nonce.to_be_bytes());
        v.extend_from_slice(&self.payload);
        // println!("[DEBUG] Backend Message Bytes: 0x{}", hex::encode(&v));
        // println!("[DEBUG] Sender: 0x{}", hex::encode(&self.sender));
        // println!("[DEBUG] Signature: 0x{}", hex::encode(&self.signature));
        for sig in &self.signature{
            if !crate::crypto::signature::verify(self.sender, sig, &v){ return false; }
        }
        true
    }
}

async fn update_network_config() {
    
}

async fn get_all_state_handler(
    State(manager): State<Arc<NodeManage>>,
) -> impl IntoResponse {
    let node_state = manager.state.read().await;
    let global_state = node_state.global_state.read().await;

    // 계정 주소와 각 계정의 잔고 맵을 정리하여 반환
    let mut accounts_info = HashMap::new();
    
    // global_state.balances는 HashMap<Address, Account> 형태라고 가정합니다.
    for (address, account) in &global_state.balances {
        let addr_hex = format!("0x{}", hex::encode(address));
        let mut formatted_balances = HashMap::new();

        for (symbol, val) in &account.balance {
            formatted_balances.insert(symbol.clone(),format!("0x{:x}", val));
        }

        accounts_info.insert(addr_hex, json!({
            "nonce": account.nonce,
            "balances": formatted_balances
        }));
    }

    Json(json!({
        "height": node_state.block_height,
        "mempool_count": node_state.mempool.len(),
        "accounts": accounts_info,
        "last_block_hash": format!("0x{}", hex::encode(node_state.last_block.hash))
    }))
}

// async fn get_all_state_handler(
//     State(manager): State<Arc<NodeManage>>,
// ) -> impl IntoResponse {
//     let node_state = manager.state.read().await;
//     let global_state = node_state.global_state.read().await;

//     // 계정 주소와 각 계정의 잔고 맵을 정리하여 반환
//     let mut accounts_info = HashMap::new();
    
//     // global_state.balances는 HashMap<Address, Account> 형태라고 가정합니다.
//     for (address, account) in &global_state.balances {
//         let addr_hex = format!("0x{}", hex::encode(address));
//         accounts_info.insert(addr_hex, json!({
//             "nonce": account.nonce,
//             "balances": account.balance // HashMap<String, u64>
//         }));
//     }

//     Json(json!({
//         "height": node_state.block_height,
//         "mempool_count": node_state.mempool.len(),
//         "accounts": accounts_info,
//         "last_block_hash": format!("0x{}", hex::encode(node_state.last_block.hash))
//     }))
// }