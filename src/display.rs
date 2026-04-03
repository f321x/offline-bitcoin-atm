use embedded_graphics::{
    geometry::Size,
    image::{Image, ImageRaw},
    mono_font::{ascii::*, MonoFont, MonoTextStyle},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use epd_waveshare::{epd1in54_v2::*, epd2in13_v2::*, epd2in7, epd2in7_v2, epd2in7b::*, prelude::*};

use crate::mempool::{self, DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::{self, IconSize, OrangeClockItem};
use crate::util::generate_qrcode;

pub fn parse_rotation(s: &str) -> DisplayRotation {
    match s {
        "0" => DisplayRotation::Rotate0,
        "90" => DisplayRotation::Rotate90,
        "180" => DisplayRotation::Rotate180,
        _ => DisplayRotation::Rotate270,
    }
}

/// Blank chromatic (red) buffer for Epd2in7b tri-color display (176*264/8 = 5808 bytes).
/// All 0xFF = no red pixels. Stored as a static to avoid repeated 5.7KB stack allocations.
static CHROMATIC_BLANK: [u8; 176 * 264 / 8] = [0xFFu8; 176 * 264 / 8];

/// 64x64 pixel Bitcoin logo bitmap (1 bit per pixel, MSB first, 8 bytes per row).
#[rustfmt::skip]
static BITCOIN_LOGO_64X64: [u8; 512] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x3f, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xff, 0xff, 0x80, 0x00, 0x00,
    0x00, 0x00, 0x0f, 0xff, 0xff, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x3f, 0xff, 0xff, 0xfc, 0x00, 0x00,
    0x00, 0x00, 0x7f, 0xff, 0xff, 0xfe, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
    0x00, 0x03, 0xff, 0xff, 0xff, 0xff, 0xc0, 0x00, 0x00, 0x07, 0xff, 0xff, 0xff, 0xff, 0xe0, 0x00,
    0x00, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xf0, 0x00, 0x00, 0x0f, 0xff, 0xfc, 0x7f, 0xff, 0xf0, 0x00,
    0x00, 0x1f, 0xff, 0xfc, 0x63, 0xff, 0xf8, 0x00, 0x00, 0x3f, 0xff, 0xfc, 0x63, 0xff, 0xfc, 0x00,
    0x00, 0x7f, 0xfe, 0x38, 0xe3, 0xff, 0xfe, 0x00, 0x00, 0x7f, 0xfe, 0x00, 0xe3, 0xff, 0xfe, 0x00,
    0x00, 0xff, 0xfe, 0x00, 0x03, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0x80, 0x03, 0xff, 0xff, 0x00,
    0x00, 0xff, 0xff, 0xc0, 0x00, 0xff, 0xff, 0x80, 0x01, 0xff, 0xff, 0xc0, 0x00, 0x7f, 0xff, 0x80,
    0x01, 0xff, 0xff, 0xc1, 0xe0, 0x3f, 0xff, 0x80, 0x01, 0xff, 0xff, 0x81, 0xf8, 0x1f, 0xff, 0x80,
    0x03, 0xff, 0xff, 0x83, 0xf8, 0x1f, 0xff, 0xc0, 0x03, 0xff, 0xff, 0x83, 0xf8, 0x1f, 0xff, 0xc0,
    0x03, 0xff, 0xff, 0x83, 0xf8, 0x1f, 0xff, 0xc0, 0x03, 0xff, 0xff, 0x01, 0xf0, 0x1f, 0xff, 0xc0,
    0x03, 0xff, 0xff, 0x00, 0x00, 0x3f, 0xff, 0xc0, 0x03, 0xff, 0xff, 0x00, 0x00, 0x7f, 0xff, 0xc0,
    0x03, 0xff, 0xff, 0x06, 0x00, 0xff, 0xff, 0xc0, 0x03, 0xff, 0xfe, 0x07, 0xc0, 0x7f, 0xff, 0xc0,
    0x03, 0xff, 0xfe, 0x0f, 0xe0, 0x3f, 0xff, 0xc0, 0x03, 0xff, 0xfe, 0x0f, 0xf0, 0x3f, 0xff, 0xc0,
    0x03, 0xff, 0xec, 0x0f, 0xf0, 0x3f, 0xff, 0xc0, 0x03, 0xff, 0xe0, 0x0f, 0xf0, 0x3f, 0xff, 0xc0,
    0x01, 0xff, 0xc0, 0x0f, 0xf0, 0x3f, 0xff, 0x80, 0x01, 0xff, 0xc0, 0x00, 0x00, 0x3f, 0xff, 0x80,
    0x01, 0xff, 0xf8, 0x00, 0x00, 0x7f, 0xff, 0x80, 0x01, 0xff, 0xfe, 0x00, 0x00, 0x7f, 0xff, 0x00,
    0x00, 0xff, 0xfe, 0x30, 0x00, 0xff, 0xff, 0x00, 0x00, 0xff, 0xfe, 0x38, 0xc7, 0xff, 0xff, 0x00,
    0x00, 0x7f, 0xfe, 0x31, 0xff, 0xff, 0xfe, 0x00, 0x00, 0x7f, 0xfc, 0x31, 0xff, 0xff, 0xfe, 0x00,
    0x00, 0x3f, 0xff, 0xf1, 0xff, 0xff, 0xfc, 0x00, 0x00, 0x1f, 0xff, 0xf1, 0xff, 0xff, 0xf8, 0x00,
    0x00, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xf0, 0x00, 0x00, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xf0, 0x00,
    0x00, 0x07, 0xff, 0xff, 0xff, 0xff, 0xe0, 0x00, 0x00, 0x03, 0xff, 0xff, 0xff, 0xff, 0xc0, 0x00,
    0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x7f, 0xff, 0xff, 0xfe, 0x00, 0x00,
    0x00, 0x00, 0x3f, 0xff, 0xff, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xff, 0xff, 0xf0, 0x00, 0x00,
    0x00, 0x00, 0x01, 0xff, 0xff, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3f, 0xfc, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Draw a QR code matrix onto any DrawTarget at the given position and scale.
fn draw_qr<D: DrawTarget<Color = Color>>(
    display: &mut D,
    matrix: &[Vec<bool>],
    start_x: i32,
    start_y: i32,
    module_size: i32,
) -> Result<(), D::Error> {
    for (y, row) in matrix.iter().enumerate() {
        for (x, &dark) in row.iter().enumerate() {
            let color = if dark { Color::Black } else { Color::White };
            Rectangle::new(
                Point::new(
                    start_x + x as i32 * module_size,
                    start_y + y as i32 * module_size,
                ),
                Size::new(module_size as u32, module_size as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        }
    }
    Ok(())
}

/// Format a number with thousands separator (e.g., 890123 → "890,123")
fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// A single row of OrangeClock display data.
struct OrangeClockRow {
    item: OrangeClockItem,
    text: String,
}

/// Build structured rows for OrangeClock display.
fn orangeclock_rows(
    data: &MempoolData,
    items: &DisplayItems,
    currency: &PriceCurrency,
) -> Vec<OrangeClockRow> {
    let mut rows = Vec::with_capacity(7);

    if items.block_height {
        if let Some(h) = data.block_height {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::BlockHeight,
                text: format_thousands(h),
            });
        }
    }

    if items.price {
        let item = match currency {
            PriceCurrency::USD => OrangeClockItem::PriceUsd,
            PriceCurrency::EUR => OrangeClockItem::PriceEur,
        };
        if let Some(p) = currency.price_from(data) {
            rows.push(OrangeClockRow {
                item,
                text: format_thousands(p),
            });
        }
    }

    if items.moscow_time {
        if let Some(p) = currency.price_from(data) {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::MoscowTime,
                text: format_thousands(mempool::moscow_time(p)),
            });
        }
    }

    if items.fees {
        if let (Some(h), Some(m), Some(l)) = (data.fee_fastest, data.fee_half_hour, data.fee_hour) {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::Fees,
                text: format!("{}/{}/{}", h, m, l),
            });
        }
    }

    if items.halving_countdown {
        if let Some(h) = data.block_height {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::HalvingCountdown,
                text: format!("~{}", format_thousands(mempool::blocks_until_halving(h))),
            });
        }
    }

    if items.difficulty_adjustment {
        if let (Some(progress), Some(change)) = (data.difficulty_progress, data.difficulty_change) {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::DifficultyAdjustment,
                text: format!("Diff {:.1}% ({:+.1}%)", progress, change),
            });
        }
    }

    if items.mempool_size {
        if let Some(count) = data.mempool_count {
            rows.push(OrangeClockRow {
                item: OrangeClockItem::MempoolSize,
                text: format!("Mempool {} txs", format_thousands(count)),
            });
        }
    }

    rows
}

/// Draw OrangeClock rows centered on screen with icons and text.
fn draw_orangeclock_layout<D: DrawTarget<Color = Color>>(
    display: &mut D,
    rows: &[OrangeClockRow],
    screen_width: i32,
    screen_height: i32,
    icon_size: IconSize,
    font: &MonoFont<'_>,
    max_rows: usize,
) -> Result<(), D::Error> {
    let rows = &rows[..rows.len().min(max_rows)];
    if rows.is_empty() {
        return Ok(());
    }

    let text_style = MonoTextStyle::new(font, Color::Black);
    let char_width = font.character_size.width as i32;
    let char_height = font.character_size.height as i32;
    let icon_gap = 6i32;
    let icon_h = icon_size.height();
    let row_height = icon_h.max(char_height) + 10;

    let total_height = rows.len() as i32 * row_height;
    let start_y = (screen_height - total_height) / 2;

    for (i, row) in rows.iter().enumerate() {
        let row_y = start_y + i as i32 * row_height;
        let text_width = row.text.chars().count() as i32 * char_width;

        if let Some(icon) = orangeclock_icons::get_icon(row.item, icon_size) {
            let total_width = icon.width as i32 + icon_gap + text_width;
            let x = (screen_width - total_width) / 2;

            let icon_raw: ImageRaw<Color> = ImageRaw::new(icon.data, icon.width);
            Image::new(&icon_raw, Point::new(x, row_y)).draw(display)?;

            let text_y = row_y + (icon.height as i32 + char_height) / 2;
            Text::new(
                &row.text,
                Point::new(x + icon.width as i32 + icon_gap, text_y),
                text_style,
            )
            .draw(display)?;
        } else {
            let x = (screen_width - text_width) / 2;
            let text_y = row_y + (icon_h + char_height) / 2;
            Text::new(&row.text, Point::new(x, text_y), text_style).draw(display)?;
        }
    }
    Ok(())
}

pub trait AtmDisplay<SPI, DELAY>
where
    SPI: embedded_hal::spi::SpiDevice,
    DELAY: embedded_hal::delay::DelayNs,
{
    fn home_screen(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn show_inserted_amount(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        amount_string: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn show_qr(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        qr_content: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn clean(&mut self, spi: &mut SPI, delay: &mut DELAY)
        -> Result<(), Box<dyn std::error::Error>>;
    fn show_orangeclock(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        data: &MempoolData,
        items: &DisplayItems,
        currency: &PriceCurrency,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

// ================= 1.54 Inch =================

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

// ================= 2.7 Inch (Tri-Color B/W/R used as B/W) =================

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

// ================= 2.7 Inch B/W Monochrome (V1 + V2) =================
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

// ================= 2.13 Inch (V2) =================

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
