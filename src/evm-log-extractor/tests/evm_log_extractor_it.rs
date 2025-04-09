use std::path::PathBuf;

use candid::utils::ArgumentEncoder;
use candid::{CandidType, decode_args, encode_args};
use evm_canister_client::client::{Log, Logs};
use evm_canister_client::{CanisterClient, CanisterClientResult, EvmCanisterClient};
use evm_log_extractor::job::logs::{LogsJobSettings, run_logs_job};
use serde::de::DeserializeOwned;
use tokio::fs::{File, read_dir};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone)]
struct MockCanisterClient {
    max_logs: usize,
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
            let (limit, offset): (usize, usize) = decode_args(&encode_args(args).unwrap()).unwrap();

            let last_log = self.max_logs.min(offset + limit);
            let first_log = offset.min(last_log);

            let mut logs = vec![];
            for i in first_log..last_log {
                logs.push(Log {
                    log: format!("{i}"),
                    offset: i,
                });
            }

            let logs = serde_json::to_value(Ok::<Logs, ()>(Logs {
                logs,
                all_logs_count: self.max_logs,
            }))
            .unwrap();

            Ok(serde_json::from_value(logs).unwrap())
        } else {
            panic!("should never call query with method {}", method);
        }
    }
}

#[tokio::test]
async fn test_extract_logs() {
    // Arrange
    let max_logs_in_canister = 1500;
    let max_logs_per_call = 1000;
    let mock_client = MockCanisterClient {
        max_logs: max_logs_in_canister,
    };
    let evm_client = EvmCanisterClient::new(mock_client);

    let temp_dir = tempfile::tempdir().unwrap();
    let logs_path = temp_dir.path().join("logs");
    assert!(!logs_path.exists());

    let logs_settings = LogsJobSettings {
        path: logs_path.to_str().unwrap().to_string(),
        max_logs_per_call,
        start_from_offset: Default::default(),
    };

    // Act 1 - get all logs
    let log_file = {
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
        assert_eq!(max_logs_per_call, previous_logs_from_file.len());

        // Assert the next start offset is updated
        {
            let start_from_offset = *logs_settings.start_from_offset.lock().await;
            assert_eq!(max_logs_per_call, start_from_offset);
        }

        log_file
    };

    // Act 2 - create a new log and get all logs
    {
        run_logs_job(evm_client.clone(), logs_settings.clone())
            .await
            .unwrap();

        // Assert the new logs are appended to the file
        let logs_from_file = file_to_vec(log_file.path()).await;
        assert_eq!(max_logs_in_canister, logs_from_file.len());

        // Assert the next start offset is updated
        {
            let start_from_offset = *logs_settings.start_from_offset.lock().await;
            assert_eq!(max_logs_in_canister, start_from_offset);
        }

        // Assert the logs are not duplicated
        let mut unique_logs = logs_from_file.clone();
        unique_logs.dedup();
        assert_eq!(logs_from_file, unique_logs);
    }

    // Act 3 - there are no more logs in the canister
    {
        run_logs_job(evm_client.clone(), logs_settings.clone())
            .await
            .unwrap();

        // Assert the new logs are appended to the file
        let logs_from_file = file_to_vec(log_file.path()).await;
        assert_eq!(max_logs_in_canister, logs_from_file.len());

        // Assert the next start offset is updated
        {
            let start_from_offset = *logs_settings.start_from_offset.lock().await;
            assert_eq!(max_logs_in_canister, start_from_offset);
        }
    }
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
