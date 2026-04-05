use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{ascii::*, MonoTextStyle},
    prelude::*,
    text::Text,
};
use epd_waveshare::{epd2in7, epd2in7_v2, prelude::*};

use super::rendering::{draw_orangeclock_layout, draw_qr, orangeclock_rows};
use super::{AtmDisplay, BITCOIN_LOGO_64X64};
use crate::mempool::{DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::IconSize;
use crate::util::generate_qrcode;

// V1 and V2 share identical resolution (176x264) and layout; only the driver module differs.

macro_rules! impl_2in7_bw_display {
    ($wrapper:ident, $epd_mod:ident) => {
        pub struct $wrapper<SPI, BUSY, DC, RST, DELAY> {
            pub(crate) epd: $epd_mod::Epd2in7<SPI, BUSY, DC, RST, DELAY>,
            pub(crate) rotation: DisplayRotation,
        }

        impl<SPI, BUSY, DC, RST, DELAY> AtmDisplay<SPI, DELAY>
            for $wrapper<SPI, BUSY, DC, RST, DELAY>
        where
            SPI: embedded_hal::spi::SpiDevice,
            DELAY: embedded_hal::delay::DelayNs,
            BUSY: embedded_hal::digital::InputPin,
            DC: embedded_hal::digital::OutputPin,
            RST: embedded_hal::digital::OutputPin,
        {
            fn home_screen(
                &mut self,
                spi: &mut SPI,
                delay: &mut DELAY,
            ) -> Result<(), Box<dyn std::error::Error>> {
                self.epd
                    .wake_up(spi, delay)
                    .map_err(|_| "Failed to wake up display")?;
                let mut display_buffer = $epd_mod::Display2in7::default();
                display_buffer.set_rotation(self.rotation);
                display_buffer
                    .clear(Color::White)
                    .map_err(|_| "Failed to clear buffer")?;

                let large_style = MonoTextStyle::new(&FONT_10X20, Color::Black);
                let small_style = MonoTextStyle::new(&FONT_6X10, Color::Black);

                Text::new(
                    "Insert Euro coins\n on the right ->\n to start ATM",
                    Point::new(11, 20),
                    large_style,
                )
                .draw(&mut display_buffer)?;

                let logo_raw: ImageRaw<Color> = ImageRaw::new(&BITCOIN_LOGO_64X64, 64);
                Image::new(&logo_raw, Point::new(195, 56)).draw(&mut display_buffer)?;

                Text::new("Prepare Lightning enabled Bitcoin\n  wallet before starting!\n  Supported coins: 5 - 50 Cent, 1 - 2 Euro", Point::new(12, 140), small_style)
                    .draw(&mut display_buffer)?;

                self.epd
                    .update_frame(spi, display_buffer.buffer(), delay)
                    .map_err(|_| "Failed to update frame")?;
                self.epd
                    .display_frame(spi, delay)
                    .map_err(|_| "Failed to display frame")?;
                self.epd
                    .sleep(spi, delay)
                    .map_err(|_| "Failed to sleep display")?;
                Ok(())
            }

            fn show_inserted_amount(
                &mut self,
                spi: &mut SPI,
                delay: &mut DELAY,
                amount_string: &str,
            ) -> Result<(), Box<dyn std::error::Error>> {
                self.epd
                    .wake_up(spi, delay)
                    .map_err(|_| "Failed to wake up display")?;
                let mut display_buffer = $epd_mod::Display2in7::default();
                display_buffer.set_rotation(self.rotation);
                display_buffer
                    .clear(Color::White)
                    .map_err(|_| "Failed to clear buffer")?;

                let large_style = MonoTextStyle::new(&FONT_10X20, Color::Black);

                Text::new("Inserted amount:", Point::new(11, 10), large_style)
                    .draw(&mut display_buffer)?;
                Text::new(amount_string, Point::new(20, 75), large_style)
                    .draw(&mut display_buffer)?;
                Text::new(
                    "Press button\n once finished.",
                    Point::new(11, 135),
                    large_style,
                )
                .draw(&mut display_buffer)?;

                self.epd
                    .update_frame(spi, display_buffer.buffer(), delay)
                    .map_err(|_| "Failed to update frame")?;
                self.epd
                    .display_frame(spi, delay)
                    .map_err(|_| "Failed to display frame")?;
                self.epd
                    .sleep(spi, delay)
                    .map_err(|_| "Failed to sleep display")?;
                Ok(())
            }

            fn show_qr(
                &mut self,
                spi: &mut SPI,
                delay: &mut DELAY,
                qr_content: &str,
            ) -> Result<(), Box<dyn std::error::Error>> {
                self.epd
                    .wake_up(spi, delay)
                    .map_err(|_| "Failed to wake up display")?;
                let mut display_buffer = $epd_mod::Display2in7::default();
                display_buffer.set_rotation(self.rotation);
                display_buffer
                    .clear(Color::White)
                    .map_err(|_| "Failed to clear buffer")?;

                let matrix =
                    generate_qrcode(qr_content).map_err(|_| "Failed to generate QR code")?;
                let qr_size = matrix.len() as i32;
                let module_size: i32 = 3;
                let start_x = (264 - qr_size * module_size) / 2;
                let start_y = (176 - qr_size * module_size) / 2;

                draw_qr(&mut display_buffer, &matrix, start_x, start_y, module_size)?;

                let style = MonoTextStyle::new(&FONT_6X10, Color::Black);
                Text::new("Scan QR code", Point::new(11, 10), style)
                    .draw(&mut display_buffer)?;
                Text::new("Reset - press button", Point::new(11, 170), style)
                    .draw(&mut display_buffer)?;

                self.epd
                    .update_frame(spi, display_buffer.buffer(), delay)
                    .map_err(|_| "Failed to update frame")?;
                self.epd
                    .display_frame(spi, delay)
                    .map_err(|_| "Failed to display frame")?;
                self.epd
                    .sleep(spi, delay)
                    .map_err(|_| "Failed to sleep display")?;
                Ok(())
            }

            fn clean(
                &mut self,
                spi: &mut SPI,
                delay: &mut DELAY,
            ) -> Result<(), Box<dyn std::error::Error>> {
                self.epd
                    .wake_up(spi, delay)
                    .map_err(|_| "Failed to wake up display")?;
                self.epd
                    .clear_frame(spi, delay)
                    .map_err(|_| "Failed to clear frame")?;
                self.epd
                    .display_frame(spi, delay)
                    .map_err(|_| "Failed to display frame")?;
                self.epd
                    .sleep(spi, delay)
                    .map_err(|_| "Failed to sleep display")?;
                Ok(())
            }

            fn show_orangeclock(
                &mut self,
                spi: &mut SPI,
                delay: &mut DELAY,
                data: &MempoolData,
                items: &DisplayItems,
                currency: &PriceCurrency,
            ) -> Result<(), Box<dyn std::error::Error>> {
                self.epd
                    .wake_up(spi, delay)
                    .map_err(|_| "Failed to wake up display")?;
                let mut display_buffer = $epd_mod::Display2in7::default();
                display_buffer.set_rotation(self.rotation);
                display_buffer
                    .clear(Color::White)
                    .map_err(|_| "Failed to clear buffer")?;

                let rows = orangeclock_rows(data, items, currency);
                draw_orangeclock_layout(
                    &mut display_buffer,
                    &rows,
                    264,
                    176,
                    IconSize::Large,
                    &FONT_10X20,
                    4,
                )?;

                self.epd
                    .update_frame(spi, display_buffer.buffer(), delay)
                    .map_err(|_| "Failed to update frame")?;
                self.epd
                    .display_frame(spi, delay)
                    .map_err(|_| "Failed to display frame")?;
                self.epd
                    .sleep(spi, delay)
                    .map_err(|_| "Failed to sleep display")?;
                Ok(())
            }
        }
    };
}

impl_2in7_bw_display!(Display2in7BwWrapper, epd2in7);
impl_2in7_bw_display!(Display2in7V2Wrapper, epd2in7_v2);
