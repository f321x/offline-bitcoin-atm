use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    prelude::*,
    text::Text,
};
use epd_waveshare::{epd2in13_v2::*, prelude::*};

use super::rendering::{draw_orangeclock_layout, draw_qr, orangeclock_rows};
use super::AtmDisplay;
use crate::mempool::{DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::IconSize;
use crate::util::generate_qrcode;

pub struct Display2in13Wrapper<SPI, BUSY, DC, RST, DELAY> {
    pub(crate) epd: Epd2in13<SPI, BUSY, DC, RST, DELAY>,
    pub(crate) rotation: DisplayRotation,
}

impl<SPI, BUSY, DC, RST, DELAY> AtmDisplay<SPI, DELAY>
    for Display2in13Wrapper<SPI, BUSY, DC, RST, DELAY>
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
        // Epd2in13 V2 resolution is 122x250
        let mut display_buffer = Display2in13::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let large_style = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);
        let small_style = MonoTextStyle::new(&FONT_6X10, Color::Black);

        Text::new("LIGHTNING ATM", Point::new(5, 5), large_style).draw(&mut display_buffer)?;
        Text::new(
            "Insert coins\non the right\nside to start",
            Point::new(3, 33),
            small_style,
        )
        .draw(&mut display_buffer)?;

        Text::new(
            "Prepare Lightning enabled Bitcoin\nwallet before starting!",
            Point::new(0, 95),
            small_style,
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

    fn show_inserted_amount(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        amount_string: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.epd
            .wake_up(spi, delay)
            .map_err(|_| "Failed to wake up display")?;
        let mut display_buffer = Display2in13::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let large_style = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);

        Text::new("Inserted amount:", Point::new(10, 4), large_style).draw(&mut display_buffer)?;
        Text::new(amount_string, Point::new(35, 45), large_style).draw(&mut display_buffer)?;
        Text::new(
            " Press button\n to show QR code",
            Point::new(0, 85),
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
        // Epd2in13 V2 resolution 122x250, rotated to 250x122
        let mut display_buffer = Display2in13::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let matrix = generate_qrcode(qr_content).map_err(|_| "Failed to generate QR code")?;
        let qr_size = matrix.len() as i32;
        let module_size: i32 = 2;
        // Place QR on the right side, text label on the left
        let qr_area_x = 250 - 10; // right margin
        let start_x = qr_area_x - qr_size * module_size;
        let start_y = (122 - qr_size * module_size) / 2;

        draw_qr(&mut display_buffer, &matrix, start_x, start_y, module_size)?;

        let style = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);
        Text::new("Scan\n\nQR\n\ncode", Point::new(5, 20), style).draw(&mut display_buffer)?;

        let small_style = MonoTextStyle::new(&FONT_6X10, Color::Black);
        Text::new("Reset:\npress\nbutton", Point::new(5, 95), small_style)
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
        let mut display_buffer = Display2in13::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let rows = orangeclock_rows(data, items, currency);
        draw_orangeclock_layout(
            &mut display_buffer,
            &rows,
            250,
            122,
            IconSize::Small,
            &FONT_6X10,
            3,
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
