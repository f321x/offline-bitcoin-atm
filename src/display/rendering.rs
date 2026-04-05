use embedded_graphics::{
    geometry::Size,
    image::{Image, ImageRaw},
    mono_font::{MonoFont, MonoTextStyle},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use epd_waveshare::prelude::*;

use crate::mempool::{self, DisplayItems, MempoolData, PriceCurrency};
use crate::orangeclock_icons::{self, IconSize, OrangeClockItem};

pub(super) fn draw_qr<D: DrawTarget<Color = Color>>(
    display: &mut D,
    matrix: &[Vec<bool>],
    start_x: i32,
    start_y: i32,
    module_size: i32,
) -> Result<(), D::Error> {
    let black_style = PrimitiveStyle::with_fill(Color::Black);
    let white_style = PrimitiveStyle::with_fill(Color::White);
    for (y, row) in matrix.iter().enumerate() {
        for (x, &dark) in row.iter().enumerate() {
            let style = if dark { &black_style } else { &white_style };
            Rectangle::new(
                Point::new(
                    start_x + x as i32 * module_size,
                    start_y + y as i32 * module_size,
                ),
                Size::new(module_size as u32, module_size as u32),
            )
            .into_styled(*style)
            .draw(display)?;
        }
    }
    Ok(())
}

pub(super) fn format_thousands(n: u64) -> String {
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

pub(super) struct OrangeClockRow {
    pub(super) item: OrangeClockItem,
    pub(super) text: String,
}

/// Build structured rows for OrangeClock display.
pub(super) fn orangeclock_rows(
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

    let price = currency.price_from(data);

    if items.price {
        let item = match currency {
            PriceCurrency::USD => OrangeClockItem::PriceUsd,
            PriceCurrency::EUR => OrangeClockItem::PriceEur,
        };
        if let Some(p) = price {
            rows.push(OrangeClockRow {
                item,
                text: format_thousands(p),
            });
        }
    }

    if items.moscow_time {
        if let Some(p) = price {
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
pub(super) fn draw_orangeclock_layout<D: DrawTarget<Color = Color>>(
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
