// This code is a working version of the downloads page. Does not chunk download, does not unzip
// when complete either (added code to new version) and does not sort by console.
use std::fs::{self, File};
use std::io::{Read, Write};

use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{Emitter, Window};
use tokio::task;

use crate::query::db_path;

#[derive(Serialize, Clone, Debug)]
struct DownloadProgressPayload {
    id: u32,
    progress: String,
}

#[derive(Serialize, Clone, Debug)]
struct DownloadCompletePayload {
    id: u32,
}

fn mark_downloaded(id: u32) -> Result<(), String> {
    let conn = Connection::open(db_path())
        .map_err(|e| format!("Failed to open DB: {}", e))?;

    conn.execute(
        "UPDATE games SET is_downloaded = 1 WHERE id = ?1",
        [id as i64],
    )
    .map_err(|e| format!("Failed to update is_downloaded: {}", e))?;

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn download_file(
    url: String,
    file_name: String,
    id: u32,
    window: Window,
) -> Result<String, String> {
    let download_task = task::spawn_blocking(move || -> Result<String, String> {
        // ~/.roms-tauri/downloads
        let mut downloads_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        downloads_dir.push(".roms-tauri");
        downloads_dir.push("downloads");
        fs::create_dir_all(&downloads_dir)
            .map_err(|e| format!("Failed to create folder: {}", e))?;

        // Ensure extension
        let mut final_file_name = file_name.clone();
        if !final_file_name.contains('.') {
            final_file_name.push_str(".zip");
        }
        let file_path = downloads_dir.join(final_file_name);

        println!("Downloading from: {}", url);
        println!("Saving to: {:?}", file_path);

        let mut response = Client::new()
            .get(&url)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let total_size: u64 = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let mut file =
            File::create(&file_path).map_err(|e| format!("File create error: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 8192];

        // If server doesn't provide content length, we can't compute %/
        if total_size == 0 {
            window.emit(
                "download-progress",
                DownloadProgressPayload {
                    id,
                    progress: "Downloading…".to_string(),
                },
            ).map_err(|e| format!("Emit failed: {}", e))?;
        }

        loop {
            let bytes_read = response
                .read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;

            downloaded = downloaded.saturating_add(bytes_read as u64);

            if total_size > 0 {
                let percent = (downloaded as f64 / total_size as f64) * 100.0;

                window.emit(
                    "download-progress",
                    DownloadProgressPayload {
                        id,
                        progress: format!("{:.2}%", percent),
                    },
                ).map_err(|e| format!("Emit failed: {}", e))?;
            }
        }

        // Guarantee UI ends at 100% when possible
        if total_size > 0 {
            window.emit(
                "download-progress",
                DownloadProgressPayload {
                    id,
                    progress: "100.00%".to_string(),
                },
            ).map_err(|e| format!("Emit failed: {}", e))?;
        }

        // Mark as downloaded in DB
        mark_downloaded(id)?;

        // Notify UI to flip this row to "Redownload"
        window.emit(
            "download-complete",
            DownloadCompletePayload { id },
        ).map_err(|e| format!("Emit failed: {}", e))?;

        Ok(format!("Downloaded to {:?}", file_path))
    });

    download_task.await.map_err(|e| e.to_string())?
}

