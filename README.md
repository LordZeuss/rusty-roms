# Rusty Roms

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
* Sony Playstation
* Sony Playstation 2
* Sony Playstation 3
* Sony Playstation Portable
* Sony Playstation Vita
* Microsoft Xbox
* Microsoft Xbox 360
* Commodore 64
* Sega Dreamcast

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

---

## Usage

On first launch, you will need to create the database so it can be cached. All this is done automatically, but you will need to click update library.

1. Click the settings button at the bottom
2. Select 'Update Library'

This will create the database and cache all of the links. The speed this takes typically depends on your network speed, but it shouldn't take more than a few minutes at most.

Once you have updated library, you can search for your games! You don't need to update the library after the intial load unless you want to.
