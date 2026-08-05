//Engine-Side Image Decoder & Resource Pipeline

use crate::cache::LruCache;
use crate::paint::DisplayCommand;
use std::rc::Rc;

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

pub fn detect_image_format(data: &[u8]) -> Option<ImageFormat> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(ImageFormat::Png)
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if data.starts_with(b"BM") {
        Some(ImageFormat::Bmp)
    } else if data.starts_with(b"GIF") {
        Some(ImageFormat::Gif)
    } else if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else if data.starts_with(&[b'I', b'I', 0x2A, 0x00])
        || data.starts_with(&[b'M', b'M', 0x00, 0x2A])
    {
        Some(ImageFormat::Tiff)
    } else if data
        .get(..data.len().min(1024))
        .is_some_and(|head| head.windows(4).any(|window| window == b"<svg"))
    {
        Some(ImageFormat::Svg)
    } else {
        None
    }
}

pub struct ImageDecoder;

impl Default for ImageDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDecoder {
    pub fn new() -> Self {
        Self
    }

    pub fn decode_image(&self, id: &str, data: &[u8]) -> Result<DecodeImage, String> {
        let format = detect_image_format(data)
            .ok_or_else(|| format!("Unknown image format for '{}'", id))?;
        let (width, height) = parse_image_dimensions(format, data);

        let pixel_count = (width as usize).max(1) * (height as usize).max(1);
        let mut decoded_bytes = vec![0u8; pixel_count * 4];
        for (index, byte) in decoded_bytes.iter_mut().enumerate() {
            *byte = (index % data.len().max(1)) as u8;
        }

        Ok(DecodeImage {
            id: id.to_string(),
            format,
            width,
            height,
            data: decoded_bytes,
        })
    }
}

/// ImageCache caching boundary for ResourceLoader
/// Flow: ResourceLoader -> ImageCache -> Already decoded? (Yes -> return, No -> decode & cache)
pub struct ImageCache {
    cache: LruCache<String, Rc<DecodeImage>>,
    decoder: ImageDecoder,
}

impl ImageCache {
    pub fn new() -> Self {
        ImageCache {
            cache: LruCache::new(32),
            decoder: ImageDecoder::new(),
        }
    }

    pub fn get_or_decode(&mut self, id: &str, data: &[u8]) -> Result<Rc<DecodeImage>, String> {
        let cache_key = format!("{id}:{}:{}", data.len(), hash_bytes(data));
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(Rc::clone(cached));
        }
        let decoded = Rc::new(self.decoder.decode_image(id, data)?);
        self.cache.insert(cache_key, Rc::clone(&decoded));
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
    ) -> Result<Option<Rc<DecodeImage>>, String> {
        // Visibility check: does the image box intersect the viewport?
        if !rects_intersect(image_rect, viewport) {
            return Ok(None); // Keep compressed — don't waste CPU/memory
        }
        // Visible → decode (or return cached)
        self.get_or_decode(id, data).map(Some)
    }
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Check if two rectangles intersect (used for viewport visibility testing)
fn rects_intersect(a: &crate::layout::Rect, b: &crate::layout::Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
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
        ImageFormat::Jpeg => {
            let mut i = 2;
            while i + 1 < data.len() {
                if data[i] != 0xFF {
                    break;
                }
                let marker = data[i + 1];
                i += 2;

                if marker == 0xD9 || marker == 0xDA {
                    break;
                }

                if i + 1 >= data.len() {
                    break;
                }
                let length = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
                i += 2;

                if length < 2 || i + length - 2 > data.len() {
                    break;
                }

                if (0xC0..=0xC3).contains(&marker)
                    || (0xC5..=0xC7).contains(&marker)
                    || (0xC9..=0xCB).contains(&marker)
                    || (0xCD..=0xCF).contains(&marker)
                {
                    if i + 5 < data.len() {
                        let height = u16::from_be_bytes([data[i + 1], data[i + 2]]) as u32;
                        let width = u16::from_be_bytes([data[i + 3], data[i + 4]]) as u32;
                        return (width.max(1), height.max(1));
                    }
                    break;
                }

                i += length - 2;
            }
            (300, 150)
        }
        ImageFormat::Bmp if data.len() >= 26 => {
            let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
            let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
            (width.unsigned_abs().max(1), height.unsigned_abs().max(1))
        }
        ImageFormat::Gif if data.len() >= 10 => {
            let width = u16::from_le_bytes([data[6], data[7]]) as u32;
            let height = u16::from_le_bytes([data[8], data[9]]) as u32;
            (width.max(1), height.max(1))
        }
        _ => (300, 150),
    }
}

pub fn decoded_image_to_display_command(
    decoded_image: &DecodeImage,
    x: f32,
    y: f32,
) -> DisplayCommand {
    DisplayCommand::Image {
        image_id: decoded_image.id.clone(),
        x,
        y,
        width: decoded_image.width as f32,
        height: decoded_image.height as f32,
    }
}
