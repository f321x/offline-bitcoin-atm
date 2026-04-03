pub const COIN_MAP: [u64; 10] = [0, 0, 5, 10, 20, 50, 100, 200, 1, 2];

/// Convert pulse count to cent value. Returns None for invalid pulse counts.
pub fn pulses_to_cents(pulses: u32) -> Option<u64> {
    if pulses >= 2 && (pulses as usize) < COIN_MAP.len() {
        let val = COIN_MAP[pulses as usize];
        if val > 0 {
            Some(val)
        } else {
            None
        }
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoinInteraction {
    Button,
    Coin(u64),
}

#[cfg(feature = "esp")]
mod esp_impl {
    use super::*;
    use esp_idf_hal::gpio::{Input, InterruptType, Output, PinDriver};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    const PULSE_TIMEOUT_MS: u64 = 200;
    const DEBOUNCE_US: u32 = 35_000; // 35ms debounce, matches C++ reference

    /// ISR-private debounce timestamp (µs, truncated to u32 — wraps every ~71 min)
    static LAST_PULSE_US: AtomicU32 = AtomicU32::new(0);

    pub struct CoinDetector<'a> {
        coin_pin: PinDriver<'a, Input>,
        mosfet_pin: PinDriver<'a, Output>,
        pulse_count: Arc<AtomicU32>,
        last_pulse_snapshot: u32,
        last_change_time: Instant,
    }

    impl<'a> CoinDetector<'a> {
        pub fn new(mut coin_pin: PinDriver<'a, Input>, mosfet_pin: PinDriver<'a, Output>) -> Self {
            let pulse_count = Arc::new(AtomicU32::new(0));

            // Set up falling-edge interrupt on coin pin
            coin_pin.set_interrupt_type(InterruptType::NegEdge).unwrap();
            let counter = pulse_count.clone();
            let pin_num = coin_pin.pin() as i32;
            // SAFETY: The closure is Send and lives as long as the PinDriver (owned by Self).
            // Only one ISR subscriber per pin; the atomic counter is the sole shared state.
            // esp_timer_get_time(), LAST_PULSE_US, and gpio_intr_enable() are ISR-safe.
            unsafe {
                coin_pin
                    .subscribe(move || {
                        let now = esp_idf_svc::sys::esp_timer_get_time() as u32;
                        let prev = LAST_PULSE_US.load(Ordering::Relaxed);
                        if now.wrapping_sub(prev) >= DEBOUNCE_US {
                            LAST_PULSE_US.store(now, Ordering::Relaxed);
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                        // Re-enable GPIO interrupt — handle_isr() disables it before calling us
                        esp_idf_svc::sys::gpio_intr_enable(pin_num);
                    })
                    .unwrap();
            }
            coin_pin.enable_interrupt().unwrap();

            Self {
                coin_pin,
                mosfet_pin,
                pulse_count,
                last_pulse_snapshot: 0,
                last_change_time: Instant::now(),
            }
        }

        pub fn set_accepting(&mut self, accepting: bool) {
            if accepting {
                let _ = self.mosfet_pin.set_low();
            } else {
                let _ = self.mosfet_pin.set_high();
            }
        }

        /// Non-blocking check for completed coin insertion.
        /// Call this periodically from the main loop (~10ms interval).
        /// Returns Some(cents) when a complete pulse train has been received
        /// (no new pulses for PULSE_TIMEOUT_MS).
        pub fn check_for_coin(&mut self) -> Option<u64> {
            let current = self.pulse_count.load(Ordering::Relaxed);

            if current == 0 {
                return None;
            }

            // New pulses arrived since last check
            if current != self.last_pulse_snapshot {
                log::debug!(
                    "Pulse count changed: {} -> {}",
                    self.last_pulse_snapshot,
                    current
                );
                // Disable MOSFET on first pulse to prevent overlapping coins
                if self.last_pulse_snapshot == 0 {
                    self.set_accepting(false);
                }
                self.last_pulse_snapshot = current;
                self.last_change_time = Instant::now();
                return None;
            }

            // Same count as last check — check if timeout has passed
            if self.last_change_time.elapsed() >= std::time::Duration::from_millis(PULSE_TIMEOUT_MS)
            {
                // Pulse train complete, consume the count
                let pulses = self.pulse_count.swap(0, Ordering::Relaxed);
                self.last_pulse_snapshot = 0;
                log::info!("Pulse train complete: {} raw pulses", pulses);

                // Re-enable interrupt for next coin
                let _ = self.coin_pin.enable_interrupt();

                let cents = pulses_to_cents(pulses);
                if cents.is_none() {
                    log::warn!("Invalid pulse count {}, ignoring", pulses);
                    self.set_accepting(true); // Re-enable — no deadlock
                }
                return cents;
            }

            None
        }

        /// Blocking wait for either a coin insertion or button press.
        /// Used for the main event loop.
        pub fn wait_for_event<F>(&mut self, check_button: F) -> CoinInteraction
        where
            F: Fn() -> bool,
        {
            self.set_accepting(true);

            loop {
                if check_button() {
                    log::debug!("Button press detected (pin LOW)");
                    self.set_accepting(false);
                    return CoinInteraction::Button;
                }

                if let Some(cents) = self.check_for_coin() {
                    log::debug!("Coin detected: {} cents", cents);
                    return CoinInteraction::Coin(cents);
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

#[cfg(feature = "esp")]
pub use esp_impl::CoinDetector;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulses_to_cents_valid_range() {
        assert_eq!(pulses_to_cents(2), Some(5)); // 5 cent
        assert_eq!(pulses_to_cents(3), Some(10)); // 10 cent
        assert_eq!(pulses_to_cents(4), Some(20)); // 20 cent
        assert_eq!(pulses_to_cents(5), Some(50)); // 50 cent
        assert_eq!(pulses_to_cents(6), Some(100)); // 1 euro
        assert_eq!(pulses_to_cents(7), Some(200)); // 2 euro
        assert_eq!(pulses_to_cents(8), Some(1)); // 1 cent
        assert_eq!(pulses_to_cents(9), Some(2)); // 2 cent
    }

    #[test]
    fn test_pulses_to_cents_zero() {
        assert_eq!(pulses_to_cents(0), None);
    }

    #[test]
    fn test_pulses_to_cents_one() {
        assert_eq!(pulses_to_cents(1), None);
    }

    #[test]
    fn test_pulses_to_cents_out_of_range() {
        assert_eq!(pulses_to_cents(10), None);
        assert_eq!(pulses_to_cents(100), None);
        assert_eq!(pulses_to_cents(u32::MAX), None);
    }

    #[test]
    fn test_coin_map_matches_hardware_spec() {
        // HX-616 coin acceptor pulse mapping
        assert_eq!(COIN_MAP, [0, 0, 5, 10, 20, 50, 100, 200, 1, 2]);
    }

    #[test]
    fn test_coin_interaction_enum() {
        assert_eq!(CoinInteraction::Button, CoinInteraction::Button);
        assert_eq!(CoinInteraction::Coin(100), CoinInteraction::Coin(100));
        assert_ne!(CoinInteraction::Coin(100), CoinInteraction::Coin(200));
        assert_ne!(CoinInteraction::Coin(100), CoinInteraction::Button);
    }
}
