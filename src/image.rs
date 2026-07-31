//Engine-Side Image Decoder & Resource Pipeline 

use std::collections::HashMap;
use crate::paint::DisplayCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Bmp,
    Gif,
    WebP,
    Tiff,
    Svg,
}

#[derive(Debug, Clone)]
pub struct DecodeImage {
    pub id: String,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub fn DetectImageFormat(data: &[u8]) -> Option<ImageFormat> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(ImageFormat::Png)
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if data.starts_with(&[b'B', b'M']) {
        Some(ImageFormat::Bmp)
    } else if data.starts_with(&[b'G', b'I', b'F']) {
        Some(ImageFormat::Gif)
    } else if data.starts_with(&[b'R', b'I', b'F', b'F']) && data.len() > 8 && &data[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else if data.starts_with(&[b'I', b'I', 0x2A, 0x00]) || data.starts_with(&[b'M', b'M', 0x00, 0x2A]) {
        Some(ImageFormat::Tiff)
    } else if data.starts_with(b"<?xml") || data.windows(5).any(|window| window == b"<svg ") {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}

pub struct ImageDecoder;

impl ImageDecoder {
    pub fn new() -> Self {
        Self
    }

    pub fn decode_image(&self, id: &str, data: &[u8]) -> Result<DecodeImage, String> {
        // Implement the image decoding logic here
        // For example, you can use the `image` crate to decode the image data
        // and return a DecodeImage struct with the decoded information.
        let format = DetectImageFormat(data).ok_or_else(|| format!("Unknown image format for '{}'", id))?;
        let (width, height) = parse_image_dimensions(format, data);

        Ok(DecodeImage {
            id: id.to_string(),
            format,
            width,
            height,
            data: data.to_vec(),
        })
    }
}

/// ImageCache caching boundary for ResourceLoader
/// Flow: ResourceLoader -> ImageCache -> Already decoded? (Yes -> return, No -> decode & cache)
pub struct ImageCache {
    cache: HashMap<String, DecodeImage>,
    decoder: ImageDecoder,
}

impl ImageCache {
    pub fn new() -> Self {
        ImageCache {
            cache: HashMap::new(),
            decoder: ImageDecoder::new(),
        }
    }

    pub fn get_or_decode(&mut self, id: &str, data: &[u8]) -> Result<DecodeImage, String> {
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached.clone());
        }
        let decoded = self.decoder.decode_image(id, data)?;
        self.cache.insert(id.to_string(), decoded.clone());
        Ok(decoded)
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Lazy decoding (Pillar 7): Only decode if the image's layout box
    /// is within the visible viewport. Off-screen images stay compressed
    /// to save CPU and memory.
    ///
    /// 100 images on page → Visible? → Yes → Decode | No → Keep compressed
    pub fn get_or_decode_if_visible(
        &mut self,
        id: &str,
        data: &[u8],
        image_rect: &crate::layout::Rect,
        viewport: &crate::layout::Rect,
    ) -> Option<DecodeImage> {
        // Visibility check: does the image box intersect the viewport?
        if !rects_intersect(image_rect, viewport) {
            return None; // Keep compressed — don't waste CPU/memory
        }
        // Visible → decode (or return cached)
        self.get_or_decode(id, data).ok()
    }
}

/// Check if two rectangles intersect (used for viewport visibility testing)
fn rects_intersect(a: &crate::layout::Rect, b: &crate::layout::Rect) -> bool {
    a.x < b.x + b.width
        && a.x + a.width > b.x
        && a.y < b.y + b.height
        && a.y + a.height > b.y
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract width and height from PNG, JPEG, GIF, BMP headers (with safe fallbacks)
fn parse_image_dimensions(format: ImageFormat, data: &[u8]) -> (u32, u32) {
    match format {
        ImageFormat::Png if data.len() >= 24 => {
            let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            (width.max(1), height.max(1))
        }
        ImageFormat::Bmp if data.len() >= 26 => {
            let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
            let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
            (width.max(1), height.max(1))
        }
        ImageFormat::Gif if data.len() >= 10 => {
            let width = u16::from_le_bytes([data[6], data[7]]) as u32;
            let height = u16::from_le_bytes([data[8], data[9]]) as u32;
            (width.max(1), height.max(1))
        }
        _ => (300, 150), // Fallback default image box dimensions per CSS spec
    }
}

pub fn DecodedImageToDisplayCommand(decoded_image: &DecodeImage, x: f32, y: f32) -> DisplayCommand {
    DisplayCommand::Image {
        image_id: decoded_image.id.clone(),
        x,
        y,
        width: decoded_image.width as f32,
        height: decoded_image.height as f32,
    }
}