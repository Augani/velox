use crate::decode::{DecodedImage, PixelFormat};

pub fn generate_thumbnail(source: &DecodedImage, max_size: u32) -> DecodedImage {
    if max_size == 0 {
        return DecodedImage {
            width: 0,
            height: 0,
            format: PixelFormat::Rgba8,
            data: Vec::new(),
        };
    }

    if source.width <= max_size && source.height <= max_size {
        return DecodedImage {
            width: source.width,
            height: source.height,
            format: source.format,
            data: source.data.clone(),
        };
    }

    let scale = if source.width >= source.height {
        max_size as f64 / source.width as f64
    } else {
        max_size as f64 / source.height as f64
    };

    let new_width = (source.width as f64 * scale).round().max(1.0) as u32;
    let new_height = (source.height as f64 * scale).round().max(1.0) as u32;

    let Some(img) = image::RgbaImage::from_raw(source.width, source.height, source.data.clone())
    else {
        return DecodedImage {
            width: 0,
            height: 0,
            format: PixelFormat::Rgba8,
            data: Vec::new(),
        };
    };

    let resized = image::imageops::resize(
        &img,
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    );

    DecodedImage {
        width: new_width,
        height: new_height,
        format: PixelFormat::Rgba8,
        data: resized.into_raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_respects_max_size() {
        let source = DecodedImage {
            width: 200,
            height: 100,
            format: PixelFormat::Rgba8,
            data: vec![128u8; 200 * 100 * 4],
        };

        let thumb = generate_thumbnail(&source, 50);
        assert!(thumb.width <= 50);
        assert!(thumb.height <= 50);
        assert!(thumb.width > 0);
        assert!(thumb.height > 0);
    }

    #[test]
    fn thumbnail_no_upscale() {
        let source = DecodedImage {
            width: 30,
            height: 20,
            format: PixelFormat::Rgba8,
            data: vec![255u8; 30 * 20 * 4],
        };

        let thumb = generate_thumbnail(&source, 100);
        assert_eq!(thumb.width, 30);
        assert_eq!(thumb.height, 20);
    }

    #[test]
    fn thumbnail_tall_image() {
        let source = DecodedImage {
            width: 100,
            height: 400,
            format: PixelFormat::Rgba8,
            data: vec![0u8; 100 * 400 * 4],
        };

        let thumb = generate_thumbnail(&source, 64);
        assert!(thumb.width <= 64);
        assert!(thumb.height <= 64);
    }
}
