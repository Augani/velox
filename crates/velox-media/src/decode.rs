use std::path::Path;

use image::ImageReader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
    Rgb8,
}

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub data: Vec<u8>,
}

pub fn decode_from_bytes(data: &[u8]) -> Result<DecodedImage, image::ImageError> {
    let cursor = std::io::Cursor::new(data);
    let reader = ImageReader::new(cursor).with_guessed_format()?;
    let img = reader.decode()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        width,
        height,
        format: PixelFormat::Rgba8,
        data: rgba.into_raw(),
    })
}

pub fn decode_from_path(path: impl AsRef<Path>) -> Result<DecodedImage, image::ImageError> {
    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        width,
        height,
        format: PixelFormat::Rgba8,
        data: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    #[test]
    fn decode_png_from_bytes() {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        let rgba_data = vec![255u8; 4 * 4 * 4];
        encoder
            .write_image(&rgba_data, 4, 4, image::ExtendedColorType::Rgba8)
            .unwrap();

        let decoded = decode_from_bytes(&buf).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.format, PixelFormat::Rgba8);
        assert_eq!(decoded.data.len(), 4 * 4 * 4);
    }
}
