#![doc = "Bounded media decoding profiles shared by capture and rendering."]

use png::{BitDepth, ColorType, Transformations};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

pub const PNG_RGBA8_PROFILE: &str = "nuif-png-rgba8-0";
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
    #[error("PNG does not satisfy {PNG_RGBA8_PROFILE}: {0}")]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PngProfileHeader {
    pub width: u32,
    pub height: u32,
    pub decoded_bytes: usize,
    pub has_srgb: bool,
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
