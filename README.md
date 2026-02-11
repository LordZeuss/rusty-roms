# Rusty Roms

![Screenshots](assets/video/example.mp4)

## Features

* **Multi-Platform:** Runs on all main desktop operating systems (Windows/Mac/Linux)
* **Chunked Downloads:** Single download broken into 4 chunks for maximum download speeds
* **Multi-Threaded Downloads:** Download as many games as you want, they'll all download at once!
* **Auto Extraction:** Automatically unzips your downloaded roms
* **Blazing Fast Search:** Data is locally cached, allowing a blazing fast search, regardless of network connection
* **Strong Tech Stack:** A Tauri app built with the memory safe Rust language and Svelte for a modern, reactive UI

---

## Supported Platforms

* Nintendo New 3DS
* Nintendo 3DS
* Nintendo DSi
* Nintendo DS
* Nintendo Game Boy
* Nintendo Game Boy Color
* Nintendo Game Boy Advance
* Nintendo Entertainment System
* Nintendo 64
* Nintendo GameCube
* Nintendo Wii
* Nintendo Wii U
* Sony Playstation 3
* Sony Playstation Portable
* Sony Playstation Vita
* Microsoft Xbox 360

**MORE TO COME THIS WEEK!**

---

## Installation
Download the latest release from the releases page or build from source

### Build from Source

Prerequisites:
    * Install tauri cli via cargo/rust: https://tauri.app/
    * Install rust: https://rust-lang.org/install

```
git clone https://github.com/lordzeuss/rusty-roms
cd rusty-roms
cargo tauri build
```
Binary is located at rusty-roms/src-tauri/target/release/bundle
