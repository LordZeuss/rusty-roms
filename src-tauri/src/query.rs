use rusqlite::Connection;
use tauri::command;
use std::path::PathBuf;
use dirs;

#[derive(Clone, serde::Serialize)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub console: String,
    pub size: String,
    pub dl_link: String,
    pub is_downloaded: bool,
}

pub fn db_path() -> PathBuf {
    let mut p = dirs::home_dir()
        .expect("Could not determine home directory");

    p.push(".rusty-roms");

    std::fs::create_dir_all(&p)
        .expect("Failed to create .rusty-roms directory");

    p.push("games.db");
    p
}

#[command]
pub fn search_games(search: String) -> Result<Vec<Game>, String> {
    let conn = Connection::open(db_path())
        .map_err(|e| format!("Failed to open DB: {}", e))?;

    // normalize input the same way as SQL: lowercase + remove separators/spaces
    let normalized: String = search
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_' && *c != ':')
        .collect();

    let pattern = format!("%{}%", normalized);

    let mut stmt = conn
        .prepare(
            "SELECT id, name, console, size, dl_link, is_downloaded
             FROM games
             WHERE LOWER(REPLACE(REPLACE(REPLACE(REPLACE(name, ' ', ''), '-', ''), '_', ''), ':', '')) LIKE ?
             LIMIT 200"
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let games_iter = stmt
        .query_map([pattern], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                console: row.get(2)?,
                size: row.get(3)?,
                dl_link: row.get(4)?,
                is_downloaded: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| format!("Query execution failed: {}", e))?;

    let mut results = Vec::new();
    for game in games_iter {
        results.push(game.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(results)
}


