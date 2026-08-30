#![no_main]

use libfuzzer_sys::fuzz_target;
use nuif_font::inspect_opentype_static;
use nuif_media::{decode_png_rgba8, inspect_png_rgba8};

fuzz_target!(|input: &[u8]| {
    let Some((&selector, bytes)) = input.split_first() else {
        return;
    };
    match selector & 1 {
        0 => {
            if let Ok(image) = decode_png_rgba8(bytes) {
                let header = inspect_png_rgba8(bytes).expect("decoded PNG must inspect");
                assert_eq!(image.width, header.width);
                assert_eq!(image.height, header.height);
                assert_eq!(image.rgba.len(), header.decoded_bytes);
            }
        }
        _ => {
            if let Ok(inspection) = inspect_opentype_static(bytes, 0) {
                assert_eq!(inspection.byte_length, bytes.len());
                assert!(inspection.glyph_count > 0);
                assert!(!inspection.coverage.is_empty());
            }
        }
    }
});
