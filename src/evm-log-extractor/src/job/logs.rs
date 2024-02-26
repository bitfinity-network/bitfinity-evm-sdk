use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Datelike, Utc};
use evm_canister_client::client::Log;
use evm_canister_client::{CanisterClient, EvmCanisterClient};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Logs Jobs settings
#[derive(Clone)]
pub struct LogsJobSettings {
    /// Path where to put log files
    pub path: String,
    pub max_logs_per_call: usize,
    pub start_from_offset: Arc<Mutex<usize>>,
}

/// Download the Logs from an evmc instance and saves them to a file.
pub async fn run_logs_job<C: CanisterClient>(
    client: EvmCanisterClient<C>,
    settings: LogsJobSettings,
) -> anyhow::Result<()> {
    let offset = *settings.start_from_offset.lock().await;
    let logs = client.ic_logs(settings.max_logs_per_call, offset).await??;

    let current_date = chrono::Utc::now();
    let filename = filename(&current_date);
    write_logs(&logs.logs, &settings.path, &filename).await?;

    *settings.start_from_offset.lock().await = logs
        .logs
        .last()
        .map(|log| log.offset + 1)
        .unwrap_or_else(|| logs.all_logs_count);

    Ok(())
}

fn filename(date: &DateTime<Utc>) -> String {
    format!(
        "{}_{:02}_{:02}_logs.log",
        date.year(),
        date.month(),
        date.day()
    )
}

async fn write_logs(logs: &[Log], path: &str, filename: &str) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(Path::new(path)).await?;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(path).join(filename))
        .await?;

    for log in logs {
        file.write_all(log.log.trim().as_bytes()).await?;
        file.write_all(b"\n").await?;
    }
    file.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    use super::*;

    #[tokio::test]
    async fn test_job_should_create_file() {
        // Arrange
        let temp_dir = tempdir().unwrap();
        let logs_path = temp_dir.path().join("some").join("else");
        let logs_file = "file.log";
        assert!(!logs_path.exists());

        let logs = vec![
            Log {
                log: "line 0".to_string(),
                offset: 0,
            },
            Log {
                log: "line 1".to_string(),
                offset: 0,
            },
        ];

        // Act
        write_logs(&logs, logs_path.to_str().unwrap(), logs_file)
            .await
            .unwrap();

        // Assert
        let log_file = logs_path.join(logs_file);
        assert!(log_file.exists());

        let file = File::open(log_file).await.unwrap();
        let reader = BufReader::new(file);

        let mut lines = reader.lines();
        assert_eq!(Some("line 0".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(Some("line 1".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(None, lines.next_line().await.unwrap());
    }

    #[tokio::test]
    async fn test_job_should_append_to_file() {
        // Arrange
        let temp_dir = tempdir().unwrap();
        let logs_path = temp_dir.path();
        let logs_file = "file.log";

        let logs = vec![
            Log {
                log: "line 0".to_string(),
                offset: 0,
            },
            Log {
                log: "line 1".to_string(),
                offset: 0,
            },
        ];
        write_logs(&logs, logs_path.to_str().unwrap(), logs_file)
            .await
            .unwrap();

        assert!(logs_path.join(logs_file).exists());

        // Act
        let new_logs = vec![
            Log {
                log: "line 2".to_string(),
                offset: 0,
            },
            Log {
                log: "line 3".to_string(),
                offset: 0,
            },
        ];
        write_logs(&new_logs, logs_path.to_str().unwrap(), logs_file)
            .await
            .unwrap();

        // Assert
        let log_file = logs_path.join(logs_file);
        assert!(log_file.exists());

        let file = File::open(log_file).await.unwrap();
        let reader = BufReader::new(file);

        let mut lines = reader.lines();
        assert_eq!(Some("line 0".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(Some("line 1".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(Some("line 2".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(Some("line 3".to_owned()), lines.next_line().await.unwrap());
        assert_eq!(None, lines.next_line().await.unwrap());
    }

    #[test]
    fn test_filename() {
        // Arrange
        let date_2014_01_01 = NaiveDate::from_ymd_opt(2014, 1, 1)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap()
            .and_utc();
        let date_2021_12_01 = NaiveDate::from_ymd_opt(2021, 12, 1)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap()
            .and_utc();
        let date_2014_03_17 = NaiveDate::from_ymd_opt(2014, 3, 17)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap()
            .and_utc();

        // Act & Assert
        assert_eq!("2014_01_01_logs.log", filename(&date_2014_01_01));
        assert_eq!("2021_12_01_logs.log", filename(&date_2021_12_01));
        assert_eq!("2014_03_17_logs.log", filename(&date_2014_03_17));
    }
}
