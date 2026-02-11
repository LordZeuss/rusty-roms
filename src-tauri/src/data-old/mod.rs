// ------------------------ Imports ------------------------

// Scraper dependencies
use reqwest;
use scraper::{Html, Selector};
use rusqlite::{params, Connection, Result};

// Std dependencies
use std::fs;
use std::path::PathBuf;
use dirs;

// ------------------------ Data Struct ------------------------

pub struct Game {
    pub name: String,
    pub date: String,
    pub size: String,
    pub dl_link: String,
    pub is_downloaded: bool,
}

// ------------------------ DB Helpers ------------------------

pub fn db_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Could not find home directory");
    path.push(".roms-tauri");
    fs::create_dir_all(&path).expect("Failed to create .roms-tauri directory");
    path.push("games.db");
    path
}

pub fn save_to_db(conn: &Connection, game: &Game, console: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO games (name, console, date, size, dl_link, is_downloaded) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![game.name, console, game.date, game.size, game.dl_link, game.is_downloaded],
    )?;
    Ok(())
}

pub fn remove_old_db() -> std::io::Result<()> {
    let mut file_path = db_path();
    if file_path.exists() {
        fs::remove_file(&file_path)?;
        println!("Removed old DB at {:?}", file_path);
    } else {
        println!("No old DB to delete at {:?}...Continuing...", file_path);
    }
    Ok(())
}

// ------------------------ Scraper ------------------------

pub fn scrape() -> Result<(), Box<dyn std::error::Error>> {
    let db_dir = db_path().parent().unwrap().to_path_buf();
    fs::create_dir_all(&db_dir)?;

    remove_old_db()?; 
    setup()?;
    console_fill()?;

    let conn = Connection::open(db_path())?;

    // Ensure tables exist

    let console_rows: Vec<(String, String)> = conn
        .prepare("SELECT console, url FROM consoles ORDER BY id")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    if console_rows.is_empty() {
        println!("No consoles found in the database!");
        return Ok(());
    }

    for (console_name, site_url) in console_rows {
        println!("Scraping console: {} ({})", console_name, site_url);

        let response = reqwest::blocking::get(&site_url)?;
        let html = response.text()?;
        let document = Html::parse_document(&html);

        let game_row_selector = Selector::parse("tr")?;
        let name_selector = Selector::parse(".link a")?;
        let date_selector = Selector::parse("td:nth-child(3)")?;
        let size_selector = Selector::parse("td:nth-child(2)")?;

        for row in document.select(&game_row_selector) {
            let name = row
                .select(&name_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_owned())
                .unwrap_or_else(|| "Unknown".to_owned());

            let partial_link = row
                .select(&name_selector)
                .next()
                .and_then(|e| e.value().attr("href"))
                .map(|url| url.to_owned())
                .unwrap_or_else(|| "Unknown".to_owned());

            let link = format!("{}{}", site_url, partial_link);

            let date = row
                .select(&date_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_owned())
                .unwrap_or_else(|| "Unknown".to_owned());

            let size = row
                .select(&size_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_owned())
                .unwrap_or_else(|| "Unknown".to_owned());

            let is_downloaded: bool = false;

            let game = Game {
                name,
                date,
                size,
                dl_link: link,
                is_downloaded,
            };

            save_to_db(&conn, &game, &console_name)?;
        }

        println!("Finished scraping console: {}", console_name);

        duplicate_cleanup_consoles()?;
        duplicate_cleanup_games()?;
        remove_bad_data()?;
    }

    println!("All consoles scraped successfully!");
    Ok(())
}

// ------------------------ DB Utilities ------------------------

pub fn duplicate_cleanup_games() -> Result<()> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        "
        WITH duplicates AS (
          SELECT MIN(rowid) AS keep_id
          FROM games
          GROUP BY name
        )
        DELETE FROM games
        WHERE rowid NOT IN (SELECT keep_id FROM duplicates)",
        [],
    )?;
    Ok(())
}

pub fn duplicate_cleanup_consoles() -> Result<()> {
    let conn = Connection::open(db_path())?;
    conn.execute(
        "
        WITH duplicates AS (
          SELECT MIN(rowid) AS keep_id
          FROM consoles
          GROUP BY console
        )
        DELETE FROM consoles
        WHERE rowid NOT IN (SELECT keep_id FROM duplicates)",
        [],
    )?;
    Ok(())
}

pub fn remove_bad_data() -> Result<()> {
    let conn = Connection::open(db_path())?;
    let bad_names = ["Unknown", "Parent directory/", "./", "../"];
    for name in bad_names {
        conn.execute("DELETE FROM games WHERE name = ?1", [name])?;
    }
    conn.execute(
        "
        UPDATE games SET name = REPLACE(name, '.zip', '')
        WHERE name LIKE '%.zip'",
        [],
    )?;
    Ok(())
}

// ------------------------ Consoles Helper ------------------------

pub fn insert_consoles(conn: &Connection, consoles: &[(&str, &str)]) -> Result<()> {
    for (console, url) in consoles {
        conn.execute(
            "INSERT INTO consoles (console, url) VALUES (?1, ?2)",
            params![console, url],
        )?;
    }
    Ok(())
}

pub fn console_fill() -> Result<()> {
    let conn = Connection::open(db_path())?;
    let consoles = [
        ("Nintendo New 3DS", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20New%20Nintendo%203DS%20%28Decrypted%29/"),
        ("Nintendo 3DS", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%203DS%20%28Decrypted%29/"),
        ("Nintendo DSi", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%20DSi%20%28Decrypted%29/"),
        ("Nintendo DS", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%20DS%20%28Decrypted%29/"),
        ("Nintendo Game Boy", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy/"),
        ("Nintendo Game Boy Color", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy%20Color/"),
        ("Nintendo Game Boy Advance", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Game%20Boy%20Advance/"),
        ("Nintendo Entertainment System", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%20Entertainment%20System%20%28Headered%29/"),
        ("Nintendo 64", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Nintendo%2064%20%28BigEndian%29/"),
        ("Nintendo GameCube", "https://myrient.erista.me/files/Redump/Nintendo%20-%20GameCube%20-%20NKit%20RVZ%20%5Bzstd-19-128k%5D/"),
        ("Nintendo Wii", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Wii%20%28Digital%29%20%28CDN%29/"),
        ("Nintendo Wii U", "https://myrient.erista.me/files/No-Intro/Nintendo%20-%20Wii%20U%20%28Digital%29%20%28CDN%29/"),
        ("Sony Playstation 3", "https://myrient.erista.me/files/No-Intro/Sony%20-%20PlayStation%203%20%28PSN%29%20%28Content%29/"),
        ("Sony Playstation Portable", "https://myrient.erista.me/files/No-Intro/Sony%20-%20PlayStation%20Portable%20%28PSN%29%20%28Decrypted%29/"),
        ("Sony Playstation Vita", "https://myrient.erista.me/files/No-Intro/Sony%20-%20PlayStation%20Vita%20%28PSN%29%20%28Content%29/"),
        ("Microsoft Xbox 360", "https://myrient.erista.me/files/No-Intro/Microsoft%20-%20Xbox%20360%20%28Digital%29/"),
    ];
    insert_consoles(&conn, &consoles)?;
    println!("Added Consoles");
    Ok(())
}

pub fn setup() -> Result<()> {
    let conn = Connection::open(db_path())?;

    // Create consoles table
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS consoles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            console TEXT NOT NULL,
            url TEXT NOT NULL
        )",
        [],
    )?;

    // Create games table
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            console TEXT NOT NULL,
            date TEXT NOT NULL,
            size TEXT NOT NULL,
            dl_link TEXT NOT NULL,
            is_downloaded BOOLEAN NOT NULL
        )",
        [],
    )?;

    println!("DB created with Games and Consoles Table Created");
    Ok(())
}

