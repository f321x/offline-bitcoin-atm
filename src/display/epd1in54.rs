use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    prelude::*,
    text::Text,
};
use epd_waveshare::{epd1in54_v2::*, prelude::*};

use super::rendering::{draw_orangeclock_layout, draw_qr, orangeclock_rows};
use super::AtmDisplay;
use crate::mempool::{DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::IconSize;
use crate::util::generate_qrcode;

pub struct Display1in54Wrapper<SPI, BUSY, DC, RST, DELAY> {
    pub(crate) epd: Epd1in54<SPI, BUSY, DC, RST, DELAY>,
    pub(crate) rotation: DisplayRotation,
}

impl<SPI, BUSY, DC, RST, DELAY> AtmDisplay<SPI, DELAY>
    for Display1in54Wrapper<SPI, BUSY, DC, RST, DELAY>
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
        let mut display_buffer = Display1in54::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let large_style = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);
        let small_style = MonoTextStyle::new(&FONT_6X10, Color::Black);

        Text::new(
            "Insert\nEuro coins\non the\nright\nside to\nstart ->",
            Point::new(0, 10),
            large_style,
        )
        .draw(&mut display_buffer)
        .map_err(|_| "Failed to draw main text")?;

        Text::new("Prepare Lightning enabled Bitcoin\nwallet before starting!\n\nSupported coins:\n1 Cent and 2 Euro", Point::new(0, 160), small_style)
            .draw(&mut display_buffer)
            .map_err(|_| "Failed to draw instructions")?;

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
        let mut display_buffer = Display1in54::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let large_style = MonoTextStyle::new(&FONT_9X18_BOLD, Color::Black);
        let huge_style = MonoTextStyle::new(&FONT_10X20, Color::Black);

        Text::new("Inserted amount:", Point::new(0, 4), large_style).draw(&mut display_buffer)?;
        Text::new(amount_string, Point::new(10, 90), huge_style).draw(&mut display_buffer)?;
        Text::new(
            " Press button\n once finished.",
            Point::new(0, 160),
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
        let mut display_buffer = Display1in54::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let matrix = generate_qrcode(qr_content).map_err(|_| "Failed to generate QR code")?;
        let qr_size = matrix.len() as i32;
        let module_size: i32 = 3;
        let qr_area = 150;
        let start_x = (qr_area - qr_size * module_size) / 2;
        let start_y = 20 + (qr_area - qr_size * module_size) / 2;

        let small_style = MonoTextStyle::new(&FONT_6X10, Color::Black);
        Text::new("Please scan QR code:", Point::new(0, 12), small_style)
            .draw(&mut display_buffer)?;

        draw_qr(&mut display_buffer, &matrix, start_x, start_y, module_size)?;

        Text::new("Press button to reset", Point::new(0, 190), small_style)
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
        let mut display_buffer = Display1in54::default();
        display_buffer.set_rotation(self.rotation);
        display_buffer
            .clear(Color::White)
            .map_err(|_| "Failed to clear buffer")?;

        let rows = orangeclock_rows(data, items, currency);
        draw_orangeclock_layout(
            &mut display_buffer,
            &rows,
            200,
            200,
            IconSize::Small,
            &FONT_9X18_BOLD,
            5,
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
