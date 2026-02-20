use thiserror::Error;

/// PNG magic bytes
const PNG_HEADER: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug, Clone)]
pub struct PngInfo {
    pub width: u32,
    pub height: u32,
    pub size: usize,
}

#[derive(Debug, Error)]
pub enum PngError {
    #[error("Invalid PNG header - data may be corrupted or not a PNG")]
    InvalidHeader,

    #[error("PNG data too small ({0} bytes) - capture may have failed")]
    TooSmall(usize),

    #[error("PNG missing IEND chunk - file is truncated")]
    MissingIEND,

    #[error("Failed to parse IHDR chunk: {0}")]
    InvalidIHDR(String),

    #[error("PNG data starts with text, not binary - likely ADB warning message: {0}")]
    TextPrefix(String),
}

/// Validate PNG data and extract info
pub fn validate_png(data: &[u8]) -> Result<PngInfo, PngError> {
    // Check for text prefix (common with ADB warnings)
    if !data.is_empty() && data[0].is_ascii() && data[0] != 0x89 {
        let text_preview: String = data
            .iter()
            .take(100)
            .take_while(|&&b| b != 0x89)
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' })
            .collect();
        return Err(PngError::TextPrefix(text_preview));
    }

    if data.len() < 8 {
        return Err(PngError::TooSmall(data.len()));
    }

    if data[0..8] != PNG_HEADER {
        return Err(PngError::InvalidHeader);
    }

    if data.len() < 1000 {
        return Err(PngError::TooSmall(data.len()));
    }

    let has_iend = data
        .windows(4)
        .rev()
        .take(20)
        .any(|w| w == b"IEND");

    if !has_iend {
        return Err(PngError::MissingIEND);
    }

    let (width, height) = parse_ihdr(data)?;

    Ok(PngInfo {
        width,
        height,
        size: data.len(),
    })
}

fn parse_ihdr(data: &[u8]) -> Result<(u32, u32), PngError> {
    if data.len() < 24 {
        return Err(PngError::InvalidIHDR("Data too short".into()));
    }

    if &data[12..16] != b"IHDR" {
        return Err(PngError::InvalidIHDR("IHDR chunk not found at expected position".into()));
    }

    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

    if width == 0 || height == 0 || width > 10000 || height > 20000 {
        return Err(PngError::InvalidIHDR(format!(
            "Invalid dimensions: {}x{}",
            width, height
        )));
    }

    Ok((width, height))
}

/// Strip any text prefix from PNG data (e.g., ADB warnings)
pub fn strip_text_prefix(data: &[u8]) -> Option<&[u8]> {
    data.windows(8)
        .position(|w| w == PNG_HEADER)
        .map(|pos| &data[pos..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_png_header() {
        let mut data = Vec::new();
        data.extend_from_slice(&PNG_HEADER);
        data.extend_from_slice(&[0, 0, 0, 13]); // length
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&[0, 0, 1, 0]); // width = 256
        data.extend_from_slice(&[0, 0, 2, 0]); // height = 512
        data.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth, color type, etc.
        data.extend_from_slice(&[0, 0, 0, 0]); // CRC (fake)
        data.resize(2000, 0);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"IEND");
        data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);

        let result = validate_png(&data);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.width, 256);
        assert_eq!(info.height, 512);
    }

    #[test]
    fn test_text_prefix_detection() {
        let data = b"WARNING: some adb message\x89PNG\r\n\x1a\n...";
        let result = validate_png(data);
        assert!(matches!(result, Err(PngError::TextPrefix(_))));
    }

    #[test]
    fn test_strip_text_prefix() {
        let data = b"WARNING: message\x89PNG\r\n\x1a\nrest".to_vec();
        let stripped = strip_text_prefix(&data);
        assert!(stripped.is_some());
        assert_eq!(&stripped.unwrap()[0..8], PNG_HEADER);
    }
}
