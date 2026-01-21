mod network;
mod block;
mod crypto;
mod exec;

use std::{env, fs, sync::{Arc, OnceLock}};
use once_cell::sync::Lazy;
use serde::Deserialize;
use network::node::NodeManage;

#[derive(Debug, Deserialize)]
struct AppConfig {
    node: NodeConfig,
    network: NetworkConfig,
    storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
struct NodeConfig {
    ip_address: String,
    port: u16,
    rpc_port: u16, // RPC 포트 추가 (YAML에 있다고 가정)
    wallet_address: String,
    private: String,
    node_type: String,
    is_genesis: bool,
}

#[derive(Debug, Deserialize)]
struct NetworkConfig {
    boot_nodes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StorageConfig {
    db_path: String,
}

static PRIVATE_KEY: OnceLock<[u8;32]> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 설정 파싱 (기존 로직 유지)
    let args: Vec<String> = env::args().collect();
    let config_path = args.iter().position(|x| x == "--config")
        .map(|pos| &args[pos + 1]).unwrap();
    let yaml_str = fs::read_to_string(config_path)?;
    let config: AppConfig = serde_yaml::from_str(&yaml_str)?;
    let wallet_bytes = decode_address(&config.node.wallet_address);
    let private_bytes = decode_private(&config.node.private);
    PRIVATE_KEY.set(private_bytes).ok();
    // 2. NodeManage 생성
    let manager = Arc::new(NodeManage::new(
        config.node.port,
        &config.node.ip_address,
        wallet_bytes,
        &config.storage.db_path,
        config.node.is_genesis,
    ));

    // 3. 지저분한 spawn 다 치우고 딱 하나만 실행
    println!("🚀 노드를 시작합니다 (포트: {})", config.node.port);
    
    // manager.start 내부에 이미 miner, rpc, heartbeat 스폰 로직이 다 들어있으므로
    // 여기서 start를 await 하면 내부 루프(listener.accept)까지 쭉 실행됩니다.
    let seeds: Vec<&str> = config.network.boot_nodes.iter().map(|s| s.as_str()).collect();
    manager.start(seeds).await;

    Ok(())
}

fn decode_address(addr: &str) -> [u8; 20] {
    let clean_addr = addr.trim_start_matches("0x");
    // 16진수 문자열은 20바이트일 때 길이가 40이어야 합니다.
    if clean_addr.len() != 40 {
        panic!("INVALID_ADDRESS_LENGTH: Expected 40 hex chars, got {}", clean_addr.len());
    }
    let mut bytes = [0u8; 20];
    let decoded = hex::decode(clean_addr).expect("WRONG_WALLET_FORMAT");
    bytes.copy_from_slice(&decoded);
    bytes
}


fn decode_private(addr: &str) -> [u8; 32] {
    let clean_addr = addr.trim_start_matches("0x");
    // 16진수 문자열은 20바이트일 때 길이가 40이어야 합니다.
    if clean_addr.len() != 64 {
        panic!("INVALID_ADDRESS_LENGTH: Expected 40 hex chars, got {}", clean_addr.len());
    }
    let mut bytes = [0u8; 32];
    let decoded = hex::decode(clean_addr).expect("WRONG_WALLET_FORMAT");
    bytes.copy_from_slice(&decoded);
    bytes
}

//mod network;
//mod block; // 기존 블록 모듈
//mod crypto;
//mod exec;
//use network::node::Node;
//use serde::Deserialize;
//use std::{env, fs, net::SocketAddr, sync::Arc};
//use tokio::time::{sleep, Duration};

//use crate::{ network::node::NodeManage};

//#[derive(Debug, Deserialize)]
//struct AppConfig{
    //node: NodeConfig,
    //network: NetworkConfig,
    //storage: StorageConfig,
//}
//#[derive(Debug, Deserialize)]
//struct NodeConfig{
    //ip_address: String,
    //port: u16,
    //wallet_address: String,
    //node_type: String,
    //is_genesis: bool,
//}
//#[derive(Debug, Deserialize)]
//struct NetworkConfig{boot_nodes: Vec<String>,}
//#[derive(Debug, Deserialize)]
//struct StorageConfig{db_path: String,}




//#[tokio::main]
//async fn main() {
    //// 1. 명령줄 인자로 내 포트와 접속할 상대방 포트를 받는다고 가정합니다.
    //// 예: cargo run -- 8080 8081 (내 포트 8080, 접속할 상대 8081)
    //let args: Vec<String> = env::args().collect();
    //let config_path = if let Some(pos) = args.iter().position(|x| x == "--config"){
        //&args[pos + 1]
    //}else{
        //"config.yaml"
    //};
    //let yaml_str = fs::read_to_string(config_path)
        //.expect("CONFIG_FILE_ERROR");
    //let config: AppConfig = serde_yaml::from_str(&yaml_str)
        //.expect("YAML_PARSING_ERROR");
    //let wallet_bytes = decode_address(&config.node.wallet_address);


    //// }
//}

//fn decode_address(addr:&str) -> [u8;20]{
    //let clean_addr = addr.trim_start_matches("0x");
    //if clean_addr.len() != 20 {panic!("INVALID_ADDRESS");}
    //let mut bytes = [0u8;20];
    //let decoded = hex::decode(clean_addr).expect("WRONG_WALLET_FORMAT");
    //bytes.copy_from_slice(&decoded);
    //bytes
//}