mod board;
mod coins;
mod config;
mod display;
mod lnurl;
mod mempool;
mod orangeclock_icons;
mod state;
mod util;
mod wifi;

use epd_waveshare::epd1in54_v2::Epd1in54;
use epd_waveshare::epd2in13_v2::Epd2in13;
use epd_waveshare::epd2in7::Epd2in7;
use epd_waveshare::epd2in7_v2::Epd2in7 as Epd2in7V2;
use epd_waveshare::epd2in7b::Epd2in7b;
use epd_waveshare::prelude::*;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::spi::config::MODE_0;
use esp_idf_hal::{
    delay::Delay,
    gpio::{PinDriver, Pull},
    peripherals::Peripherals,
    spi::*,
    units::FromValueType,
};
use esp_idf_svc::eventloop::EspSystemEventLoop;

use std::thread;
use std::time::{Duration, Instant};

use crate::board::{BoardPins, BoardType};
use crate::display::AtmDisplay;
use crate::state::AppState;

const IDLE_REFRESH_SECS: u64 = 43200; // 12 hours
const COIN_TIMEOUT_SECS: u64 = 360; // 6 minutes
const QR_DISPLAY_TIMEOUT_SECS: u64 = 600; // 10 minutes
const CLEAN_PRESS_THRESHOLD: u32 = 3;
const BOOT_BUTTON_GPIO: u8 = 0;
const PRESS_WINDOW_SECS: u64 = 4;
const PRESS_DEBOUNCE_MS: u64 = 500;
const PRESS_POLL_MS: u64 = 50;
const ATM_SCREEN_TIMEOUT_SECS: u64 = 60; // OrangeClock: show ATM home for 60s after button press

fn format_amount(cents: u64, currency: &str) -> String {
    format!("{:.2} {}", cents as f64 / 100.0, currency)
}

/// Count rapid button presses within a time window (for screen-clean trigger).
fn count_rapid_presses(
    button_pin: &PinDriver<'_, esp_idf_hal::gpio::Input>,
) -> u32 {
    let mut press_count = 1u32;
    let press_start = Instant::now();
    thread::sleep(Duration::from_millis(PRESS_DEBOUNCE_MS));
    while press_start.elapsed() < Duration::from_secs(PRESS_WINDOW_SECS) {
        if button_pin.is_low() {
            press_count += 1;
            thread::sleep(Duration::from_millis(PRESS_DEBOUNCE_MS));
        }
        thread::sleep(Duration::from_millis(PRESS_POLL_MS));
    }
    press_count
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::set_max_level(log::LevelFilter::Debug);

    let mut config = config::Config::open().expect("failed to get non-volatile storage");

    // Check if BOOT button (GPIO0) is held during startup — operator-only config portal trigger.
    // GPIO0 is physically inside the enclosed case, not accessible to ATM users.
    {
        let boot_pin =
            PinDriver::input(unsafe { AnyIOPin::steal(BOOT_BUTTON_GPIO) }, Pull::Up).unwrap();
        thread::sleep(Duration::from_millis(100));
        if boot_pin.is_low() {
            thread::sleep(Duration::from_millis(500));
            if boot_pin.is_low() {
                log::info!("BOOT button held during startup — entering config portal");
                wifi::start_config_portal(&mut config);
            }
        }
        // boot_pin dropped here, releasing GPIO0 before Peripherals::take()
    }

    // Get LNBits connection details from NVS
    let lnbits_connection = match config.get_lnbits_connection() {
        Ok(Some(conn)) => conn,
        Ok(None) => {
            log::warn!("No LNBits configuration found. Starting config portal...");
            wifi::start_config_portal(&mut config);
        }
        Err(e) => {
            log::error!("Failed to read LNBits config: {:?}", e);
            panic!("Config read error");
        }
    };

    // Load OrangeClock config (optional)
    let oc_config = config.get_orangeclock().ok().flatten();
    if let Some(ref oc) = oc_config {
        log::info!(
            "OrangeClock enabled: endpoint={}, currency={:?}",
            oc.mempool_endpoint,
            oc.price_currency
        );
    }

    let peripherals = Peripherals::take().unwrap();

    // Extract modem for OrangeClock WiFi STA before using other peripherals.
    // BoardPins::new() uses unsafe steal() and doesn't need the modem.
    let modem = peripherals.modem;
    let spi2 = peripherals.spi2;

    // Determine board type from config or default
    let board_type_str = config
        .get_board_type()
        .unwrap_or_else(|_| config::DEFAULT_BOARD_TYPE.to_string());
    let board_type = BoardType::from_str(&board_type_str);
    log::info!("Selected Board: {:?}", board_type);

    let pins = BoardPins::new(board_type).expect("Failed to initialize board pins");

    // --- GPIO Setup ---
    let coin_pin = PinDriver::input(pins.coin, Pull::Up).unwrap();

    let mut mosfet_pin = PinDriver::output(pins.mosfet).unwrap();
    mosfet_pin.set_low().unwrap(); // Start enabled (LOW = Accept)

    let button_pin = PinDriver::input(pins.button, Pull::Up).unwrap();

    let mut button_led = PinDriver::output(pins.button_led).unwrap();
    button_led.set_high().unwrap(); // LED ON

    // --- Display Setup ---
    // E-paper specific pins
    let dc = PinDriver::output(pins.dc).unwrap();
    let rst = PinDriver::output(pins.rst).unwrap();
    let busy = PinDriver::input(pins.busy, Pull::Floating).unwrap();

    let spi_config = SpiConfig::new()
        .baudrate(4_u32.MHz().into())
        .data_mode(MODE_0);

    let mut spi = SpiDeviceDriver::new_single(
        spi2,
        pins.sclk,
        pins.mosi,
        None::<AnyIOPin<'_>>,
        Some(pins.cs),
        &SpiDriverConfig::new(),
        &spi_config,
    )
    .unwrap();

    let mut delay = Delay::new_default();

    let display_type = config
        .get_display_type()
        .unwrap_or_else(|_| config::DEFAULT_DISPLAY_TYPE.to_string());
    let rotation_str = config
        .get_rotation()
        .unwrap_or_else(|_| config::DEFAULT_ROTATION.to_string());
    let rotation = display::parse_rotation(&rotation_str);
    log::info!(
        "Selected Display: {}, Rotation: {}°",
        display_type,
        rotation_str
    );

    log::debug!("Initializing e-paper display...");
    let mut display: Box<dyn AtmDisplay<SpiDeviceDriver<'static, SpiDriver>, Delay>> =
        match display_type.as_str() {
            "GxEPD2_270" => {
                let epd = Epd2in7::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
                Box::new(display::Display2in7BwWrapper { epd, rotation })
            }
            "GxEPD2_270_V2" => {
                let epd = Epd2in7V2::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
                Box::new(display::Display2in7V2Wrapper { epd, rotation })
            }
            "GxEPD2_270_3C" => {
                let epd = Epd2in7b::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
                Box::new(display::Display2in7Wrapper { epd, rotation })
            }
            "GxEPD2_213_B74" => {
                let epd = Epd2in13::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
                Box::new(display::Display2in13Wrapper { epd, rotation })
            }
            _ => {
                // Default to 1.54
                let epd = Epd1in54::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
                Box::new(display::Display1in54Wrapper { epd, rotation })
            }
        };
    log::debug!("E-paper display initialized successfully");

    // --- OrangeClock: spawn background fetcher on Core 0 ---
    let shared_mempool_data = if let Some(ref oc) = oc_config {
        let sysloop = EspSystemEventLoop::take().unwrap();
        match wifi::WifiStation::new(modem, sysloop) {
            Ok(mut wifi_station) => {
                log::info!("OrangeClock: connecting to WiFi '{}'...", oc.wifi_ssid);
                wifi_station.connect(&oc.wifi_ssid, &oc.wifi_password);
                Some(mempool::spawn_fetcher(wifi_station, oc.clone()))
            }
            Err(e) => {
                log::error!("OrangeClock: WiFi init failed: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    log::debug!("Initializing coin detector...");
    let mut coin_detector = coins::CoinDetector::new(coin_pin, mosfet_pin);
    log::debug!("Coin detector initialized");
    let mut app_state = AppState::Idle;

    // Track accumulated amount
    let mut current_amount_cents: u64 = 0;

    loop {
        match app_state {
            AppState::Idle => {
                log::info!("State: Idle");
                button_led.set_high().unwrap();

                if let (Some(ref shared_data), Some(ref oc)) = (&shared_mempool_data, &oc_config) {
                    // === OrangeClock idle loop ===
                    // Show initial screen: OrangeClock data if available, else home screen
                    let mut last_data: Option<mempool::MempoolData> = None;
                    if let Ok(guard) = shared_data.lock() {
                        last_data = guard.clone();
                    }

                    if let Some(ref data) = last_data {
                        if let Err(e) = display.show_orangeclock(
                            &mut spi,
                            &mut delay,
                            data,
                            &oc.display_items,
                            &oc.price_currency,
                        ) {
                            log::error!("Display orangeclock error: {}", e);
                        }
                    } else {
                        if let Err(e) = display.home_screen(&mut spi, &mut delay) {
                            log::error!("Display home_screen error: {}", e);
                        }
                    }

                    let mut atm_screen_until: Option<Instant> = None;
                    let mut needs_redraw = false;
                    let mut last_clean = Instant::now();

                    loop {
                        if let Some(cents) = coin_detector.check_for_coin() {
                            current_amount_cents = cents;
                            app_state = AppState::CountingCoins(current_amount_cents);
                            break;
                        }

                        if button_pin.is_low() {
                            log::info!("Button pressed in OrangeClock Idle");
                            let press_count = count_rapid_presses(&button_pin);
                            if press_count >= CLEAN_PRESS_THRESHOLD {
                                log::info!("Screen clean triggered ({} presses)", press_count);
                                button_led.set_low().unwrap();
                                let _ = display.clean(&mut spi, &mut delay);
                                thread::sleep(Duration::from_secs(30));
                                button_led.set_high().unwrap();
                                last_clean = Instant::now();
                            }
                            let _ = display.home_screen(&mut spi, &mut delay);
                            atm_screen_until =
                                Some(Instant::now() + Duration::from_secs(ATM_SCREEN_TIMEOUT_SECS));
                            needs_redraw = false;
                            continue;
                        }

                        // Periodic e-paper full refresh to prevent ghosting
                        if last_clean.elapsed().as_secs() > IDLE_REFRESH_SECS {
                            log::info!("OrangeClock: 12h e-paper refresh");
                            let _ = display.clean(&mut spi, &mut delay);
                            thread::sleep(Duration::from_secs(10));
                            last_clean = Instant::now();
                            needs_redraw = true;
                        }

                        if let Some(until) = atm_screen_until {
                            if Instant::now() >= until {
                                atm_screen_until = None;
                                needs_redraw = true;
                            }
                        }

                        if atm_screen_until.is_none() {
                            if let Ok(guard) = shared_data.try_lock() {
                                if *guard != last_data {
                                    last_data = guard.clone();
                                    needs_redraw = true;
                                }
                            }
                        }

                        if needs_redraw && atm_screen_until.is_none() {
                            if let Some(ref data) = last_data {
                                if let Err(e) = display.show_orangeclock(
                                    &mut spi,
                                    &mut delay,
                                    data,
                                    &oc.display_items,
                                    &oc.price_currency,
                                ) {
                                    log::error!("Display orangeclock error: {}", e);
                                }
                            } else {
                                let _ = display.home_screen(&mut spi, &mut delay);
                            }
                            needs_redraw = false;
                        }

                        thread::sleep(Duration::from_millis(50));
                    }
                } else {
                    // === Original idle loop (no OrangeClock) ===
                    if let Err(e) = display.home_screen(&mut spi, &mut delay) {
                        log::error!("Display home_screen error: {}", e);
                    }

                    let idle_start = Instant::now();
                    loop {
                        if idle_start.elapsed().as_secs() > IDLE_REFRESH_SECS {
                            log::info!("12h idle refresh");
                            let _ = display.clean(&mut spi, &mut delay);
                            thread::sleep(Duration::from_secs(10));
                            break; // Re-enter Idle which redraws home_screen
                        }

                        match coin_detector.wait_for_event(|| button_pin.is_low()) {
                            coins::CoinInteraction::Button => {
                                log::info!("Button pressed in Idle");
                                let press_count = count_rapid_presses(&button_pin);
                                if press_count >= CLEAN_PRESS_THRESHOLD {
                                    log::info!("Screen clean triggered ({} presses)", press_count);
                                    button_led.set_low().unwrap();
                                    let _ = display.clean(&mut spi, &mut delay);
                                    thread::sleep(Duration::from_secs(30));
                                    let _ = display.home_screen(&mut spi, &mut delay);
                                }
                                break;
                            }
                            coins::CoinInteraction::Coin(val) => {
                                current_amount_cents = val;
                                app_state = AppState::CountingCoins(current_amount_cents);
                                break;
                            }
                        }
                    }
                }
            }
            AppState::CountingCoins(amount) => {
                log::info!("State: CountingCoins - {}", amount);
                button_led.set_low().unwrap();

                if let Err(e) = display.show_inserted_amount(
                    &mut spi,
                    &mut delay,
                    &format_amount(amount, &lnbits_connection.currency),
                ) {
                    log::error!("Display amount error: {}", e);
                }

                coin_detector.set_accepting(true);
                button_led.set_high().unwrap(); // LED ON — ready for next coin

                let mut last_coin_time = Instant::now();
                loop {
                    if last_coin_time.elapsed().as_secs() > COIN_TIMEOUT_SECS {
                        log::info!("Coin timeout, auto-proceeding to withdrawal");
                        app_state = AppState::WithdrawReady(current_amount_cents);
                        break;
                    }

                    match coin_detector.wait_for_event(|| button_pin.is_low()) {
                        coins::CoinInteraction::Button => {
                            log::info!("Button pressed - Finishing deposit");
                            app_state = AppState::WithdrawReady(current_amount_cents);
                            break;
                        }
                        coins::CoinInteraction::Coin(val) => {
                            current_amount_cents += val;
                            last_coin_time = Instant::now();
                            log::info!("Added {}, Total: {}", val, current_amount_cents);
                            if let Err(e) = display.show_inserted_amount(
                                &mut spi,
                                &mut delay,
                                &format_amount(current_amount_cents, &lnbits_connection.currency),
                            ) {
                                log::error!("Display amount error: {}", e);
                            }
                            button_led.set_high().unwrap(); // LED ON — ready for next coin
                        }
                    }
                }
            }
            AppState::WithdrawReady(amount) => {
                log::info!("State: WithdrawReady - {}", amount);
                let lnurl_str = lnurl::make_lnurl(
                    &lnbits_connection.base_url,
                    &lnbits_connection.atm_secret,
                    amount,
                );

                if let Err(e) = display.show_qr(&mut spi, &mut delay, &lnurl_str) {
                    log::error!("Display QR error: {}", e);
                }

                let start_time = Instant::now();
                let mut led_on = false;
                loop {
                    if button_pin.is_low() {
                        log::info!("Button pressed - Resetting");
                        current_amount_cents = 0;
                        app_state = AppState::Idle;
                        break;
                    }

                    let elapsed = start_time.elapsed();
                    if elapsed.as_secs() > QR_DISPLAY_TIMEOUT_SECS {
                        log::info!("QR display timeout (10 min)");
                        current_amount_cents = 0;
                        app_state = AppState::Idle;
                        break;
                    }

                    let should_be_on = elapsed.as_millis() % 1000 < 500;
                    if should_be_on != led_on {
                        led_on = should_be_on;
                        if led_on {
                            button_led.set_high().unwrap();
                        } else {
                            button_led.set_low().unwrap();
                        }
                    }

                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}
