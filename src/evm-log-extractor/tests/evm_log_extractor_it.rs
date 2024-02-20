use std::path::PathBuf;

use candid::utils::ArgumentEncoder;
use candid::{decode_args, encode_args, CandidType};
use evm_canister_client::client::{Log, Logs};
use evm_canister_client::{CanisterClient, CanisterClientResult, EvmCanisterClient};
use evm_log_extractor::job::logs::{run_logs_job, LogsJobSettings};
use serde::de::DeserializeOwned;
use tokio::fs::{read_dir, File};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone)]
struct MockCanisterClient{
    logs: Vec<String>
}

#[async_trait::async_trait]
impl CanisterClient for MockCanisterClient {
    async fn update<T, R>(&self, _method: &str, _args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send + Sync,
        R: DeserializeOwned + CandidType,
    {
        panic!("should never call update")
    }

    async fn query<T, R>(&self, method: &str, args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send + Sync,
        R: DeserializeOwned + CandidType,
    {
        if method == "ic_logs" {
            let (offset, limit): (usize, usize) = decode_args(&encode_args(args).unwrap()).unwrap();
            
            let logs = self.logs[offset..offset + limit].iter().enumerate().map(|(i, log)| Log {
                offset: i,
                log: log.clone(),
            }).collect::<Vec<_>>();

            let logs = serde_json::to_value(Ok::<Logs, ()>(Logs {
                logs,
                all_logs_count: self.logs.len(),
            })).unwrap();
            Ok(serde_json::from_value(logs).unwrap())
        } else {
            panic!("should never call query with method {}", method);
        }
    }
}

#[tokio::test]
async fn test_extract_logs() {
    // Arrange
    let mock_client = MockCanisterClient;
    let evm_client = EvmCanisterClient::new(mock_client);

    let temp_dir = tempfile::tempdir().unwrap();
    let logs_path = temp_dir.path().join("logs");
    assert!(!logs_path.exists());

    let logs_settings = LogsJobSettings {
        path: logs_path.to_str().unwrap().to_string(),
        max_logs_per_call: 1000,
        start_from_offset: Default::default(),
    };

    // Act 1 - get all logs
    run_logs_job(evm_client.clone(), logs_settings.clone())
        .await
        .unwrap();

    // Assert the logs file is created
    assert!(logs_path.exists());
    let log_file = read_dir(logs_path)
        .await
        .unwrap()
        .next_entry()
        .await
        .unwrap()
        .unwrap();
    assert!(log_file.file_name().to_str().unwrap().ends_with(".log"));

    // Assert there is at least one log in the file (there should be at least the one from `admin_ic_permissions_add`)
    let previous_logs_from_file = file_to_vec(log_file.path()).await;
    assert!(!previous_logs_from_file.is_empty());
    assert!(previous_logs_from_file.len() < logs_settings.max_logs_per_call);

    // Assert the next start offset is updated
    let previous_start_from_offset = { *logs_settings.start_from_offset.lock().await };

    // Act 2 - create a new log and get all logs
    run_logs_job(evm_client.clone(), logs_settings.clone())
        .await
        .unwrap();

    // Assert the new logs are appended to the file
    let logs_from_file = file_to_vec(log_file.path()).await;
    assert!(!logs_from_file.is_empty());
    assert!(logs_from_file.len() > previous_logs_from_file.len());

    // Assert the next start offset is updated
    {
        let start_from_offset = *logs_settings.start_from_offset.lock().await;
        assert!(start_from_offset > previous_start_from_offset);
    }

    // Assert the logs are not duplicated
    let mut unique_logs = logs_from_file.clone();
    unique_logs.dedup();
    assert_eq!(logs_from_file, unique_logs);

}

async fn file_to_vec(file_path: PathBuf) -> Vec<String> {
    let file = File::open(file_path).await.unwrap();
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut logs_from_file = vec![];
    while let Some(line) = lines.next_line().await.unwrap() {
        logs_from_file.push(line);
    }

    logs_from_file
}
