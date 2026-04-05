use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{ascii::*, MonoTextStyle},
    prelude::*,
    text::Text,
};
use epd_waveshare::{epd2in7b::*, prelude::*};

use super::rendering::{draw_orangeclock_layout, draw_qr, orangeclock_rows};
use super::{AtmDisplay, BITCOIN_LOGO_64X64};
use crate::mempool::{DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::IconSize;
use crate::util::generate_qrcode;

/// Blank chromatic (red) buffer for Epd2in7b tri-color display (176*264/8 = 5808 bytes).
/// All 0xFF = no red pixels. Stored as a static to avoid repeated 5.7KB stack allocations.
static CHROMATIC_BLANK: [u8; 176 * 264 / 8] = [0xFFu8; 176 * 264 / 8];

pub struct Display2in7Wrapper<SPI, BUSY, DC, RST, DELAY> {
    pub(crate) epd: Epd2in7b<SPI, BUSY, DC, RST, DELAY>,
    pub(crate) rotation: DisplayRotation,
}

impl<SPI, BUSY, DC, RST, DELAY> AtmDisplay<SPI, DELAY>
    for Display2in7Wrapper<SPI, BUSY, DC, RST, DELAY>
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
        // Epd2in7b resolution 176 x 264
        let mut display_buffer = Display2in7b::default();
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
            .update_color_frame(spi, delay, display_buffer.buffer(), &CHROMATIC_BLANK)
            .map_err(|_| "Failed to update color frame")?;
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
        let mut display_buffer = Display2in7b::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let large_style = MonoTextStyle::new(&FONT_10X20, Color::Black);

        Text::new("Inserted amount:", Point::new(11, 10), large_style).draw(&mut display_buffer)?;
        Text::new(amount_string, Point::new(20, 75), large_style).draw(&mut display_buffer)?;
        Text::new(
            "Press button\n once finished.",
            Point::new(11, 135),
            large_style,
        )
        .draw(&mut display_buffer)?;

        self.epd
            .update_color_frame(spi, delay, display_buffer.buffer(), &CHROMATIC_BLANK)
            .map_err(|_| "Failed to update color frame")?;
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
        // Epd2in7b resolution 176x264, rotated to 264x176
        let mut display_buffer = Display2in7b::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let matrix = generate_qrcode(qr_content).map_err(|_| "Failed to generate QR code")?;
        let qr_size = matrix.len() as i32;
        let module_size: i32 = 3;
        let start_x = (264 - qr_size * module_size) / 2;
        let start_y = (176 - qr_size * module_size) / 2;

        draw_qr(&mut display_buffer, &matrix, start_x, start_y, module_size)?;

        let style = MonoTextStyle::new(&FONT_6X10, Color::Black);
        Text::new("Scan QR code", Point::new(11, 10), style).draw(&mut display_buffer)?;
        Text::new("Reset - press button", Point::new(11, 170), style).draw(&mut display_buffer)?;

        self.epd
            .update_color_frame(spi, delay, display_buffer.buffer(), &CHROMATIC_BLANK)
            .map_err(|_| "Failed to update color frame")?;
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
        let mut display_buffer = Display2in7b::default();
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
            .update_color_frame(spi, delay, display_buffer.buffer(), &CHROMATIC_BLANK)
            .map_err(|_| "Failed to update color frame")?;
        self.epd
            .display_frame(spi, delay)
            .map_err(|_| "Failed to display frame")?;
        self.epd
            .sleep(spi, delay)
            .map_err(|_| "Failed to sleep display")?;
        Ok(())
    }
}
