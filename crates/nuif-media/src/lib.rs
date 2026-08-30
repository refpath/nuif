#![doc = "Bounded media decoding profiles shared by capture and rendering."]

use png::{BitDepth, ColorType, Transformations};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

pub const PNG_RGBA8_PROFILE: &str = "nuif-png-rgba8-0";
pub const PNG_BASIC_RGBA8_PROFILE: &str = "nuif-png-basic-rgba8-1";
pub const MAX_PNG_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PNG_WIDTH: u32 = 8_192;
pub const MAX_PNG_HEIGHT: u32 = 8_192;
pub const MAX_PNG_PIXELS: u64 = 16_777_216;
pub const MAX_PNG_CHUNKS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgba8Image {
    pub width: u32,
    pub height: u32,
    #[serde(with = "serde_bytes")]
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum MediaError {
    #[error("PNG does not satisfy the selected NUIF decoder profile: {0}")]
    UnsupportedPng(&'static str),
    #[error("PNG resource limit exceeded: {0}")]
    ResourceLimit(&'static str),
    #[error("PNG decoder rejected the datastream: {0}")]
    Decode(String),
}

/// Decodes the deliberately narrow straight-alpha RGBA8 PNG profile.
///
/// The profile accepts one non-interlaced 8-bit truecolor-with-alpha image,
/// one or more contiguous IDAT chunks and at most one pre-IDAT sRGB chunk.
/// No implicit palette, grayscale, transparency, ICC, gamma, Exif, animation,
/// orientation or color conversion is performed. Original bytes remain the
/// authoritative resource; this result is a derived encoded-sRGB pixel cache.
///
/// # Errors
///
/// Rejects malformed, ambiguous, unsupported or resource-exceeding inputs.
pub fn decode_png_rgba8(bytes: &[u8]) -> Result<Rgba8Image, MediaError> {
    let header = inspect_png_rgba8(bytes)?;
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::IDENTITY);
    let mut reader = decoder
        .read_info()
        .map_err(|error| MediaError::Decode(error.to_string()))?;
    let info = reader.info();
    if info.width != header.width
        || info.height != header.height
        || info.color_type != ColorType::Rgba
        || info.bit_depth != BitDepth::Eight
        || info.interlaced
    {
        return Err(MediaError::UnsupportedPng("decoded header changed profile"));
    }
    let buffer_size = reader
        .output_buffer_size()
        .ok_or(MediaError::ResourceLimit("decoded byte count"))?;
    if buffer_size != header.decoded_bytes {
        return Err(MediaError::UnsupportedPng(
            "decoded byte count differs from RGBA8",
        ));
    }
    let mut rgba = vec![0; buffer_size];
    let output = reader
        .next_frame(&mut rgba)
        .map_err(|error| MediaError::Decode(error.to_string()))?;
    if output.buffer_size() != header.decoded_bytes {
        return Err(MediaError::UnsupportedPng(
            "decoder emitted a partial or converted frame",
        ));
    }
    rgba.truncate(output.buffer_size());
    Ok(Rgba8Image {
        width: header.width,
        height: header.height,
        rgba,
    })
}

/// Decodes the bounded basic PNG profile into straight-alpha encoded-sRGB
/// RGBA8 pixels.
///
/// Unlike [`decode_png_rgba8`], this profile admits the common PNG colour
/// types that can be expanded to RGBA8 without reducing sample precision:
/// 1/2/4/8-bit greyscale and indexed colour, plus 8-bit RGB, greyscale-alpha
/// and RGBA. Palette and colour-key transparency are expanded. The profile
/// still rejects 16-bit samples, interlace and colour-management metadata.
///
/// # Errors
///
/// Rejects malformed, ambiguous, unsupported or resource-exceeding inputs.
pub fn decode_png_basic_rgba8(bytes: &[u8]) -> Result<Rgba8Image, MediaError> {
    let header = inspect_png_basic_rgba8(bytes)?;
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|error| MediaError::Decode(error.to_string()))?;
    let info = reader.info();
    if info.width != header.width || info.height != header.height || info.interlaced {
        return Err(MediaError::UnsupportedPng("decoded header changed profile"));
    }
    let buffer_size = reader
        .output_buffer_size()
        .ok_or(MediaError::ResourceLimit("decoded byte count"))?;
    if buffer_size > header.decoded_bytes {
        return Err(MediaError::ResourceLimit("decoded byte count"));
    }
    let mut pixel_bytes = vec![0; buffer_size];
    let output = reader
        .next_frame(&mut pixel_bytes)
        .map_err(|error| MediaError::Decode(error.to_string()))?;
    pixel_bytes.truncate(output.buffer_size());
    let (color, depth) = reader.output_color_type();
    if depth != BitDepth::Eight {
        return Err(MediaError::UnsupportedPng(
            "normalization did not produce 8-bit samples",
        ));
    }
    let rgba = normalize_rgba8(color, &pixel_bytes, header.decoded_bytes)?;
    Ok(Rgba8Image {
        width: header.width,
        height: header.height,
        rgba,
    })
}

/// Dispatches one of the implemented PNG decoder profiles.
///
/// # Errors
///
/// Rejects unknown profile names and any input rejected by the selected
/// profile.
pub fn decode_png_profile(profile: &str, bytes: &[u8]) -> Result<Rgba8Image, MediaError> {
    match profile {
        PNG_RGBA8_PROFILE => decode_png_rgba8(bytes),
        PNG_BASIC_RGBA8_PROFILE => decode_png_basic_rgba8(bytes),
        _ => Err(MediaError::UnsupportedPng("unknown decoder profile")),
    }
}

/// Returns the exact RGBA8 cache size implied by a supported profile without
/// inflating image data.
///
/// # Errors
///
/// Rejects unknown profiles and inputs outside the selected structural
/// profile.
pub fn png_profile_decoded_bytes(profile: &str, bytes: &[u8]) -> Result<usize, MediaError> {
    match profile {
        PNG_RGBA8_PROFILE => inspect_png_rgba8(bytes).map(|header| header.decoded_bytes),
        PNG_BASIC_RGBA8_PROFILE => {
            inspect_png_basic_rgba8(bytes).map(|header| header.decoded_bytes)
        }
        _ => Err(MediaError::UnsupportedPng("unknown decoder profile")),
    }
}

fn normalize_rgba8(
    color: ColorType,
    decoded: &[u8],
    expected_rgba_bytes: usize,
) -> Result<Vec<u8>, MediaError> {
    let pixels = expected_rgba_bytes / 4;
    match color {
        ColorType::Rgba if decoded.len() == expected_rgba_bytes => Ok(decoded.to_vec()),
        ColorType::Rgb if decoded.len() == pixels.saturating_mul(3) => {
            let mut rgba = Vec::with_capacity(expected_rgba_bytes);
            for pixel in decoded.as_chunks::<3>().0 {
                rgba.extend_from_slice(pixel);
                rgba.push(255);
            }
            Ok(rgba)
        }
        ColorType::Grayscale if decoded.len() == pixels => {
            let mut rgba = Vec::with_capacity(expected_rgba_bytes);
            for value in decoded {
                rgba.extend([*value, *value, *value, 255]);
            }
            Ok(rgba)
        }
        ColorType::GrayscaleAlpha if decoded.len() == pixels.saturating_mul(2) => {
            let mut rgba = Vec::with_capacity(expected_rgba_bytes);
            for pixel in decoded.as_chunks::<2>().0 {
                rgba.extend([pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
            Ok(rgba)
        }
        _ => Err(MediaError::UnsupportedPng(
            "normalization did not produce RGB8 or RGBA8",
        )),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PngProfileHeader {
    pub width: u32,
    pub height: u32,
    pub decoded_bytes: usize,
    pub has_srgb: bool,
    pub chunks: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PngBasicProfileHeader {
    pub width: u32,
    pub height: u32,
    pub decoded_bytes: usize,
    pub color_type: ColorType,
    pub bit_depth: BitDepth,
    pub has_srgb: bool,
    pub has_transparency: bool,
    pub chunks: usize,
}

/// Validates the encoded structure and allocation bounds without inflating it.
/// CRC and DEFLATE integrity are then checked by the selected decoder.
///
/// # Errors
///
/// Rejects a non-profile chunk sequence, header or resource bound.
pub fn inspect_png_rgba8(bytes: &[u8]) -> Result<PngProfileHeader, MediaError> {
    if bytes.len() > MAX_PNG_BYTES {
        return Err(MediaError::ResourceLimit("encoded bytes"));
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(MediaError::UnsupportedPng("signature"));
    }
    let mut offset = 8_usize;
    let mut chunks = 0_usize;
    let mut header = None;
    let mut seen_idat = false;
    let mut idat_ended = false;
    let mut seen_iend = false;
    let mut seen_srgb = false;
    while offset < bytes.len() {
        chunks = chunks.saturating_add(1);
        if chunks > MAX_PNG_CHUNKS {
            return Err(MediaError::ResourceLimit("chunk count"));
        }
        let prefix_end = offset
            .checked_add(8)
            .ok_or(MediaError::ResourceLimit("chunk offset"))?;
        if prefix_end > bytes.len() {
            return Err(MediaError::UnsupportedPng("truncated chunk header"));
        }
        let length = read_u32(&bytes[offset..offset + 4])? as usize;
        let kind = &bytes[offset + 4..prefix_end];
        let end = prefix_end
            .checked_add(length)
            .and_then(|value| value.checked_add(4))
            .ok_or(MediaError::ResourceLimit("chunk length"))?;
        if end > bytes.len() {
            return Err(MediaError::UnsupportedPng("truncated chunk payload"));
        }
        let data = &bytes[prefix_end..prefix_end + length];
        match kind {
            b"IHDR" if chunks == 1 && header.is_none() && length == 13 => {
                header = Some(parse_header(data)?);
            }
            b"sRGB"
                if header.is_some() && !seen_idat && !seen_srgb && length == 1 && data[0] <= 3 =>
            {
                seen_srgb = true;
            }
            b"IDAT" if header.is_some() && !idat_ended && !seen_iend => {
                seen_idat = true;
            }
            b"IEND" if seen_idat && !seen_iend && length == 0 => {
                seen_iend = true;
            }
            _ => {
                return Err(MediaError::UnsupportedPng(
                    "chunk sequence or ancillary metadata",
                ));
            }
        }
        if seen_idat && kind != b"IDAT" {
            idat_ended = true;
        }
        offset = end;
        if seen_iend {
            break;
        }
    }
    let mut header = header.ok_or(MediaError::UnsupportedPng("missing IHDR"))?;
    if !seen_idat || !seen_iend || offset != bytes.len() {
        return Err(MediaError::UnsupportedPng(
            "missing IDAT/IEND or trailing bytes",
        ));
    }
    header.has_srgb = seen_srgb;
    header.chunks = chunks;
    Ok(header)
}

/// Validates the basic PNG profile structure without inflating image data.
/// CRC and DEFLATE integrity are checked by the selected decoder.
///
/// # Errors
///
/// Rejects a non-profile chunk sequence, header or resource bound.
#[expect(
    clippy::too_many_lines,
    reason = "the strict chunk-state machine is kept contiguous so its accepted grammar remains auditable"
)]
pub fn inspect_png_basic_rgba8(bytes: &[u8]) -> Result<PngBasicProfileHeader, MediaError> {
    if bytes.len() > MAX_PNG_BYTES {
        return Err(MediaError::ResourceLimit("encoded bytes"));
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(MediaError::UnsupportedPng("signature"));
    }
    let mut offset = 8_usize;
    let mut chunks = 0_usize;
    let mut header = None;
    let mut seen_srgb = false;
    let mut palette_entries = None;
    let mut seen_transparency = false;
    let mut seen_idat = false;
    let mut idat_ended = false;
    let mut seen_iend = false;
    while offset < bytes.len() {
        chunks = chunks.saturating_add(1);
        if chunks > MAX_PNG_CHUNKS {
            return Err(MediaError::ResourceLimit("chunk count"));
        }
        let prefix_end = offset
            .checked_add(8)
            .ok_or(MediaError::ResourceLimit("chunk offset"))?;
        if prefix_end > bytes.len() {
            return Err(MediaError::UnsupportedPng("truncated chunk header"));
        }
        let length = read_u32(&bytes[offset..offset + 4])? as usize;
        let kind = &bytes[offset + 4..prefix_end];
        let end = prefix_end
            .checked_add(length)
            .and_then(|value| value.checked_add(4))
            .ok_or(MediaError::ResourceLimit("chunk length"))?;
        if end > bytes.len() {
            return Err(MediaError::UnsupportedPng("truncated chunk payload"));
        }
        let data = &bytes[prefix_end..prefix_end + length];
        match kind {
            b"IHDR" if chunks == 1 && header.is_none() && length == 13 => {
                header = Some(parse_basic_header(data)?);
            }
            b"sRGB"
                if header.is_some()
                    && !seen_srgb
                    && palette_entries.is_none()
                    && !seen_transparency
                    && !seen_idat
                    && length == 1
                    && data[0] <= 3 =>
            {
                seen_srgb = true;
            }
            b"PLTE"
                if header.is_some()
                    && palette_entries.is_none()
                    && !seen_transparency
                    && !seen_idat
                    && length > 0
                    && length <= 768
                    && length.is_multiple_of(3) =>
            {
                let current = header.ok_or(MediaError::UnsupportedPng("missing IHDR"))?;
                if current.color_type != ColorType::Indexed {
                    return Err(MediaError::UnsupportedPng(
                        "suggested palettes are outside this profile",
                    ));
                }
                let entries = length / 3;
                if entries > 1_usize << current.bit_depth as u8 {
                    return Err(MediaError::UnsupportedPng("palette exceeds bit depth"));
                }
                palette_entries = Some(entries);
            }
            b"tRNS" if header.is_some() && !seen_transparency && !seen_idat && length > 0 => {
                let current = header.ok_or(MediaError::UnsupportedPng("missing IHDR"))?;
                let valid = match current.color_type {
                    ColorType::Grayscale => length == 2,
                    ColorType::Rgb => length == 6,
                    ColorType::Indexed => palette_entries.is_some_and(|entries| length <= entries),
                    ColorType::GrayscaleAlpha | ColorType::Rgba => false,
                };
                if !valid {
                    return Err(MediaError::UnsupportedPng("invalid transparency chunk"));
                }
                seen_transparency = true;
            }
            b"IDAT" if header.is_some() && !idat_ended && !seen_iend => {
                let current = header.ok_or(MediaError::UnsupportedPng("missing IHDR"))?;
                if current.color_type == ColorType::Indexed && palette_entries.is_none() {
                    return Err(MediaError::UnsupportedPng("indexed image lacks palette"));
                }
                seen_idat = true;
            }
            b"IEND" if seen_idat && !seen_iend && length == 0 => {
                seen_iend = true;
            }
            _ => {
                return Err(MediaError::UnsupportedPng(
                    "chunk sequence or ancillary metadata",
                ));
            }
        }
        if seen_idat && kind != b"IDAT" {
            idat_ended = true;
        }
        offset = end;
        if seen_iend {
            break;
        }
    }
    let mut header = header.ok_or(MediaError::UnsupportedPng("missing IHDR"))?;
    if !seen_idat || !seen_iend || offset != bytes.len() {
        return Err(MediaError::UnsupportedPng(
            "missing IDAT/IEND or trailing bytes",
        ));
    }
    header.has_srgb = seen_srgb;
    header.has_transparency = seen_transparency;
    header.chunks = chunks;
    Ok(header)
}

fn parse_header(data: &[u8]) -> Result<PngProfileHeader, MediaError> {
    let width = read_u32(&data[0..4])?;
    let height = read_u32(&data[4..8])?;
    if width == 0 || height == 0 || width > MAX_PNG_WIDTH || height > MAX_PNG_HEIGHT {
        return Err(MediaError::ResourceLimit("dimensions"));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PNG_PIXELS {
        return Err(MediaError::ResourceLimit("decoded pixels"));
    }
    if data[8..] != [8, 6, 0, 0, 0] {
        return Err(MediaError::UnsupportedPng(
            "requires RGBA8, standard compression/filter and no interlace",
        ));
    }
    let decoded_bytes =
        usize::try_from(pixels * 4).map_err(|_| MediaError::ResourceLimit("decoded bytes"))?;
    Ok(PngProfileHeader {
        width,
        height,
        decoded_bytes,
        has_srgb: false,
        chunks: 0,
    })
}

fn parse_basic_header(data: &[u8]) -> Result<PngBasicProfileHeader, MediaError> {
    let width = read_u32(&data[0..4])?;
    let height = read_u32(&data[4..8])?;
    if width == 0 || height == 0 || width > MAX_PNG_WIDTH || height > MAX_PNG_HEIGHT {
        return Err(MediaError::ResourceLimit("dimensions"));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PNG_PIXELS {
        return Err(MediaError::ResourceLimit("decoded pixels"));
    }
    let bit_depth =
        BitDepth::from_u8(data[8]).ok_or(MediaError::UnsupportedPng("invalid bit depth"))?;
    let color_type =
        ColorType::from_u8(data[9]).ok_or(MediaError::UnsupportedPng("invalid color type"))?;
    let allowed = match color_type {
        ColorType::Grayscale | ColorType::Indexed => matches!(
            bit_depth,
            BitDepth::One | BitDepth::Two | BitDepth::Four | BitDepth::Eight
        ),
        ColorType::Rgb | ColorType::GrayscaleAlpha | ColorType::Rgba => {
            bit_depth == BitDepth::Eight
        }
    };
    if !allowed || data[10..] != [0, 0, 0] {
        return Err(MediaError::UnsupportedPng(
            "requires lossless RGBA8 expansion, standard compression/filter and no interlace",
        ));
    }
    let decoded_bytes =
        usize::try_from(pixels * 4).map_err(|_| MediaError::ResourceLimit("decoded bytes"))?;
    Ok(PngBasicProfileHeader {
        width,
        height,
        decoded_bytes,
        color_type,
        bit_depth,
        has_srgb: false,
        has_transparency: false,
        chunks: 0,
    })
}

fn read_u32(bytes: &[u8]) -> Result<u32, MediaError> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| MediaError::UnsupportedPng("truncated integer"))?;
    Ok(u32::from_be_bytes(array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture(color: ColorType) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut output), 2, 1);
            encoder.set_color(color);
            encoder.set_depth(BitDepth::Eight);
            encoder.set_filter(png::Filter::NoFilter);
            let mut writer = encoder.write_header().unwrap();
            let pixels = if color == ColorType::Rgba {
                &[1_u8, 2, 3, 4, 5, 6, 7, 8][..]
            } else {
                &[1_u8, 2][..]
            };
            writer.write_image_data(pixels).unwrap();
        }
        output
    }

    fn basic_fixture(
        width: u32,
        color: ColorType,
        depth: BitDepth,
        pixels: &[u8],
        palette: Option<&[u8]>,
        transparency: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut output), width, 1);
            encoder.set_color(color);
            encoder.set_depth(depth);
            encoder.set_filter(png::Filter::NoFilter);
            if let Some(palette) = palette {
                encoder.set_palette(palette);
            }
            if let Some(transparency) = transparency {
                encoder.set_trns(transparency);
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        output
    }

    #[test]
    fn strict_rgba8_reaches_exact_pixels() {
        let bytes = fixture(ColorType::Rgba);
        let image = decode_png_rgba8(&bytes).unwrap();
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.rgba, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(inspect_png_rgba8(&bytes).unwrap().decoded_bytes, 8);
    }

    #[test]
    fn basic_profile_expands_common_color_types_exactly() {
        let grayscale = basic_fixture(
            4,
            ColorType::Grayscale,
            BitDepth::Two,
            &[0b00_01_10_11],
            None,
            None,
        );
        assert_eq!(
            decode_png_basic_rgba8(&grayscale).unwrap().rgba,
            [
                0, 0, 0, 255, 85, 85, 85, 255, 170, 170, 170, 255, 255, 255, 255, 255
            ]
        );

        let indexed = basic_fixture(
            4,
            ColorType::Indexed,
            BitDepth::Two,
            &[0b00_01_10_11],
            Some(&[1, 2, 3, 10, 20, 30, 40, 50, 60, 70, 80, 90]),
            Some(&[0, 85, 170, 255]),
        );
        assert_eq!(
            decode_png_basic_rgba8(&indexed).unwrap().rgba,
            [1, 2, 3, 0, 10, 20, 30, 85, 40, 50, 60, 170, 70, 80, 90, 255]
        );

        let rgb = basic_fixture(
            2,
            ColorType::Rgb,
            BitDepth::Eight,
            &[1, 2, 3, 4, 5, 6],
            None,
            None,
        );
        assert_eq!(
            decode_png_basic_rgba8(&rgb).unwrap().rgba,
            [1, 2, 3, 255, 4, 5, 6, 255]
        );

        let grayscale_alpha = basic_fixture(
            2,
            ColorType::GrayscaleAlpha,
            BitDepth::Eight,
            &[9, 10, 11, 12],
            None,
            None,
        );
        assert_eq!(
            decode_png_basic_rgba8(&grayscale_alpha).unwrap().rgba,
            [9, 9, 9, 10, 11, 11, 11, 12]
        );
    }

    #[test]
    fn basic_profile_preserves_strict_profile_and_rejects_precision_loss() {
        let rgba = fixture(ColorType::Rgba);
        assert_eq!(
            decode_png_basic_rgba8(&rgba).unwrap(),
            decode_png_rgba8(&rgba).unwrap()
        );
        let rgba16 = basic_fixture(
            1,
            ColorType::Rgba,
            BitDepth::Sixteen,
            &[0, 1, 0, 2, 0, 3, 0, 4],
            None,
            None,
        );
        assert!(matches!(
            decode_png_basic_rgba8(&rgba16),
            Err(MediaError::UnsupportedPng(_))
        ));
    }

    #[test]
    fn ambiguous_conversion_and_corruption_are_rejected() {
        assert!(matches!(
            decode_png_rgba8(&fixture(ColorType::Grayscale)),
            Err(MediaError::UnsupportedPng(_))
        ));
        let mut corrupt = fixture(ColorType::Rgba);
        let index = corrupt.len() - 5;
        corrupt[index] ^= 1;
        assert!(decode_png_rgba8(&corrupt).is_err());
    }

    #[test]
    fn trailing_data_and_one_over_dimensions_fail_before_decode() {
        let mut trailing = fixture(ColorType::Rgba);
        trailing.write_all(b"x").unwrap();
        assert!(inspect_png_rgba8(&trailing).is_err());

        let mut oversized = fixture(ColorType::Rgba);
        oversized[16..20].copy_from_slice(&(MAX_PNG_WIDTH + 1).to_be_bytes());
        assert!(matches!(
            inspect_png_rgba8(&oversized),
            Err(MediaError::ResourceLimit("dimensions"))
        ));
    }
}
