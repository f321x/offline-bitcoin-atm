//! logic for user configuration of the atm, persisted in esp flash

use crate::mempool::{DisplayItems, OrangeClockConfig, PriceCurrency};
use crate::util::LNBitsConnection;

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

// NVS key constants
const KEY_BASE_URL: &str = "base_url";
const KEY_PAGE_BASE_URL: &str = "page_base_url";
const KEY_ATM_SECRET: &str = "atm_secret";
const KEY_CURRENCY: &str = "currency";
const KEY_DISPLAY_TYPE: &str = "display_type";
const KEY_BOARD_TYPE: &str = "board_type";
const KEY_ROTATION: &str = "rotation";

// OrangeClock NVS keys (all ≤15 chars)
const KEY_OC_ENABLED: &str = "oc_enabled";
const KEY_OC_SSID: &str = "oc_ssid";
const KEY_OC_PASS: &str = "oc_pass";
const KEY_OC_MEMPOOL: &str = "oc_mempool";
const KEY_OC_ITEMS: &str = "oc_items";
const KEY_OC_CURRENCY: &str = "oc_currency";

pub const DEFAULT_DISPLAY_TYPE: &str = "GxEPD2_150_BN";
pub const DEFAULT_BOARD_TYPE: &str = "Generic";
pub const DEFAULT_ROTATION: &str = "270";

pub struct Config {
    nvs: EspNvs<NvsDefault>,
}

impl Config {
    pub fn open() -> Result<Self, esp_idf_svc::sys::EspError> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let nvs = EspNvs::new(nvs_partition, "config", true)?;
        log::debug!("loaded config from storage");
        Ok(Self { nvs })
    }

    pub fn persist_lnbits_connection(
        &mut self,
        connection: &LNBitsConnection,
    ) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_str(KEY_BASE_URL, &connection.base_url)?;
        self.nvs
            .set_str(KEY_PAGE_BASE_URL, &connection.page_base_url)?;
        self.nvs.set_str(KEY_ATM_SECRET, &connection.atm_secret)?;
        self.nvs.set_str(KEY_CURRENCY, &connection.currency)?;
        log::debug!("persisted lnbits connection to storage: {:?}", connection);
        Ok(())
    }

    pub fn get_lnbits_connection(
        &self,
    ) -> Result<Option<LNBitsConnection>, esp_idf_svc::sys::EspError> {
        let mut buf = [0u8; 256];

        let base_url = match self.nvs.get_str(KEY_BASE_URL, &mut buf)? {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };

        let page_base_url = match self.nvs.get_str(KEY_PAGE_BASE_URL, &mut buf)? {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };

        let atm_secret = match self.nvs.get_str(KEY_ATM_SECRET, &mut buf)? {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };

        let currency = match self.nvs.get_str(KEY_CURRENCY, &mut buf)? {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };

        log::debug!(
            "Loaded lnbits connection: {:?} | {:?} | {:?} | {:?}",
            base_url,
            page_base_url,
            atm_secret,
            currency
        );

        Ok(Some(LNBitsConnection {
            base_url,
            page_base_url,
            atm_secret,
            currency,
        }))
    }

    pub fn persist_display_type(
        &mut self,
        display_type: &str,
    ) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_str(KEY_DISPLAY_TYPE, display_type)?;
        log::debug!("persisted display type: {}", display_type);
        Ok(())
    }

    pub fn persist_board_type(
        &mut self,
        board_type: &str,
    ) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_str(KEY_BOARD_TYPE, board_type)?;
        log::debug!("persisted board type: {}", board_type);
        Ok(())
    }

    pub fn get_board_type(&self) -> Result<String, esp_idf_svc::sys::EspError> {
        let mut buf = [0u8; 64];
        match self.nvs.get_str(KEY_BOARD_TYPE, &mut buf)? {
            Some(s) => Ok(s.to_string()),
            None => Ok(DEFAULT_BOARD_TYPE.to_string()),
        }
    }

    pub fn get_display_type(&self) -> Result<String, esp_idf_svc::sys::EspError> {
        let mut buf = [0u8; 64];
        match self.nvs.get_str(KEY_DISPLAY_TYPE, &mut buf)? {
            Some(s) => Ok(s.to_string()),
            None => Ok(DEFAULT_DISPLAY_TYPE.to_string()),
        }
    }

    pub fn persist_rotation(&mut self, rotation: &str) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs.set_str(KEY_ROTATION, rotation)?;
        log::debug!("persisted rotation: {}", rotation);
        Ok(())
    }

    pub fn get_rotation(&self) -> Result<String, esp_idf_svc::sys::EspError> {
        let mut buf = [0u8; 16];
        match self.nvs.get_str(KEY_ROTATION, &mut buf)? {
            Some(s) => Ok(s.to_string()),
            None => Ok(DEFAULT_ROTATION.to_string()),
        }
    }

    // --- OrangeClock configuration ---

    pub fn persist_orangeclock(
        &mut self,
        oc: &OrangeClockConfig,
    ) -> Result<(), esp_idf_svc::sys::EspError> {
        self.nvs
            .set_str(KEY_OC_ENABLED, if oc.enabled { "1" } else { "0" })?;
        self.nvs.set_str(KEY_OC_SSID, &oc.wifi_ssid)?;
        self.nvs.set_str(KEY_OC_PASS, &oc.wifi_password)?;
        self.nvs.set_str(KEY_OC_MEMPOOL, &oc.mempool_endpoint)?;
        self.nvs
            .set_str(KEY_OC_ITEMS, &oc.display_items.to_items_string())?;
        self.nvs
            .set_str(KEY_OC_CURRENCY, oc.price_currency.as_str())?;
        log::debug!("persisted OrangeClock config (enabled={})", oc.enabled);
        Ok(())
    }

    pub fn get_orangeclock(&self) -> Result<Option<OrangeClockConfig>, esp_idf_svc::sys::EspError> {
        let mut buf = [0u8; 64];

        let enabled = match self.nvs.get_str(KEY_OC_ENABLED, &mut buf)? {
            Some(s) => s == "1",
            None => return Ok(None),
        };

        if !enabled {
            return Ok(None);
        }

        let mut buf_large = [0u8; 256];

        let wifi_ssid = match self.nvs.get_str(KEY_OC_SSID, &mut buf_large)? {
            Some(s) => s.to_string(),
            None => return Ok(None),
        };

        let wifi_password = match self.nvs.get_str(KEY_OC_PASS, &mut buf_large)? {
            Some(s) => s.to_string(),
            None => String::new(),
        };

        let mempool_endpoint = match self.nvs.get_str(KEY_OC_MEMPOOL, &mut buf_large)? {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => crate::mempool::DEFAULT_ENDPOINT.to_string(),
        };

        let display_items = match self.nvs.get_str(KEY_OC_ITEMS, &mut buf_large)? {
            Some(s) => DisplayItems::from_items_string(s),
            None => DisplayItems::default_items(),
        };

        let price_currency = match self.nvs.get_str(KEY_OC_CURRENCY, &mut buf)? {
            Some(s) => PriceCurrency::from_str(s),
            None => PriceCurrency::USD,
        };

        log::debug!(
            "Loaded OrangeClock config: ssid={}, endpoint={}, currency={:?}",
            wifi_ssid,
            mempool_endpoint,
            price_currency
        );

        Ok(Some(OrangeClockConfig {
            enabled,
            wifi_ssid,
            wifi_password,
            mempool_endpoint,
            display_items,
            price_currency,
        }))
    }
}
