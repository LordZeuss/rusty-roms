use reqwest::blocking::Client;
use std::time::Duration;

#[tauri::command]
pub fn network_check() -> Result<bool, String> {

    let url = "https://myrient.erista.me/";

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;

    // HEAD first
    if let Ok(resp) = client.head(url).send() {
        return Ok(resp.status().is_success());
    }

    // fallback to GET
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    Ok(resp.status().is_success())
}

