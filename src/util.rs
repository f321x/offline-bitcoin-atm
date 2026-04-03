use qrcode::{EcLevel, QrCode, Version};

#[derive(Debug)]
pub struct LNBitsConnection {
    pub base_url: String,
    pub page_base_url: String,
    pub atm_secret: String,
    pub currency: String,
}

impl LNBitsConnection {
    /// split lnbits device string into base_url, secret and currency
    /// e.g. https://XXXX.lnbits.com/fossa/api/v1/lnurl/XXXXX,XXXXXXXXXXXXXXXXXXXXXX,USD
    pub fn from_device_string(device_string: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = device_string.splitn(3, ',').collect();

        if parts.len() != 3 {
            return Err("Device string must contain exactly 3 comma-separated parts: base_url,secret,currency");
        }

        let base_url = parts[0].trim();
        let atm_secret = parts[1].trim();
        let currency = parts[2].trim();

        if base_url.is_empty() {
            return Err("Base URL cannot be empty");
        }

        if atm_secret.is_empty() {
            return Err("ATM secret cannot be empty");
        }

        if currency.is_empty() {
            return Err("Currency cannot be empty");
        }

        let api_pos = base_url.find("api").ok_or("'api' not found in base URL")?;
        let atm_page_base_url = format!("{}atm?lightning=", &base_url[..api_pos]);

        Ok(Self {
            base_url: base_url.to_string(),
            page_base_url: atm_page_base_url.to_string(),
            atm_secret: atm_secret.to_string(),
            currency: currency.to_string(),
        })
    }
}

/// Generates a QR code from the given data string
/// Returns a 2D vector of booleans where true = black pixel, false = white pixel
pub fn generate_qrcode(data: &str) -> Result<Vec<Vec<bool>>, qrcode::types::QrError> {
    let code = QrCode::with_version(data.as_bytes(), Version::Normal(6), EcLevel::L)?;

    // Get the QR code as a matrix
    let width = code.width();
    let mut matrix = Vec::with_capacity(width);

    for y in 0..width {
        let mut row = Vec::with_capacity(width);
        for x in 0..width {
            // true for dark modules (black), false for light modules (white)
            row.push(code[(x, y)] != qrcode::Color::Light);
        }
        matrix.push(row);
    }

    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- QR code generation tests ---

    #[test]
    fn test_generate_qrcode_returns_square_matrix() {
        let matrix = generate_qrcode("LNURL1TEST").unwrap();
        assert!(!matrix.is_empty());
        // QR version 6 = 41x41 modules
        assert_eq!(matrix.len(), 41);
        for row in &matrix {
            assert_eq!(row.len(), 41);
        }
    }

    #[test]
    fn test_generate_qrcode_contains_both_colors() {
        let matrix = generate_qrcode("LNURL1TEST").unwrap();
        let has_dark = matrix.iter().any(|row| row.iter().any(|&v| v));
        let has_light = matrix.iter().any(|row| row.iter().any(|&v| !v));
        assert!(has_dark, "QR matrix should contain dark modules");
        assert!(has_light, "QR matrix should contain light modules");
    }

    #[test]
    fn test_generate_qrcode_deterministic() {
        let m1 = generate_qrcode("LNURL1SAME").unwrap();
        let m2 = generate_qrcode("LNURL1SAME").unwrap();
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_generate_qrcode_different_input_different_output() {
        let m1 = generate_qrcode("LNURL1AAA").unwrap();
        let m2 = generate_qrcode("LNURL1BBB").unwrap();
        assert_ne!(m1, m2);
    }

    // --- LNBitsConnection parsing tests ---

    #[test]
    fn test_from_device_string_valid() {
        let conn = LNBitsConnection::from_device_string(
            "https://example.com/fossa/api/v1/lnurl/abc123,mysecretkey,EUR",
        )
        .unwrap();
        assert_eq!(
            conn.base_url,
            "https://example.com/fossa/api/v1/lnurl/abc123"
        );
        assert_eq!(conn.atm_secret, "mysecretkey");
        assert_eq!(conn.currency, "EUR");
        assert_eq!(
            conn.page_base_url,
            "https://example.com/fossa/atm?lightning="
        );
    }

    #[test]
    fn test_from_device_string_missing_parts() {
        assert!(LNBitsConnection::from_device_string("onlyonepart").is_err());
        assert!(LNBitsConnection::from_device_string("two,parts").is_err());
    }

    #[test]
    fn test_from_device_string_empty_url() {
        assert!(LNBitsConnection::from_device_string(",secret,EUR").is_err());
    }

    #[test]
    fn test_from_device_string_empty_secret() {
        assert!(LNBitsConnection::from_device_string("https://example.com/api/v1,,EUR").is_err());
    }

    #[test]
    fn test_from_device_string_empty_currency() {
        assert!(
            LNBitsConnection::from_device_string("https://example.com/api/v1,secret,").is_err()
        );
    }

    #[test]
    fn test_from_device_string_no_api_in_url() {
        assert!(
            LNBitsConnection::from_device_string("https://example.com/v1/lnurl,secret,EUR")
                .is_err()
        );
    }

    #[test]
    fn test_from_device_string_trims_whitespace() {
        let conn = LNBitsConnection::from_device_string(
            " https://example.com/api/v1/lnurl , secret , USD ",
        )
        .unwrap();
        assert_eq!(conn.base_url, "https://example.com/api/v1/lnurl");
        assert_eq!(conn.atm_secret, "secret");
        assert_eq!(conn.currency, "USD");
    }
}
