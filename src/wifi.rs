use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::{EspIOError, Write};
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};

use std::sync::mpsc;
use std::time::Duration;

use crate::config::Config;
use crate::mempool::{DisplayItems, OrangeClockConfig, PriceCurrency};
use crate::util::LNBitsConnection;

const AP_SSID: &str = "LightningATM";

const CONFIG_HTML: &str = include_str!("config_portal.html");

/// Start WiFi AP and serve a configuration portal.
/// Blocks until the user submits config, then saves to NVS and restarts.
/// Never returns.
pub fn start_config_portal(config: &mut Config) -> ! {
    log::info!("Starting WiFi AP configuration portal...");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), None).unwrap(),
        sysloop,
    )
    .unwrap();

    let ap_config = AccessPointConfiguration {
        ssid: AP_SSID.try_into().unwrap(),
        auth_method: AuthMethod::None,
        channel: 1,
        ..Default::default()
    };
    wifi.set_configuration(&Configuration::AccessPoint(ap_config))
        .unwrap();
    wifi.start().unwrap();
    wifi.wait_netif_up().unwrap();

    let ip_info = wifi.wifi().ap_netif().get_ip_info().unwrap();

    let mut mdns = EspMdns::take().unwrap();
    mdns.set_hostname("atm").unwrap();
    mdns.add_service(Some("Lightning ATM"), "_http", "_tcp", 80, &[])
        .unwrap();

    log::info!(
        "WiFi AP '{}' started. Connect and visit http://atm.local or http://{}",
        AP_SSID,
        ip_info.ip
    );

    let (tx, rx) = mpsc::sync_channel::<String>(1);

    let mut http_config = HttpConfig::default();
    http_config.stack_size = 10240;
    let mut server = EspHttpServer::new(&http_config).unwrap();

    server
        .fn_handler::<EspIOError, _>("/", Method::Get, |req| {
            let mut resp = req.into_ok_response()?;
            resp.write_all(CONFIG_HTML.as_bytes())?;
            Ok(())
        })
        .unwrap();

    server
        .fn_handler::<EspIOError, _>("/config", Method::Post, move |mut req| {
            let mut buf = [0u8; 2048];
            let mut total = 0;
            loop {
                let n = req.read(&mut buf[total..])?;
                if n == 0 {
                    break;
                }
                total += n;
                if total >= buf.len() {
                    break;
                }
            }
            let body = String::from_utf8_lossy(&buf[..total]).to_string();
            let _ = tx.send(body);

            let mut resp = req.into_ok_response()?;
            resp.write_all(b"<!DOCTYPE html><html><head><meta charset='UTF-8'><meta name='viewport' content='width=device-width,initial-scale=1'></head><body style='background:#1a1a2e;color:#e0e0e0;text-align:center;margin-top:80px;font-family:-apple-system,BlinkMacSystemFont,sans-serif'><h2 style='color:#f7931a'>Configuration Saved!</h2><p>The ATM will restart now...</p></body></html>")?;
            Ok(())
        })
        .unwrap();

    // Block until form is submitted
    let form_body = rx.recv().expect("Config portal channel closed");
    log::info!("Received config form submission");

    // Parse URL-encoded form data and save to NVS
    let device_string = parse_form_value(&form_body, "device_string").unwrap_or_default();
    let display_type = parse_form_value(&form_body, "display_type")
        .unwrap_or_else(|| crate::config::DEFAULT_DISPLAY_TYPE.to_string());
    let board_type = parse_form_value(&form_body, "board_type")
        .unwrap_or_else(|| crate::config::DEFAULT_BOARD_TYPE.to_string());
    let rotation = parse_form_value(&form_body, "rotation")
        .unwrap_or_else(|| crate::config::DEFAULT_ROTATION.to_string());

    match LNBitsConnection::from_device_string(&device_string) {
        Ok(conn) => {
            config
                .persist_lnbits_connection(&conn)
                .expect("Failed to save LNBits connection");
            config
                .persist_display_type(&display_type)
                .expect("Failed to save display type");
            config
                .persist_board_type(&board_type)
                .expect("Failed to save board type");
            config
                .persist_rotation(&rotation)
                .expect("Failed to save rotation");
            log::info!("ATM configuration saved successfully.");
        }
        Err(e) => {
            log::error!("Invalid device string: {}.", e);
        }
    }

    // Parse and save OrangeClock config (independent of LNBits config)
    let oc_config = parse_orangeclock_config(&form_body);
    config
        .persist_orangeclock(&oc_config)
        .expect("Failed to save OrangeClock config");
    log::info!(
        "OrangeClock config saved (enabled={}). Restarting...",
        oc_config.enabled
    );

    std::thread::sleep(Duration::from_secs(2));
    unsafe { esp_idf_svc::sys::esp_restart() };
    #[allow(unreachable_code)]
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Parse a value from URL-encoded form data (key=value&key2=value2)
fn parse_form_value(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    for pair in body.split('&') {
        if let Some(value) = pair.strip_prefix(&prefix) {
            // application/x-www-form-urlencoded encodes spaces as '+',
            // but percent-encoding only handles %XX sequences.
            let plus_decoded = value.replace('+', " ");
            return Some(
                percent_encoding::percent_decode_str(&plus_decoded)
                    .decode_utf8_lossy()
                    .into_owned(),
            );
        }
    }
    None
}

/// Check if a checkbox form field is present (checked)
fn is_checkbox_checked(body: &str, key: &str) -> bool {
    parse_form_value(body, key).is_some()
}

/// Parse OrangeClock configuration from form submission
fn parse_orangeclock_config(body: &str) -> OrangeClockConfig {
    let enabled = is_checkbox_checked(body, "oc_enabled");
    let wifi_ssid = parse_form_value(body, "oc_ssid").unwrap_or_default();
    let wifi_password = parse_form_value(body, "oc_pass").unwrap_or_default();
    let mempool_endpoint = parse_form_value(body, "oc_mempool")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| crate::mempool::DEFAULT_ENDPOINT.to_string());
    let price_currency = parse_form_value(body, "oc_currency")
        .map(|s| PriceCurrency::from_str(&s))
        .unwrap_or(PriceCurrency::USD);

    let display_items = DisplayItems {
        block_height: is_checkbox_checked(body, "oc_item_height"),
        price: is_checkbox_checked(body, "oc_item_price"),
        moscow_time: is_checkbox_checked(body, "oc_item_moscow"),
        fees: is_checkbox_checked(body, "oc_item_fees"),
        halving_countdown: is_checkbox_checked(body, "oc_item_halving"),
        difficulty_adjustment: is_checkbox_checked(body, "oc_item_diff"),
        mempool_size: is_checkbox_checked(body, "oc_item_mempool"),
    };

    OrangeClockConfig {
        enabled,
        wifi_ssid,
        wifi_password,
        mempool_endpoint,
        display_items,
        price_currency,
    }
}

// --- WiFi Station Mode (for OrangeClock) ---

pub struct WifiStation {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl WifiStation {
    /// Create a new WiFi station from the modem peripheral.
    pub fn new(
        modem: Modem<'static>,
        sysloop: EspSystemEventLoop,
    ) -> Result<Self, esp_idf_svc::sys::EspError> {
        let wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), None)?, sysloop)?;
        Ok(Self { wifi })
    }

    /// Attempt to connect to a WiFi network. Returns true on success.
    pub fn connect(&mut self, ssid: &str, password: &str) -> bool {
        let auth = if password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        };

        let client_config = ClientConfiguration {
            ssid: ssid.try_into().unwrap_or_default(),
            password: password.try_into().unwrap_or_default(),
            auth_method: auth,
            ..Default::default()
        };

        if let Err(e) = self
            .wifi
            .set_configuration(&Configuration::Client(client_config))
        {
            log::error!("OrangeClock WiFi: config error: {:?}", e);
            return false;
        }

        if let Err(e) = self.wifi.start() {
            log::error!("OrangeClock WiFi: start error: {:?}", e);
            return false;
        }

        match self.wifi.connect() {
            Ok(()) => {}
            Err(e) => {
                log::warn!("OrangeClock WiFi: connect error: {:?}", e);
                return false;
            }
        }

        match self.wifi.wait_netif_up() {
            Ok(()) => {
                let ip_info = self.wifi.wifi().sta_netif().get_ip_info().unwrap();
                log::info!("OrangeClock WiFi: connected, IP: {}", ip_info.ip);
                true
            }
            Err(e) => {
                log::warn!("OrangeClock WiFi: netif up timeout: {:?}", e);
                false
            }
        }
    }

    /// Check if WiFi is currently connected.
    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }
}
