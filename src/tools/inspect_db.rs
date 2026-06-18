use rocksdb::{DB, Options};
use std::fs::File;
use std::io::{Write, BufWriter};

fn main() {
    let path = "./data/node_9000";
    let output_path = "db_inspect_report.txt";

    let mut opts = Options::default();
    let db = DB::open_for_read_only(&opts, path, false).expect("DB Open Failed");
    let iter = db.iterator(rocksdb::IteratorMode::Start);

    let file = File::create(output_path).expect("파일 생성 실패");
    let mut writer = BufWriter::new(file);

    writeln!(writer, "\n{:=^60}", " REAL-TIME BLOCKCHAIN STORAGE REPORT ").unwrap();

    for item in iter {
        let (key, value) = item.expect("Iterator error");
        if key.is_empty() { continue; }

        // key[0]가 'v', 'b', 'a' 등 약속된 프리픽스인지 확인
        let prefix = key[0] as char;
        let body = &key[1..];

        match prefix {
            // 1. 블록 데이터 (b + BlockHash)
            'b' => {
                writeln!(writer, "[BLOCK] Hash: 0x{}", hex_encode(body)).unwrap();
                writeln!(writer, "        Data Size: {} bytes", value.len()).unwrap();
            }

            // 2. 블록 인덱스 (i + Height)
            'i' => {
                if body.len() == 8 {
                    let height = u64::from_be_bytes(body.try_into().unwrap());
                    writeln!(writer, "[INDEX] Height: {} -> Block Hash: 0x{}", height, hex_encode(&value)).unwrap();
                }
            }

            // 3. 계정 정보 (a + Address) - 현재 깨져 보이는 부분 해결
            'a' => {
                writeln!(writer, "[ACCOUNT] Address: 0x{}", hex_encode(body)).unwrap();
                // 프로젝트가 postcard를 사용한다면 여기서 역직렬화 시도 가능
                writeln!(writer, "          Encoded State: {}", hex_encode(&value)).unwrap();
            }

            // 4. MPT 노드 (v + NodeHash) - 리포트에서 [UNKNOWN]으로 떴던 것들
            'v' => {
                writeln!(writer, "[MPT_NODE] Node Hash: 0x{}", hex_encode(body)).unwrap();
                writeln!(writer, "           RLP Payload: {}", hex_encode(&value)).unwrap();
            }

            // 5. 트랜잭션 수신 확인 (p + TxHash)
            'p' => {
                writeln!(writer, "[TX_RECEIPT] Tx Hash: 0x{}", hex_encode(body)).unwrap();
            }

            // 6. 시스템 설정 및 기타 (단어 키)
            _ => {
                let key_str = String::from_utf8_lossy(&key);
                if key_str == "last_block" {
                    writeln!(writer, "[SYSTEM] Last Block Hash: 0x{}", hex_encode(&value)).unwrap();
                } else {
                    // 프리픽스가 없는 일반 데이터인 경우
                    writeln!(writer, "[RAW_DATA] Key: {} | Val: {}", key_str, hex_encode(&value)).unwrap();
                }
            }
        }
        writeln!(writer, "{:-^60}", "").unwrap();
    }

    writer.flush().unwrap();
    println!("분석 완료: {}", output_path);
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}