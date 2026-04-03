# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Offline Bitcoin Lightning ATM running on ESP32. Accepts physical euro coins via a coin acceptor, generates LNURL-withdraw QR codes offline (no internet needed for operation), and displays them on an e-paper screen for users to scan with a Lightning wallet.

The firmware is written in Rust targeting `xtensa-esp32-espidf` using the esp-idf framework.

## Build & Development Commands

```bash
# Build (requires esp toolchain installed via espup)
cargo build

# Build release
cargo build --release

# Flash and monitor (requires ESP32 connected via USB)
cargo run                          # uses espflash via .cargo/config.toml runner
cargo espflash flash --monitor     # alternative

# Unit tests (run on host x86, NOT on ESP32)
cargo test --lib --no-default-features --target x86_64-unknown-linux-gnu

# Lint
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

## Toolchain Setup

```bash
cargo install espup --locked
espup install
cat $HOME/export-esp.sh >> ~/.bashrc
cargo install cargo-generate espflash ldproxy
```

The Rust toolchain is pinned to `channel = "esp"` in `rust-toolchain.toml`. Target is `xtensa-esp32-espidf` with ESP-IDF v5.5.

## Architecture

The firmware is a state machine loop in `main.rs` with a dual-core design:

- **Core 1 (main):** Runs the ATM state machine — `Idle → CountingCoins → WithdrawReady`
- **Core 0 (background):** Runs the OrangeClock mempool data fetcher when enabled, spawned via FreeRTOS `xTaskCreatePinnedToCore`

### Modules

- **`state.rs`** — `AppState` enum: `Idle`, `CountingCoins(u64)`, `WithdrawReady(u64)`
- **`lnurl.rs`** — Offline LNURL-withdraw generation. Implements the LNBits Fossa protocol: AES-256-CBC encryption with EVP_BytesToKey-style key derivation (MD5 rounds), bech32-encoded output. This is the core crypto logic.
- **`display.rs`** — `AtmDisplay` trait with per-resolution implementations for 1.54", 2.13", and 2.7" Waveshare e-paper displays using `epd-waveshare` + `embedded-graphics`. Also renders OrangeClock data screens.
- **`board.rs`** — `BoardType` enum (`Generic`/`Waveshare`) maps to GPIO pin assignments for different ESP32 boards
- **`coins.rs`** — Pulse-counting coin detection via GPIO ISR with debounce. `COIN_MAP` array maps pulse count to cent value. The ISR handler uses `AtomicU32` for lock-free pulse counting.
- **`config.rs`** — NVS (non-volatile storage) persistence for board type, display type, LNBits connection, and OrangeClock settings. NVS keys must be ≤15 chars.
- **`util.rs`** — `LNBitsConnection` parsing from device string, QR code matrix generation
- **`wifi.rs`** — WiFi AP configuration portal (HTML form via `include_str!`) for device setup. Also provides `WifiStation` for OrangeClock's STA-mode WiFi connection.
- **`mempool.rs`** — OrangeClock feature: fetches Bitcoin network data (block height, price, fees, difficulty, mempool stats) from mempool.space API with a fallback endpoint. Background fetcher runs on Core 0 with `Arc<Mutex<Option<MempoolData>>>` shared state.
- **`orangeclock_icons.rs`** — Bitmap icon data for OrangeClock display rendering
- **`lib.rs`** — Re-exports `coins`, `lnurl`, `mempool`, `state`, `util` for host-side unit testing

### Key Design Constraints

- The `esp` feature flag gates all ESP32-specific deps. The `[lib]` target with `--no-default-features` compiles platform-independent modules for x86 testing.
- The `epd-waveshare` crate is pinned to a specific git rev (`1e525bb`) for unreleased display driver support.
- The espflash runner erases the NVS config partition on every flash (`--erase-parts nvs` in `.cargo/config.toml`).
- Config portal is triggered by holding GPIO0 (BOOT button, physically inside the case) during startup.

## Hardware

- ESP32 NodeMCU or Waveshare ESP32 with integrated e-paper driver
- Waveshare e-paper displays (1.54", 2.13", 2.7")
- HX-616 programmable coin acceptor (pulse-based, 12V via step-up converter)
- MOSFET module to enable/disable coin acceptor
- LED push button for user interaction

## Companion Projects

- `telegram_notification_bot/` — Telegram bot (Python) for ATM status notifications. Not part of the ESP32 firmware build.
- `web-flasher/` — Browser-based firmware flasher (Web Serial API). Allows flashing ESP32 from Chrome/Edge without local toolchain.
