use serde::Serialize;
use tauri::{Emitter, Window};
use tokio::task;

use crate::data;

#[derive(Serialize, Clone, Debug)]
struct StartupProgressPayload {
    percent: u8,
    message: String,
}

fn emit_progress(window: &Window, percent: u8, message: impl Into<String>) -> Result<(), String> {
    window
        .emit(
            "startup-progress",
            StartupProgressPayload {
                percent,
                message: message.into(),
            },
        )
        .map_err(|e| format!("Emit failed: {}", e))
}

#[tauri::command]
pub async fn run_startup_tasks(window: Window) -> Result<(), String> {
    let task = task::spawn_blocking(move || -> Result<(), String> {
        emit_progress(&window, 0, "Starting…")?;

        emit_progress(&window, 5, "Removing old DB…")?;
        data::remove_old_db().map_err(|e| format!("remove_old_db failed: {}", e))?;

        emit_progress(&window, 15, "Creating DB tables…")?;
        data::setup().map_err(|e| format!("setup failed: {}", e))?;

        emit_progress(&window, 25, "Populating consoles…")?;
        data::console_fill().map_err(|e| format!("console_fill failed: {}", e))?;

        // Scrape = 30..100 with per-console progress
        emit_progress(&window, 30, "Scraping…")?;
        data::scrape_with_progress(|pct, msg| {
            // pct is already 30..100
            let _ = emit_progress(&window, pct, msg);
        })
        .map_err(|e| format!("scrape failed: {}", e))?;

        emit_progress(&window, 100, "Done!")?;
        Ok(())
    });

    task.await.map_err(|e| e.to_string())?
}

