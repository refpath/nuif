#![doc = "Pinned profile-0 font resources and deterministic resolved text shaping."]

use harfrust::{
    Direction, FontRef, Language, SerializeFlags, ShapeOptions, ShaperData, UnicodeBuffer,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::str::FromStr;
use thiserror::Error;

pub const PINNED_FONT_NAME: &str = "Ahem";
pub const PINNED_FONT_VERSION: &str = "1.50";
pub const PINNED_FONT_SHA256: &str =
    "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc";
pub const SHAPER_NAME: &str = "HarfRust";
pub const SHAPER_VERSION: &str = "0.13.3";
pub const UNICODE_VERSION: &str = "17.0.0";
pub const FRACTIONAL_POSITION_DENOMINATOR: u32 = 64;
pub const CLUSTER_UNIT: &str = "unicode_scalar_index";
pub const MAX_SHAPING_CODEPOINTS: usize = 65_536;

#[must_use]
pub fn pinned_font_bytes() -> &'static [u8] {
    font_test_data::AHEM
}

#[must_use]
pub fn pinned_font_identity() -> FontIdentity {
    FontIdentity {
        family: PINNED_FONT_NAME.to_owned(),
        version: PINNED_FONT_VERSION.to_owned(),
        sha256: PINNED_FONT_SHA256.to_owned(),
        byte_length: pinned_font_bytes().len(),
        license: "public domain or CC0-1.0".to_owned(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontIdentity {
    pub family: String,
    pub version: String,
    pub sha256: String,
    pub byte_length: usize,
    pub license: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeRequest<'a> {
    pub text: &'a str,
    pub font_sha256: &'a str,
    pub font_size: f64,
    pub direction: TextDirection,
    pub language: &'a str,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapedRun {
    pub text: String,
    pub font: FontIdentity,
    pub font_size: f64,
    pub units_per_em: u16,
    pub direction: TextDirection,
    pub language: String,
    pub shaper: String,
    pub shaper_version: String,
    pub unicode_version: String,
    pub fractional_position_denominator: u32,
    pub cluster_unit: String,
    pub glyphs: Vec<ShapedGlyph>,
    pub serialized_glyphs: String,
    pub x_advance_font_units: i64,
    pub y_advance_font_units: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub x_advance: i32,
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum TextError {
    #[error("font hash {observed} is unavailable; profile 0 provides {expected}")]
    FontUnavailable {
        expected: &'static str,
        observed: String,
    },
    #[error("font hash {hash} is absent from the evaluation context")]
    FontAbsentFromContext { hash: String },
    #[error("font size must be finite and positive")]
    InvalidFontSize,
    #[error("text has {observed} codepoints, exceeding the shaping limit {limit}")]
    TooManyCodepoints { limit: usize, observed: usize },
    #[error("the pinned font bytes are invalid: {0}")]
    InvalidPinnedFont(String),
    #[error("language tag is empty")]
    InvalidLanguage,
    #[error("could not allocate the bounded shaping buffer")]
    BufferAllocationFailed,
}

/// Shapes one logical run with the exact profile-0 font and shaper inputs.
///
/// # Errors
///
/// Returns a typed error for an unavailable font hash, invalid size or
/// language, excessive input, or invalid built-in font data.
pub fn shape(request: &ShapeRequest<'_>) -> Result<ShapedRun, TextError> {
    if request.font_sha256 != PINNED_FONT_SHA256 {
        return Err(TextError::FontUnavailable {
            expected: PINNED_FONT_SHA256,
            observed: request.font_sha256.to_owned(),
        });
    }
    if !request.font_size.is_finite() || request.font_size <= 0.0 {
        return Err(TextError::InvalidFontSize);
    }
    let codepoints = request.text.chars().count();
    if codepoints > MAX_SHAPING_CODEPOINTS {
        return Err(TextError::TooManyCodepoints {
            limit: MAX_SHAPING_CODEPOINTS,
            observed: codepoints,
        });
    }
    let language = Language::from_str(request.language).map_err(|_| TextError::InvalidLanguage)?;
    let font = FontRef::new(pinned_font_bytes())
        .map_err(|error| TextError::InvalidPinnedFont(error.to_string()))?;
    let shaper_data = ShaperData::new(&font);
    let shaper = shaper_data.shaper(&font).build();
    let mut buffer = UnicodeBuffer::new();
    if !buffer.reserve(codepoints) {
        return Err(TextError::BufferAllocationFailed);
    }
    for (index, codepoint) in request.text.chars().enumerate() {
        let cluster = u32::try_from(index).map_err(|_| TextError::TooManyCodepoints {
            limit: MAX_SHAPING_CODEPOINTS,
            observed: codepoints,
        })?;
        buffer.add(codepoint, cluster);
    }
    buffer.set_direction(match request.direction {
        TextDirection::LeftToRight => Direction::LeftToRight,
        TextDirection::RightToLeft => Direction::RightToLeft,
    });
    buffer.set_language(language);
    buffer.guess_segment_properties();
    let output = shaper.shape(buffer, ShapeOptions::new());
    let serialized_glyphs = output.serialize(&shaper, SerializeFlags::NO_GLYPH_NAMES);
    let glyphs = output
        .glyph_infos()
        .iter()
        .zip(output.glyph_positions())
        .map(|(info, position)| ShapedGlyph {
            glyph_id: info.glyph_id,
            cluster: info.cluster,
            x_advance: position.x_advance,
            y_advance: position.y_advance,
            x_offset: position.x_offset,
            y_offset: position.y_offset,
        })
        .collect::<Vec<_>>();
    let x_advance_font_units = glyphs.iter().map(|glyph| i64::from(glyph.x_advance)).sum();
    let y_advance_font_units = glyphs.iter().map(|glyph| i64::from(glyph.y_advance)).sum();
    Ok(ShapedRun {
        text: request.text.to_owned(),
        font: pinned_font_identity(),
        font_size: request.font_size,
        units_per_em: u16::try_from(shaper.units_per_em()).unwrap_or(u16::MAX),
        direction: request.direction,
        language: request.language.to_owned(),
        shaper: SHAPER_NAME.to_owned(),
        shaper_version: SHAPER_VERSION.to_owned(),
        unicode_version: UNICODE_VERSION.to_owned(),
        fractional_position_denominator: FRACTIONAL_POSITION_DENOMINATOR,
        cluster_unit: CLUSTER_UNIT.to_owned(),
        glyphs,
        serialized_glyphs,
        x_advance_font_units,
        y_advance_font_units,
    })
}

#[must_use]
pub fn pinned_font_hash_is_valid() -> bool {
    let digest = Sha256::digest(pinned_font_bytes());
    let mut actual = String::with_capacity(64);
    for byte in digest {
        write!(actual, "{byte:02x}").expect("writing to a String cannot fail");
    }
    actual == PINNED_FONT_SHA256
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str, direction: TextDirection) -> ShapeRequest<'_> {
        ShapeRequest {
            text,
            font_sha256: PINNED_FONT_SHA256,
            font_size: 18.0,
            direction,
            language: "en",
        }
    }

    #[test]
    fn pinned_font_hash_and_harfbuzz_golden_match() {
        assert!(pinned_font_hash_is_valid());
        let run = shape(&request("A B", TextDirection::LeftToRight)).unwrap();
        assert_eq!(run.serialized_glyphs, "[35=0+1000|3=1+1000|36=2+1000]");
        assert_eq!(run.x_advance_font_units, 3000);
    }

    #[test]
    fn direction_is_part_of_resolved_output() {
        let run = shape(&request("A B", TextDirection::RightToLeft)).unwrap();
        assert_eq!(run.serialized_glyphs, "[36=2+1000|3=1+1000|35=0+1000]");
    }

    #[test]
    fn clusters_are_unicode_scalar_indices() {
        let run = shape(&request("pÉ 横", TextDirection::LeftToRight)).unwrap();
        assert_eq!(run.cluster_unit, CLUSTER_UNIT);
        assert_eq!(
            run.serialized_glyphs,
            "[82=0+1000|100=1+1000|3=2+1000|275=3+1000]"
        );
    }

    #[test]
    fn unregistered_fonts_and_excessive_runs_are_rejected() {
        let mut missing = request("A", TextDirection::LeftToRight);
        missing.font_sha256 = "missing";
        assert!(matches!(
            shape(&missing),
            Err(TextError::FontUnavailable { .. })
        ));
        let text = "A".repeat(MAX_SHAPING_CODEPOINTS + 1);
        assert!(matches!(
            shape(&request(&text, TextDirection::LeftToRight)),
            Err(TextError::TooManyCodepoints { .. })
        ));
    }
}
