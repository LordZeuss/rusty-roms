use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf, Component};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}, Mutex};

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
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

fn ensure_settings_table(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .map_err(|e| format!("Failed creating settings table: {}", e))?;
    Ok(())
}

fn default_download_dir() -> Result<PathBuf, String> {
    let mut p = dirs::home_dir().ok_or("...")?;
    p.push(".rusty-roms");
    p.push("downloads");
    Ok(p)
}

/// Resolves the download directory:
/// - if `override_dir` is Some, use that
/// - else read from settings table
/// - else fallback to default_download_dir()
fn resolve_download_dir(override_dir: Option<String>) -> Result<PathBuf, String> {
    if let Some(p) = override_dir {
        if p.trim().is_empty() {
            return Err("downloadDir cannot be empty".into());
        }
        return Ok(PathBuf::from(p));
    }

    let conn = Connection::open(db_path())
        .map_err(|e| format!("Failed to open DB: {}", e))?;
    ensure_settings_table(&conn)?;

    let saved: Result<String, _> = conn.query_row(
        "SELECT value FROM settings WHERE key='download_dir'",
        [],
        |row| row.get(0),
    );

    match saved {
        Ok(v) if !v.trim().is_empty() => Ok(PathBuf::from(v)),
        _ => default_download_dir(),
    }
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

fn emit_progress(window: &Window, id: u32, msg: String) -> Result<(), String> {
    window
        .emit("download-progress", DownloadProgressPayload { id, progress: msg })
        .map_err(|e| format!("Emit failed: {}", e))
}

fn single_stream_download(
    client: &Client,
    window: &Window,
    id: u32,
    url: &str,
    file_path: &Path,
) -> Result<(), String> {
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let total_size: u64 = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut file = File::create(file_path).map_err(|e| format!("File create error: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];

    if total_size == 0 {
        emit_progress(window, id, "Downloading…".to_string())?;
    }

    loop {
        let bytes_read = response.read(&mut buffer).map_err(|e| format!("Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded = downloaded.saturating_add(bytes_read as u64);

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            emit_progress(window, id, format!("{:.2}%", percent))?;
        }
    }

    if total_size > 0 {
        emit_progress(window, id, "100.00%".to_string())?;
    }

    Ok(())
}

fn ranged_parallel_download_4(
    client: &Client,
    window: &Window,
    id: u32,
    url: &str,
    file_path: &Path,
) -> Result<(), String> {
    let head = client.head(url).send().map_err(|e| format!("HEAD failed: {}", e))?;
    if !head.status().is_success() {
        return Err(format!("HEAD HTTP error: {}", head.status()));
    }

    let total_size: u64 = head
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let accept_ranges = head
        .headers()
        .get(ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    if total_size == 0 || !accept_ranges.contains("bytes") {
        return single_stream_download(client, window, id, url, file_path);
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(file_path)
        .map_err(|e| format!("File open error: {}", e))?;

    file.set_len(total_size)
        .map_err(|e| format!("Failed to set file size: {}", e))?;

    let file = Arc::new(Mutex::new(file));
    let downloaded = Arc::new(AtomicU64::new(0));

    let chunks = 4u64;
    let chunk_size = (total_size + chunks - 1) / chunks;
    let mut handles = Vec::new();

    for i in 0..chunks {
        let start = i * chunk_size;
        if start >= total_size {
            continue;
        }
        let end = ((start + chunk_size) - 1).min(total_size - 1);

        let client = client.clone();
        let url = url.to_string();
        let file = Arc::clone(&file);
        let downloaded = Arc::clone(&downloaded);

        let handle = std::thread::spawn(move || -> Result<(), String> {
            let range_value = format!("bytes={}-{}", start, end);

            let mut resp = client
                .get(&url)
                .header(RANGE, range_value)
                .send()
                .map_err(|e| format!("Range request failed: {}", e))?;

            if !(resp.status().as_u16() == 206 || resp.status().is_success()) {
                return Err(format!("Range HTTP error: {}", resp.status()));
            }

            let mut offset = start;
            let mut buffer = [0u8; 32 * 1024];

            loop {
                let n = resp.read(&mut buffer).map_err(|e| format!("Read error: {}", e))?;
                if n == 0 {
                    break;
                }

                {
                    let mut f = file.lock().map_err(|_| "File mutex poisoned".to_string())?;
                    f.seek(SeekFrom::Start(offset))
                        .map_err(|e| format!("Seek error: {}", e))?;
                    f.write_all(&buffer[..n])
                        .map_err(|e| format!("Write error: {}", e))?;
                }

                offset += n as u64;
                downloaded.fetch_add(n as u64, Ordering::Relaxed);
            }

            Ok(())
        });

        handles.push(handle);
    }

    loop {
        let done_bytes = downloaded.load(Ordering::Relaxed);
        let percent = (done_bytes as f64 / total_size as f64) * 100.0;
        emit_progress(window, id, format!("{:.2}%", percent))?;

        if done_bytes >= total_size {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    for h in handles {
        match h.join() {
            Ok(res) => res?,
            Err(_) => return Err("A download thread panicked".to_string()),
        }
    }

    emit_progress(window, id, "100.00%".to_string())?;
    Ok(())
}

// Prevent Zip Slip: ensure zip paths stay inside destination.
fn safe_join(dest_dir: &Path, entry_name: &str) -> Result<PathBuf, String> {
    let entry_path = Path::new(entry_name);
    let mut clean = PathBuf::new();

    for comp in entry_path.components() {
        match comp {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(format!("Unsafe zip entry path: {}", entry_name));
            }
        }
    }

    Ok(dest_dir.join(clean))
}

fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let zip_file = File::open(zip_path)
        .map_err(|e| format!("Failed to open zip for extraction: {}", e))?;

    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("Failed to create extract directory: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed reading zip entry: {}", e))?;

        let outpath = safe_join(dest_dir, file.name())?;

        if file.is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed creating dir {:?}: {}", outpath, e))?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed creating dir {:?}: {}", parent, e))?;
        }

        let mut outfile =
            File::create(&outpath).map_err(|e| format!("Failed creating file {:?}: {}", outpath, e))?;

        std::io::copy(&mut file, &mut outfile)
            .map_err(|e| format!("Failed extracting {:?}: {}", outpath, e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                let _ = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode));
            }
        }
    }

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn download_file(
    url: String,
    file_name: String,
    id: u32,
    download_dir: Option<String>, // <-- pass-through from UI (optional)
    window: Window,
) -> Result<String, String> {
    let download_task = task::spawn_blocking(move || -> Result<String, String> {
        // Resolve downloads dir (override or saved setting or default)
        let downloads_dir = resolve_download_dir(download_dir)?;
        fs::create_dir_all(&downloads_dir)
            .map_err(|e| format!("Failed to create folder: {}", e))?;

        // Force .zip
        let mut final_file_name = file_name.clone();
        if !final_file_name.to_ascii_lowercase().ends_with(".zip") {
            final_file_name.push_str(".zip");
        }

        let zip_path = downloads_dir.join(&final_file_name);

        println!("Downloading from: {}", url);
        println!("Saving zip to: {:?}", zip_path);

        let client = Client::new();

        // Download zip (chunked with fallback)
        ranged_parallel_download_4(&client, &window, id, &url, &zip_path)?;

        // Extract into downloads_dir/<zip-stem>/
        let stem = Path::new(&final_file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted");

        let extract_dir = downloads_dir.join(stem);

        emit_progress(&window, id, "Extracting…".to_string())?;
        extract_zip(&zip_path, &extract_dir)?;
        emit_progress(&window, id, "Extracted".to_string())?;

        // Optional: delete zip after extraction
        // let _ = fs::remove_file(&zip_path);

        // Mark downloaded only after successful extraction
        mark_downloaded(id)?;

        // Notify UI
        window
            .emit("download-complete", DownloadCompletePayload { id })
            .map_err(|e| format!("Emit failed: {}", e))?;

        Ok(format!(
            "Downloaded to {:?} and extracted to {:?}",
            zip_path, extract_dir
        ))
    });

    download_task.await.map_err(|e| e.to_string())?
}

