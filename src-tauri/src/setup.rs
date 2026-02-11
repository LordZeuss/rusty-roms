use std::fs;
use std::path::Path;
use std::path::PathBuf;


pub fn temp_dir() {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let mut dir_path = PathBuf::from(home_dir);
    dir_path.push(".roms-tauri");

    if dir_path.exists() {
        println!("Setup directory already exists at {:?}, skipping...", dir_path);
    } else {
        if let Err(e) = fs::create_dir_all(&dir_path) {
            eprintln!("Failed to create setup directory: {}", e);
        } else {
            println!("Setup directory created at {:?}", dir_path);
        }
    }
}

