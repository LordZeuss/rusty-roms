use rusqlite::Connection;
use std::path::PathBuf;

use tauri::Window;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::query::db_path;

fn default_download_dir() -> Result<std::path::PathBuf, String> {
    let mut p = dirs::home_dir().ok_or("...")?;
    p.push("Downloads");
    p.push("Roms");
    Ok(p)
}

#[tauri::command]
pub fn clear_download_dir() -> Result<(), String> {
    let conn = Connection::open(db_path()).map_err(|e| format!("Failed to open DB: {}", e))?;
    ensure_settings_table(&conn)?;

    conn.execute("DELETE FROM settings WHERE key = 'download_dir'", [])
        .map_err(|e| format!("Failed to clear download_dir: {}", e))?;

    Ok(())
}


fn ensure_settings_table(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .map_err(|e| format!("Failed creating settings table: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_download_dir() -> Result<String, String> {
    let conn = Connection::open(db_path()).map_err(|e| format!("Failed to open DB: {}", e))?;
    ensure_settings_table(&conn)?;

    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = 'download_dir'")
        .map_err(|e| format!("Failed to prepare: {}", e))?;

    let value: Result<String, _> = stmt.query_row([], |row| row.get(0));

    match value {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Ok(default_download_dir()?.to_string_lossy().to_string()),
    }
}

#[tauri::command]
pub fn set_download_dir(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Path cannot be empty".into());
    }

    let conn = Connection::open(db_path()).map_err(|e| format!("Failed to open DB: {}", e))?;
    ensure_settings_table(&conn)?;

    conn.execute(
        "INSERT INTO settings(key, value) VALUES('download_dir', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [&path],
    )
    .map_err(|e| format!("Failed to save download_dir: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn pick_download_dir(window: Window) -> Result<Option<String>, String> {
    // tauri-plugin-dialog 2.6.0 uses callbacks, so we bridge it to async.
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();

    // Determine the platform and choose method accordingly
    if cfg!(target_os = "android") {
        // On Android, use pick_file as pick_folder is not available
        window
            .dialog()
            .file()
            .pick_folder(move |file: Option<FilePath>| {
                // ignore send error if receiver was dropped
                let _ = tx.send(file);
            });
    } else {
        // On Desktop, use pick_folder
        window
            .dialog()
            .file()
            .pick_folder(move |folder: Option<FilePath>| {
                // ignore send error if receiver was dropped
                let _ = tx.send(folder);
            });
    }

    let picked: Option<FilePath> = rx
        .await
        .map_err(|_| "Folder picker was cancelled or closed".to_string())?;

    Ok(picked.map(|p| p.to_string()))
}

