mod network;
mod block; // 기존 블록 모듈

use network::node::Node;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::time::{sleep, Duration};

use crate::network::node::NodeManage;

#[tokio::main]
async fn main() {
    // 1. 명령줄 인자로 내 포트와 접속할 상대방 포트를 받는다고 가정합니다.
    // 예: cargo run -- 8080 8081 (내 포트 8080, 접속할 상대 8081)
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("❌ 사용법: cargo run -- <내_포트> [시드_주소_1] [시드_주소_2] ...");
        println!("예시: cargo run -- 9000 127.0.0.1:9001");
        return;
    }

    // 2. 내 포트 설정
    let my_port: u16 = args[1].parse().expect("포트 번호는 숫자여야 합니다.");
    let my_addr_str = format!("127.0.0.1:{}", my_port);

    // 3. 시드 노드 주소들 추출 (2번째 인자부터 끝까지)
    let seeds: Vec<&str> = args.iter().skip(2).map(|s| s.as_str()).collect();

    println!("🚀 노드 가동 준비 중...");
    println!("📍 내 주소: {}", my_addr_str);
    println!("🌱 시드 노드 목록: {:?}", seeds);

    // 4. NodeManage 생성 및 실행
    let node_manager = Arc::new(NodeManage::new(my_port, &my_addr_str));
    
    // start 함수가 내부적으로 Discovery를 수행하고 리스닝 루프를 돌립니다.
    node_manager.start(seeds).await;
    

    /*블록체인관련되거 */
    // let args: Vec<String> = std::env::args().collect();
    // let my_port = args.get(1).expect("내 포트를 입력하세요 (예: 8080)");
    // let target_port = args.get(2); // 접속할 상대방은 없을 수도 있음

    // let my_addr = format!("127.0.0.1:{}", my_port);
    // let node = Node::new(my_port.parse().unwrap() ,&my_addr);

    // // 2. 내 노드 서버 시작 (비동기로 실행)
    // let node_clone = std::sync::Arc::new(node);
    // let node_for_start = node_clone.clone();
    
    // tokio::spawn(async move {
    //     node_for_start.start().await;
    // });

    // // 서버가 뜰 때까지 잠깐 대기
    // sleep(Duration::from_secs(1)).await;

    // // 3. 만약 접속할 상대방 주소가 있다면 연결 시도
    // if let Some(port) = target_port {
    //     let target_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    //     node_clone.clone().connect_to_peer(target_addr).await;
    // }

    // // 4. 연결 유지 및 테스트 메시지 전송 루프
    // loop {
    //     tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    //     if node_clone.peers.read().await.len()>0{
    //         println!("SEND TO ALL PEERS ...");
    //         node_clone.broadcast(network::message::NetworkMessage::Ping).await;
    //     }
    //     // 여기서 나중에 node_clone.broadcast(...)를 테스트할 수 있습니다.
    //     println!("현재 연결된 피어 수: {}", node_clone.peers.read().await.len());
    // }
}