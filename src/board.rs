use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardType {
    Generic,
    Waveshare,
}

impl BoardType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Waveshare" => BoardType::Waveshare,
            _ => BoardType::Generic,
        }
    }
}

pub struct BoardPins<'a> {
    pub coin: AnyIOPin<'a>,
    pub mosfet: AnyIOPin<'a>,
    pub button: AnyIOPin<'a>,
    pub button_led: AnyIOPin<'a>,
    // SPI Pins
    pub sclk: AnyIOPin<'a>,
    pub mosi: AnyIOPin<'a>,
    pub cs: AnyIOPin<'a>,
    // E-Ink Control
    pub dc: AnyIOPin<'a>,
    pub rst: AnyIOPin<'a>,
    pub busy: AnyIOPin<'a>,
}

impl BoardPins<'_> {
    pub fn new(board: BoardType, _pins: &mut Peripherals) -> Result<Self, String> {
        // Using unsafe to create AnyIOPin from IDs.
        // This avoids moving Peripherals and allows dynamic selection.

        match board {
            BoardType::Generic => Ok(BoardPins {
                coin: unsafe { AnyIOPin::steal(17) },
                mosfet: unsafe { AnyIOPin::steal(16) },
                button: unsafe { AnyIOPin::steal(32) },
                button_led: unsafe { AnyIOPin::steal(21) },
                sclk: unsafe { AnyIOPin::steal(18) },
                mosi: unsafe { AnyIOPin::steal(23) },
                cs: unsafe { AnyIOPin::steal(26) },
                dc: unsafe { AnyIOPin::steal(25) },
                rst: unsafe { AnyIOPin::steal(33) },
                busy: unsafe { AnyIOPin::steal(27) },
            }),
            BoardType::Waveshare => Ok(BoardPins {
                // Coin/Mosfet/Btn same as Generic for now
                coin: unsafe { AnyIOPin::steal(17) },
                mosfet: unsafe { AnyIOPin::steal(16) },
                button: unsafe { AnyIOPin::steal(32) },
                button_led: unsafe { AnyIOPin::steal(21) },
                // Waveshare SPI/Control
                sclk: unsafe { AnyIOPin::steal(13) },
                mosi: unsafe { AnyIOPin::steal(14) },
                cs: unsafe { AnyIOPin::steal(15) },
                dc: unsafe { AnyIOPin::steal(27) },
                rst: unsafe { AnyIOPin::steal(26) },
                busy: unsafe { AnyIOPin::steal(25) },
            }),
        }
    }
}
