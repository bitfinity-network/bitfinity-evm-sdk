use std::{fmt::Debug, io::{Write, Read}};

use candid::CandidType;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use did::{Block, H256, codec, Transaction, TransactionReceipt};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;


fn block_serialization(c: &mut Criterion) {
    let blocks: Vec<Block<H256>> = read_all_from_json_files("./benches/resources/json/block");
    assert!(blocks.len() >= 100);
    candid_encode_decode_bench(c, &blocks, "blocks");
    bincode_encode_decode_bench(c, &blocks, "blocks");
}

fn transaction_serialization(c: &mut Criterion) {
    let transactions: Vec<Transaction> = read_all_from_json_files("./benches/resources/json/transaction");
    assert!(transactions.len() >= 100);
    candid_encode_decode_bench(c, &transactions, "transactions");
}

fn receipt_serialization(c: &mut Criterion) {
    let receipts: Vec<TransactionReceipt> = read_all_from_json_files("./benches/resources/json/receipt");
    assert!(receipts.len() >= 100);
    candid_encode_decode_bench(c, &receipts, "receipts");
}

fn candid_encode_decode_bench<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(c: &mut Criterion, input: &[T], name: &str) {
    {
        let compressed_size = use_candid(&input);
        println!("- START --------------------------------------------------------------------");
        println!("Testing {} {} - with candid compressed. Final compressed size: {} bytes", input.len(), name, compressed_size);
        println!("---------------------------------------------------------------------");

        c.bench_function("candid serialization", |b| {
            b.iter(|| {
                let _ = use_candid(black_box(&input));
            })
        });

        println!("- END --------------------------------------------------------------------");
    }

    {
        let compressed_size = use_candid_and_gzip(&input);
        println!("- START --------------------------------------------------------------------");
        println!("Testing {} {} - candid-gzip compressed. Final compressed size: {} bytes", input.len(), name, compressed_size);
        println!("---------------------------------------------------------------------");

        c.bench_function("candid-gzip serialization", |b| {
            b.iter(|| {
                let _ = use_candid_and_gzip(black_box(&input));
            })
        });

        println!("- END --------------------------------------------------------------------");
    }
}

fn bincode_encode_decode_bench<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(c: &mut Criterion, input: &[T], name: &str) {
    {
        let compressed_size = use_bincode(&input);
        println!("- START --------------------------------------------------------------------");
        println!("Testing {} {} - bincode compressed. Final compressed size: {} bytes", input.len(), name, compressed_size);
        println!("---------------------------------------------------------------------");

        c.bench_function("bincode serialization", |b| {
            b.iter(|| {
                let _ = use_bincode(black_box(&input));
            })
        });

        println!("- END --------------------------------------------------------------------");
    }

    {
        let compressed_size = use_bincode_and_gzip(&input);
        println!("- START --------------------------------------------------------------------");
        println!("Testing {} {} - bincode-gzip compressed. Final compressed size: {} bytes", input.len(), name, compressed_size);
        println!("---------------------------------------------------------------------");

        c.bench_function("bincode-gzip serialization", |b| {
            b.iter(|| {
                let _ = use_bincode_and_gzip(black_box(&input));
            })
        });

        println!("- END --------------------------------------------------------------------");
    }
}


fn use_candid<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(input: &[T]) -> usize {
    let mut total_size = 0;
    for data in input {
        let encoded = codec::encode(data);
        total_size += encoded.len();
        let decoded = codec::decode(&encoded);
        assert_eq!(data, &decoded);
    }
    total_size
}

fn use_bincode<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(input: &[T]) -> usize {
    let mut total_size = 0;
    for data in input {
        let encoded = codec::bincode_encode(data);
        total_size += encoded.len();
        let decoded = codec::bincode_decode(&encoded);
        assert_eq!(data, &decoded);
    }
    total_size
}

fn use_candid_and_gzip<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(input: &[T]) -> usize {
    let mut total_size = 0;
    for data in input {
        let encoded = compress_to_gzip(&codec::encode(data));
        total_size += encoded.len();
        let decoded = codec::decode(&decompress_from_gzip(&encoded));
        assert_eq!(data, &decoded);
    }
    total_size
}

fn use_bincode_and_gzip<T: Serialize + DeserializeOwned + CandidType + Debug + PartialEq + Eq>(input: &[T]) -> usize {
    let mut total_size = 0;
    for data in input {
        let encoded = compress_to_gzip(&codec::bincode_encode(data));
        total_size += encoded.len();
        let decoded = codec::bincode_decode(&decompress_from_gzip(&encoded));
        assert_eq!(data, &decoded);
    }
    total_size
}

fn compress_to_gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data.as_ref()).unwrap();
    encoder.finish().unwrap()
}

fn decompress_from_gzip(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    flate2::read::GzDecoder::new(data.as_ref())
    .read_to_end(&mut result).unwrap();
    result
}   

/// Reads all the files in a directory and parses them into serde_json::Value
pub fn read_all_from_json_files<T: DeserializeOwned>(path: &str) -> Vec<T> {
    let mut jsons = vec![];
    for file in std::fs::read_dir(path).unwrap() {
        let file = file.unwrap();
        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(file.path()).unwrap()).unwrap();
        let value = value.get("result").unwrap().to_owned();
        let data: T = serde_json::from_value(value.clone()).unwrap();
        jsons.push(data);
    }
    jsons
}

criterion_group!(benches, block_serialization, transaction_serialization, receipt_serialization);
criterion_main!(benches);
