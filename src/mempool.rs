//! OrangeClock: Bitcoin network data display during ATM idle time.
//! Fetches data from mempool.space API and displays on e-paper.

use serde::Deserialize;

// --- Configuration types ---

#[derive(Clone)]
pub struct OrangeClockConfig {
    pub enabled: bool,
    pub wifi_ssid: String,
    pub wifi_password: String,
    pub mempool_endpoint: String,
    pub display_items: DisplayItems,
    pub price_currency: PriceCurrency,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PriceCurrency {
    USD,
    EUR,
}

impl PriceCurrency {
    pub fn from_str(s: &str) -> Self {
        match s {
            "EUR" => PriceCurrency::EUR,
            _ => PriceCurrency::USD,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PriceCurrency::USD => "USD",
            PriceCurrency::EUR => "EUR",
        }
    }

    pub fn price_from(&self, data: &MempoolData) -> Option<u64> {
        match self {
            PriceCurrency::USD => data.price_usd,
            PriceCurrency::EUR => data.price_eur,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DisplayItems {
    pub block_height: bool,
    pub price: bool,
    pub moscow_time: bool,
    pub fees: bool,
    pub halving_countdown: bool,
    pub difficulty_adjustment: bool,
    pub mempool_size: bool,
}

impl DisplayItems {
    /// Serialize to comma-separated string for NVS storage
    pub fn to_items_string(&self) -> String {
        let mut items = Vec::new();
        if self.block_height {
            items.push("height");
        }
        if self.price {
            items.push("price");
        }
        if self.moscow_time {
            items.push("moscow");
        }
        if self.fees {
            items.push("fees");
        }
        if self.halving_countdown {
            items.push("halving");
        }
        if self.difficulty_adjustment {
            items.push("difficulty");
        }
        if self.mempool_size {
            items.push("mempool");
        }
        items.join(",")
    }

    /// Deserialize from comma-separated string (exact token match)
    pub fn from_items_string(s: &str) -> Self {
        let tokens: Vec<&str> = s.split(',').map(|t| t.trim()).collect();
        DisplayItems {
            block_height: tokens.contains(&"height"),
            price: tokens.contains(&"price"),
            moscow_time: tokens.contains(&"moscow"),
            fees: tokens.contains(&"fees"),
            halving_countdown: tokens.contains(&"halving"),
            difficulty_adjustment: tokens.contains(&"difficulty"),
            mempool_size: tokens.contains(&"mempool"),
        }
    }

    pub fn default_items() -> Self {
        DisplayItems {
            block_height: true,
            price: true,
            moscow_time: false,
            fees: true,
            halving_countdown: false,
            difficulty_adjustment: false,
            mempool_size: false,
        }
    }

    pub fn needs_block_height(&self) -> bool {
        self.block_height || self.halving_countdown
    }

    pub fn needs_prices(&self) -> bool {
        self.price || self.moscow_time
    }
}

// --- Fetched data ---

#[derive(Clone, Default, Debug, PartialEq)]
pub struct MempoolData {
    pub block_height: Option<u64>,
    pub price_usd: Option<u64>,
    pub price_eur: Option<u64>,
    pub fee_fastest: Option<u32>,
    pub fee_half_hour: Option<u32>,
    pub fee_hour: Option<u32>,
    pub difficulty_progress: Option<f32>,
    pub difficulty_change: Option<f32>,
    pub mempool_count: Option<u64>,
}

// --- Serde models for API responses ---

#[derive(Deserialize, Debug)]
pub struct PricesResponse {
    #[serde(rename = "USD")]
    pub usd: u64,
    #[serde(rename = "EUR")]
    pub eur: u64,
}

#[derive(Deserialize, Debug)]
pub struct FeesResponse {
    #[serde(rename = "fastestFee")]
    pub fastest_fee: u32,
    #[serde(rename = "halfHourFee")]
    pub half_hour_fee: u32,
    #[serde(rename = "hourFee")]
    pub hour_fee: u32,
}

#[derive(Deserialize, Debug)]
pub struct DifficultyResponse {
    #[serde(rename = "progressPercent")]
    pub progress_percent: f32,
    #[serde(rename = "difficultyChange")]
    pub difficulty_change: f32,
}

#[derive(Deserialize, Debug)]
pub struct MempoolStatsResponse {
    pub count: u64,
}

// --- Pure functions ---

/// Calculate blocks remaining until the next Bitcoin halving.
/// Halvings occur every 210,000 blocks.
pub fn blocks_until_halving(height: u64) -> u64 {
    let next_halving = ((height / 210_000) + 1) * 210_000;
    next_halving - height
}

/// Calculate Moscow Time: satoshis per 1 unit of fiat currency.
/// Moscow Time = 100,000,000 / fiat price
pub fn moscow_time(fiat_price: u64) -> u64 {
    if fiat_price == 0 {
        return 0;
    }
    100_000_000 / fiat_price
}

/// Default mempool.space fallback endpoint
pub const FALLBACK_ENDPOINT: &str = "mempool.emzy.de";
pub const DEFAULT_ENDPOINT: &str = "mempool.space";

// --- ESP-only: HTTP fetcher and background task ---

#[cfg(feature = "esp")]
mod fetcher {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::wifi::WifiStation;
    use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};

    /// Data refresh interval (5 minutes)
    const FETCH_INTERVAL_SECS: u64 = 300;
    /// Shorter retry interval when WiFi is disconnected
    const WIFI_RETRY_SECS: u64 = 30;
    /// Max response body size to prevent heap exhaustion
    const MAX_RESPONSE_BYTES: usize = 4096;

    /// Shared state between background fetcher (Core 0) and main loop (Core 1)
    pub type SharedMempoolData = Arc<Mutex<Option<MempoolData>>>;

    /// Context passed to the FreeRTOS task
    struct FetcherContext {
        wifi: WifiStation,
        config: OrangeClockConfig,
        shared: SharedMempoolData,
    }

    /// Spawn the OrangeClock background fetcher on Core 0.
    /// Returns the shared data handle for the main loop to read.
    pub fn spawn_fetcher(wifi: WifiStation, config: OrangeClockConfig) -> SharedMempoolData {
        let shared: SharedMempoolData = Arc::new(Mutex::new(None));

        let ctx = Box::new(FetcherContext {
            wifi,
            config,
            shared: shared.clone(),
        });

        let ctx_ptr = Box::into_raw(ctx) as *mut core::ffi::c_void;

        unsafe {
            let mut handle: esp_idf_svc::sys::TaskHandle_t = core::ptr::null_mut();
            let ret = esp_idf_svc::sys::xTaskCreatePinnedToCore(
                Some(fetcher_task_entry),
                b"oc_fetch\0".as_ptr(),
                16384, // 16KB: TLS handshake + HTTP + serde needs headroom
                ctx_ptr,
                5,
                &mut handle,
                0, // Core 0
            );
            if ret != 1 {
                // pdPASS = 1
                log::error!("Failed to create OrangeClock fetcher task");
                // Reclaim the leaked context
                let _ = Box::from_raw(ctx_ptr as *mut FetcherContext);
            }
        }

        shared
    }

    /// FreeRTOS task entry point for the background fetcher
    unsafe extern "C" fn fetcher_task_entry(param: *mut core::ffi::c_void) {
        let mut ctx = Box::from_raw(param as *mut FetcherContext);
        fetcher_loop(&mut ctx);
        // Task should never return, but clean up if it does
        esp_idf_svc::sys::vTaskDelete(core::ptr::null_mut());
    }

    fn fetcher_loop(ctx: &mut FetcherContext) {
        loop {
            if !ctx.wifi.is_connected() {
                log::info!("OrangeClock: WiFi disconnected, reconnecting...");
                ctx.wifi
                    .connect(&ctx.config.wifi_ssid, &ctx.config.wifi_password);
            }

            if ctx.wifi.is_connected() {
                let data = fetch_all_data(&ctx.config);
                if let Ok(mut guard) = ctx.shared.lock() {
                    if data != MempoolData::default() {
                        *guard = Some(data);
                    } else {
                        log::warn!(
                            "OrangeClock: all API requests failed, not updating display data"
                        );
                    }
                }
                std::thread::sleep(Duration::from_secs(FETCH_INTERVAL_SECS));
            } else {
                log::warn!("OrangeClock: WiFi not available, clearing data");
                if let Ok(mut guard) = ctx.shared.lock() {
                    *guard = None;
                }
                std::thread::sleep(Duration::from_secs(WIFI_RETRY_SECS));
            }
        }
    }

    /// Fetch all enabled data from mempool API with fallback endpoint
    fn fetch_all_data(config: &OrangeClockConfig) -> MempoolData {
        let primary = &config.mempool_endpoint;
        let fallback = FALLBACK_ENDPOINT;

        let mut data = MempoolData::default();

        // Block height (needed for height display and halving countdown)
        if config.display_items.needs_block_height() {
            data.block_height = fetch_block_height(primary).or_else(|| {
                log::warn!(
                    "OrangeClock: primary endpoint failed for block height, trying fallback"
                );
                fetch_block_height(fallback)
            });
        }

        // Prices (needed for price display and moscow time)
        if config.display_items.needs_prices() {
            let prices = fetch_prices(primary).or_else(|| {
                log::warn!("OrangeClock: primary endpoint failed for prices, trying fallback");
                fetch_prices(fallback)
            });
            if let Some(p) = prices {
                data.price_usd = Some(p.usd);
                data.price_eur = Some(p.eur);
            }
        }

        // Fees
        if config.display_items.fees {
            let fees = fetch_fees(primary).or_else(|| {
                log::warn!("OrangeClock: primary endpoint failed for fees, trying fallback");
                fetch_fees(fallback)
            });
            if let Some(f) = fees {
                data.fee_fastest = Some(f.fastest_fee);
                data.fee_half_hour = Some(f.half_hour_fee);
                data.fee_hour = Some(f.hour_fee);
            }
        }

        // Difficulty adjustment
        if config.display_items.difficulty_adjustment {
            let diff = fetch_difficulty(primary).or_else(|| {
                log::warn!("OrangeClock: primary endpoint failed for difficulty, trying fallback");
                fetch_difficulty(fallback)
            });
            if let Some(d) = diff {
                data.difficulty_progress = Some(d.progress_percent);
                data.difficulty_change = Some(d.difficulty_change);
            }
        }

        // Mempool stats
        if config.display_items.mempool_size {
            let stats = fetch_mempool_stats(primary).or_else(|| {
                log::warn!(
                    "OrangeClock: primary endpoint failed for mempool stats, trying fallback"
                );
                fetch_mempool_stats(fallback)
            });
            if let Some(s) = stats {
                data.mempool_count = Some(s.count);
            }
        }

        log::info!("OrangeClock: fetched data: {:?}", data);
        data
    }

    /// Perform an HTTPS GET request and return the response body as a String
    fn http_get(url: &str) -> Option<String> {
        let config = HttpConfig {
            timeout: Some(Duration::from_secs(10)),
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        };

        let mut connection = match EspHttpConnection::new(&config) {
            Ok(c) => c,
            Err(e) => {
                log::error!("OrangeClock: HTTP connection error: {:?}", e);
                return None;
            }
        };

        if let Err(e) =
            connection.initiate_request(esp_idf_svc::http::Method::Get, url, &[])
        {
            log::error!("OrangeClock: HTTP request error for {}: {:?}", url, e);
            return None;
        }

        if let Err(e) = connection.initiate_response() {
            log::error!("OrangeClock: HTTP submit error for {}: {:?}", url, e);
            return None;
        }

        let status = connection.status();
        if status != 200 {
            log::warn!("OrangeClock: HTTP {} for {}", status, url);
            return None;
        }

        let mut buf = [0u8; 1024];
        let mut body = String::with_capacity(512);
        loop {
            match connection.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if body.len() + n > MAX_RESPONSE_BYTES {
                        log::warn!("OrangeClock: response too large for {}, truncating", url);
                        break;
                    }
                    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                        body.push_str(s);
                    }
                }
                Err(e) => {
                    log::error!("OrangeClock: HTTP read error: {:?}", e);
                    break;
                }
            }
        }

        Some(body)
    }

    fn fetch_json<T: serde::de::DeserializeOwned>(endpoint: &str, path: &str) -> Option<T> {
        let url = format!("https://{}{}", endpoint, path);
        let body = http_get(&url)?;
        serde_json::from_str(&body).ok()
    }

    fn fetch_block_height(endpoint: &str) -> Option<u64> {
        let url = format!("https://{}/api/blocks/tip/height", endpoint);
        let body = http_get(&url)?;
        body.trim().parse::<u64>().ok()
    }

    fn fetch_prices(endpoint: &str) -> Option<PricesResponse> {
        fetch_json(endpoint, "/api/v1/prices")
    }

    fn fetch_fees(endpoint: &str) -> Option<FeesResponse> {
        fetch_json(endpoint, "/api/v1/fees/recommended")
    }

    fn fetch_difficulty(endpoint: &str) -> Option<DifficultyResponse> {
        fetch_json(endpoint, "/api/v1/difficulty-adjustment")
    }

    fn fetch_mempool_stats(endpoint: &str) -> Option<MempoolStatsResponse> {
        fetch_json(endpoint, "/api/mempool")
    }
}

#[cfg(feature = "esp")]
pub use fetcher::spawn_fetcher;

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_until_halving() {
        // First halving at 210,000
        assert_eq!(blocks_until_halving(0), 210_000);
        assert_eq!(blocks_until_halving(1), 209_999);
        assert_eq!(blocks_until_halving(209_999), 1);

        // Second halving at 420,000
        assert_eq!(blocks_until_halving(210_000), 210_000);
        assert_eq!(blocks_until_halving(210_001), 209_999);

        // Near current height (~890,000, 5th epoch)
        assert_eq!(blocks_until_halving(890_000), 160_000);
        assert_eq!(blocks_until_halving(840_000), 210_000);
        assert_eq!(blocks_until_halving(840_001), 209_999);
    }

    #[test]
    fn test_moscow_time() {
        assert_eq!(moscow_time(100_000), 1_000); // $100k → 1000 sats/$
        assert_eq!(moscow_time(50_000), 2_000); // $50k → 2000 sats/$
        assert_eq!(moscow_time(1), 100_000_000); // $1 → 1 BTC worth of sats
        assert_eq!(moscow_time(0), 0); // edge case: zero price
    }

    #[test]
    fn test_display_items_roundtrip() {
        let items = DisplayItems {
            block_height: true,
            price: true,
            moscow_time: false,
            fees: true,
            halving_countdown: false,
            difficulty_adjustment: true,
            mempool_size: false,
        };

        let s = items.to_items_string();
        assert!(s.contains("height"));
        assert!(s.contains("price"));
        assert!(!s.contains("moscow"));
        assert!(s.contains("fees"));
        assert!(s.contains("difficulty"));

        let parsed = DisplayItems::from_items_string(&s);
        assert_eq!(parsed.block_height, true);
        assert_eq!(parsed.price, true);
        assert_eq!(parsed.moscow_time, false);
        assert_eq!(parsed.fees, true);
        assert_eq!(parsed.halving_countdown, false);
        assert_eq!(parsed.difficulty_adjustment, true);
        assert_eq!(parsed.mempool_size, false);
    }

    #[test]
    fn test_price_currency_roundtrip() {
        assert_eq!(PriceCurrency::from_str("USD"), PriceCurrency::USD);
        assert_eq!(PriceCurrency::from_str("EUR"), PriceCurrency::EUR);
        assert_eq!(PriceCurrency::from_str("unknown"), PriceCurrency::USD);
        assert_eq!(PriceCurrency::USD.as_str(), "USD");
        assert_eq!(PriceCurrency::EUR.as_str(), "EUR");
    }

    #[test]
    fn test_serde_prices_response() {
        let json = r#"{"USD":87000,"EUR":82000,"GBP":70000,"CAD":120000,"CHF":78000,"AUD":135000,"JPY":13000000}"#;
        let resp: PricesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.usd, 87000);
        assert_eq!(resp.eur, 82000);
    }

    #[test]
    fn test_serde_fees_response() {
        let json = r#"{"fastestFee":5,"halfHourFee":3,"hourFee":2,"economyFee":1,"minimumFee":1}"#;
        let resp: FeesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.fastest_fee, 5);
        assert_eq!(resp.half_hour_fee, 3);
        assert_eq!(resp.hour_fee, 2);
    }

    #[test]
    fn test_serde_difficulty_response() {
        let json = r#"{"progressPercent":45.23,"difficultyChange":-1.52,"estimatedRetargetDate":1700000000,"remainingBlocks":1050,"remainingTime":630000,"fastestFee":5,"halfHourFee":3,"hourFee":2,"economyFee":1,"nextRetargetHeight":893088,"timeAvg":600000,"timeOffset":0,"expectedBlocks":1008}"#;
        let resp: DifficultyResponse = serde_json::from_str(json).unwrap();
        assert!((resp.progress_percent - 45.23).abs() < 0.01);
        assert!((resp.difficulty_change - (-1.52)).abs() < 0.01);
    }

    #[test]
    fn test_serde_mempool_response() {
        let json = r#"{"count":18900,"vsize":67890000,"total_fee":1234567,"fee_histogram":[]}"#;
        let resp: MempoolStatsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 18900);
    }

    #[test]
    fn test_display_items_needs() {
        let items = DisplayItems {
            block_height: true,
            price: false,
            moscow_time: false,
            fees: false,
            halving_countdown: false,
            difficulty_adjustment: false,
            mempool_size: false,
        };
        assert!(items.needs_block_height());
        assert!(!items.needs_prices());

        let items2 = DisplayItems {
            block_height: false,
            price: false,
            moscow_time: true,
            fees: false,
            halving_countdown: true,
            difficulty_adjustment: false,
            mempool_size: false,
        };
        assert!(items2.needs_block_height()); // halving needs block height
        assert!(items2.needs_prices()); // moscow time needs prices
    }
}
