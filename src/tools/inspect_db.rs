
use rocksdb::DB;
use serde_json; // 또는 bincode (저장 시 사용한 라이브러리)
// 프로젝트의 실제 구조체 경로를 가져옵니다.
// 예: use blockPractice::block::types::{BlockData, Account}; 

fn main() {
    let path = "./data/node_9000";
    let db = DB::open_for_read_only(&rocksdb::Options::default(), path, false).unwrap();
    let iter = db.iterator(rocksdb::IteratorMode::Start);

    println!("--- DB 데이터 분석 결과 ---");
    for item in iter {
        let (key, value) = item.unwrap();
        let key_str = String::from_utf8_lossy(&key);

        // 1. 계정 정보 (Prefix가 'a' 또는 'acc_'인 경우)
        if key_str.starts_with('a') {
            println!("Type: Account");
            println!("Key (Address): {}", hex::encode(&key[1..])); // prefix 제외하고 hex 출력
            // 저장할 때 JSON을 썼다면:
            if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&value) {
                println!("Value: {:#?}", json_val);
            } else {
                println!("Value (Raw Hex): {}", hex::encode(&value));
            }
        } 
        // 2. 블록 정보 (Prefix가 'b'인 경우)
        else if key_str.starts_with("i") {
            println!("Type: Index Key");
            println!("Raw Bytes (Hex): {}", hex::encode(&key)); 
    // 결과 예시: i0000000000000001 (실제로는 69 00 00 00 00 00 00 00 01 ...)
    
            let height = u64::from_be_bytes(key[1..9].try_into().unwrap());
            println!("Parsed Height: {}", height);
            println!("Value (Hex): {}", hex::encode(&value));
        }
        else if key_str.starts_with('b') {
            println!("Type: Block");
            println!("Key (Hash): {}", hex::encode(&key[1..]));
            // 블록 구조체로 변환 시도 (프로젝트 내부 구조체 필요)
            // if let Ok(block) = bincode::deserialize::<BlockData>(&value) { ... }
            println!("Value (Raw Hex): {}", &hex::encode(&value)); // 너무 길면 잘라서 출력
        }
        // 3. 기타 (last_block 등)
        else {
            println!("Key: {}", key_str);
            println!("Value (Hex): {}", hex::encode(&value));
        }
        println!("----------------------");
    }
}