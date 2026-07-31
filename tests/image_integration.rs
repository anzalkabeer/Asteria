// ─── Image Pipeline Integration Test ─────────────────────────────
//
// Tests format detection, header parsing, ImageCache boundary (hit vs miss),
// and DecodedImageToDisplayCommand integration.

use asteria::image::{
    DecodedImageToDisplayCommand, DetectImageFormat, DecodeImage, ImageCache, ImageFormat,
};
use asteria::paint::DisplayCommand;

#[test]
fn test_detect_image_format() {
    let png_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    assert_eq!(DetectImageFormat(&png_bytes), Some(ImageFormat::Png));

    let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0];
    assert_eq!(DetectImageFormat(&jpeg_bytes), Some(ImageFormat::Jpeg));

    let bmp_bytes = [b'B', b'M', 0, 0];
    assert_eq!(DetectImageFormat(&bmp_bytes), Some(ImageFormat::Bmp));
}

#[test]
fn test_image_cache_boundary_hit_and_miss() {
    let mut cache = ImageCache::new();
    let fake_png = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 100, // width: 100
        0, 0, 0, 50,  // height: 50
    ];

    // Cache miss -> decode and store
    let img1 = cache.get_or_decode("logo.png", &fake_png).unwrap();
    assert_eq!(img1.width, 100);
    assert_eq!(img1.height, 50);
    assert_eq!(cache.len(), 1);

    // Cache hit -> return cached without decoding again
    let img2 = cache.get_or_decode("logo.png", &fake_png).unwrap();
    assert_eq!(img2.width, 100);
    assert_eq!(cache.len(), 1); // no extra cache entry
}

#[test]
fn test_decoded_image_to_display_command() {
    let img = DecodeImage {
        id: "hero.png".to_string(),
        format: ImageFormat::Png,
        width: 200,
        height: 100,
        data: vec![],
    };

    let cmd = DecodedImageToDisplayCommand(&img, 10.0, 20.0);
    if let DisplayCommand::Image { image_id, x, y, width, height } = cmd {
        assert_eq!(image_id, "hero.png");
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        assert_eq!(width, 200.0);
        assert_eq!(height, 100.0);
    } else {
        panic!("Expected DisplayCommand::Image");
    }
}
