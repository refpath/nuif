#![doc = "Pinned profile-0 font resources and deterministic resolved text shaping."]

use harfrust::{
    Direction, Feature, FontRef as HarfFontRef, Language, NormalizedCoord, SerializeFlags,
    ShapeOptions, ShaperData, ShaperInstance, UnicodeBuffer,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skrifa::{
    FontRef as SkrifaFontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{
        DrawSettings,
        pen::{PathElement, PathStyle},
    },
};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::str::FromStr;
use thiserror::Error;

pub const PINNED_FONT_NAME: &str = "Ahem";
pub const PINNED_FONT_VERSION: &str = "1.50";
pub const PINNED_FONT_SHA256: &str =
    "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc";
pub const PINNED_FONT_ASCENDER: i32 = 800;
pub const SHAPER_NAME: &str = "HarfRust";
pub const SHAPER_VERSION: &str = "0.13.3";
pub const UNICODE_VERSION: &str = "17.0.0";
pub const FRACTIONAL_POSITION_DENOMINATOR: u32 = 64;
pub const CLUSTER_UNIT: &str = "unicode_scalar_index";
pub const OUTLINE_EXTRACTOR_NAME: &str = "Skrifa";
pub const OUTLINE_EXTRACTOR_VERSION: &str = "0.46.2";
pub const OUTLINE_COORDINATE_DENOMINATOR: i32 = 64;
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
    #[serde(default = "default_ascender_font_units")]
    pub ascender_font_units: i32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, u32>,
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

/// Splits text at the mandatory hard-break characters supported by profile 0.
///
/// CRLF is one delimiter. CR, LF, NEL, LINE SEPARATOR and PARAGRAPH SEPARATOR
/// are individual delimiters. Delimiters are not included in the returned
/// lines, and a trailing delimiter produces a final empty line.
#[must_use]
pub fn hard_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut characters = text.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        let delimiter_end = match character {
            '\r' => {
                if characters
                    .peek()
                    .is_some_and(|(_, next_character)| *next_character == '\n')
                {
                    characters.next().map_or(
                        index + character.len_utf8(),
                        |(next_index, next_character)| next_index + next_character.len_utf8(),
                    )
                } else {
                    index + character.len_utf8()
                }
            }
            '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}' => index + character.len_utf8(),
            _ => continue,
        };
        lines.push(&text[start..index]);
        start = delimiter_end;
    }
    lines.push(&text[start..]);
    lines
}

/// Shapes every profile-0 hard line independently.
///
/// The complete source remains subject to the per-text resource limit, so
/// splitting cannot multiply the shaping budget.
///
/// # Errors
///
/// Returns the same typed failures as [`shape`].
pub fn shape_hard_lines(request: &ShapeRequest<'_>) -> Result<Vec<ShapedRun>, TextError> {
    let codepoints = request.text.chars().count();
    if codepoints > MAX_SHAPING_CODEPOINTS {
        return Err(TextError::TooManyCodepoints {
            limit: MAX_SHAPING_CODEPOINTS,
            observed: codepoints,
        });
    }
    hard_lines(request.text)
        .into_iter()
        .map(|line| {
            shape(&ShapeRequest {
                text: line,
                font_sha256: request.font_sha256,
                font_size: request.font_size,
                direction: request.direction,
                language: request.language,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphOutline {
    pub glyph_id: u32,
    pub extractor: String,
    pub extractor_version: String,
    pub coordinate_denominator: i32,
    pub commands: Vec<OutlineCommand>,
    pub serialized_path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verb")]
pub enum OutlineCommand {
    MoveTo {
        to: OutlinePoint,
    },
    LineTo {
        to: OutlinePoint,
    },
    QuadTo {
        control: OutlinePoint,
        to: OutlinePoint,
    },
    CurveTo {
        control_0: OutlinePoint,
        control_1: OutlinePoint,
        to: OutlinePoint,
    },
    Close,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlinePoint {
    pub x: i32,
    pub y: i32,
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
    #[error("font resource digest does not match {expected}: observed {observed}")]
    FontDigestMismatch { expected: String, observed: String },
    #[error("font size must be finite and positive")]
    InvalidFontSize,
    #[error("text has {observed} codepoints, exceeding the shaping limit {limit}")]
    TooManyCodepoints { limit: usize, observed: usize },
    #[error("the pinned font bytes are invalid: {0}")]
    InvalidPinnedFont(String),
    #[error("the packaged font resource is invalid: {0}")]
    InvalidResourceFont(String),
    #[error("language tag is empty")]
    InvalidLanguage,
    #[error("could not allocate the bounded shaping buffer")]
    BufferAllocationFailed,
    #[error("glyph {glyph_id} has no outline in the pinned font")]
    GlyphOutlineUnavailable { glyph_id: u32 },
    #[error("glyph {glyph_id} outline extraction failed: {reason}")]
    OutlineExtraction { glyph_id: u32, reason: String },
    #[error("glyph {glyph_id} contains a non-finite or out-of-range outline coordinate")]
    InvalidOutlineCoordinate { glyph_id: u32 },
}

const fn default_ascender_font_units() -> i32 {
    PINNED_FONT_ASCENDER
}

/// One digest-checked, profile-validated static font face that can be reused
/// across shaping and outline extraction without reparsing it per glyph.
pub struct ResourceFont<'a> {
    harf: HarfFontRef<'a>,
    skrifa: SkrifaFontRef<'a>,
    identity: FontIdentity,
    ascender_font_units: i32,
    features: Vec<Feature>,
    feature_settings: BTreeMap<String, u32>,
}

/// One variable TrueType face opened only for the staged RFC 0013 experiment.
///
/// This type deliberately has no package/session integration. It proves that
/// the already-bounded coordinate vector can be delivered unchanged to
/// shaping, metrics, and outline extraction before the candidate profile is
/// eligible for package acceptance.
pub struct VariableResourceTrial<'a> {
    harf: HarfFontRef<'a>,
    skrifa: SkrifaFontRef<'a>,
    instance: ShaperInstance,
    location: Vec<NormalizedCoord>,
    coordinates: Vec<nuif_font::VariableCoordinate>,
    identity: FontIdentity,
    ascender_font_units: i32,
    features: Vec<Feature>,
    feature_settings: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableShapedRun {
    pub coordinates: Vec<nuif_font::VariableCoordinate>,
    pub run: ShapedRun,
}

/// Location-adjusted global metrics from one RFC 0013 variable-font trial.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableGlobalMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    pub x_height: Option<i32>,
    pub cap_height: Option<i32>,
}

impl<'a> ResourceFont<'a> {
    /// Validates and opens an exact packaged static TrueType font.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a digest mismatch, a decoder-profile
    /// rejection, inconsistent family metadata, or an empty license record.
    pub fn new(
        bytes: &'a [u8],
        expected_sha256: &str,
        family: &str,
        license: &str,
    ) -> Result<Self, TextError> {
        Self::new_with_features(bytes, expected_sha256, family, license, &BTreeMap::new())
    }

    /// Validates and opens an exact packaged static TrueType font with global
    /// OpenType feature values applied to every shaped run.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Self::new`] and rejects feature tags
    /// that the pinned shaper cannot represent exactly.
    pub fn new_with_features(
        bytes: &'a [u8],
        expected_sha256: &str,
        family: &str,
        license: &str,
        feature_settings: &BTreeMap<String, u32>,
    ) -> Result<Self, TextError> {
        let observed = format!("{:x}", Sha256::digest(bytes));
        if observed != expected_sha256 {
            return Err(TextError::FontDigestMismatch {
                expected: expected_sha256.to_owned(),
                observed,
            });
        }
        let inspection = nuif_font::inspect_opentype_static(bytes, 0)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        if family.is_empty() || !inspection.names.iter().any(|name| name == family) {
            return Err(TextError::InvalidResourceFont(
                "asset family is absent from the inspected font".to_owned(),
            ));
        }
        if license.trim().is_empty() {
            return Err(TextError::InvalidResourceFont(
                "asset license expression is empty".to_owned(),
            ));
        }
        let harf = HarfFontRef::new(bytes)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        let skrifa = SkrifaFontRef::new(bytes)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        let ascent = skrifa
            .metrics(Size::unscaled(), LocationRef::default())
            .ascent;
        let ascender_font_units = integral_font_metric(ascent, "font ascent")?;
        let features = feature_settings
            .iter()
            .map(|(tag, value)| {
                Feature::from_str(&format!("{tag}={value}"))
                    .map_err(|_| TextError::InvalidResourceFont("invalid feature tag".to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            harf,
            skrifa,
            identity: FontIdentity {
                family: family.to_owned(),
                version: "unreported".to_owned(),
                sha256: expected_sha256.to_owned(),
                byte_length: bytes.len(),
                license: license.to_owned(),
            },
            ascender_font_units,
            features,
            feature_settings: feature_settings.clone(),
        })
    }

    /// Shapes one logical run with this exact resource face.
    ///
    /// # Errors
    ///
    /// Returns typed request or shaping failures.
    pub fn shape(&self, request: &ShapeRequest<'_>) -> Result<ShapedRun, TextError> {
        if request.font_sha256 != self.identity.sha256 {
            return Err(TextError::FontDigestMismatch {
                expected: self.identity.sha256.clone(),
                observed: request.font_sha256.to_owned(),
            });
        }
        shape_with_font(
            request,
            &self.harf,
            None,
            self.identity.clone(),
            self.ascender_font_units,
            &self.features,
            &self.feature_settings,
        )
    }

    /// Shapes every supported hard line with this exact resource face.
    ///
    /// # Errors
    ///
    /// Returns typed request or shaping failures.
    pub fn shape_hard_lines(
        &self,
        request: &ShapeRequest<'_>,
    ) -> Result<Vec<ShapedRun>, TextError> {
        let codepoints = request.text.chars().count();
        if codepoints > MAX_SHAPING_CODEPOINTS {
            return Err(TextError::TooManyCodepoints {
                limit: MAX_SHAPING_CODEPOINTS,
                observed: codepoints,
            });
        }
        hard_lines(request.text)
            .into_iter()
            .map(|line| {
                self.shape(&ShapeRequest {
                    text: line,
                    font_sha256: request.font_sha256,
                    font_size: request.font_size,
                    direction: request.direction,
                    language: request.language,
                })
            })
            .collect()
    }

    /// Extracts one unhinted outline from this exact resource face.
    ///
    /// # Errors
    ///
    /// Returns a typed unavailable-glyph or outline-geometry failure.
    pub fn outline_glyph(&self, glyph_id: u32) -> Result<GlyphOutline, TextError> {
        outline_from_font(&self.skrifa, glyph_id)
    }
}

impl<'a> VariableResourceTrial<'a> {
    /// Opens the exact single-face TrueType variable resource and applies one
    /// complete user-coordinate tuple for the RFC 0013 experiment.
    ///
    /// # Errors
    ///
    /// Rejects digest, metadata, family, license, feature, or coordinate
    /// disagreement before any shaping or outline operation can run.
    pub fn new_with_features(
        bytes: &'a [u8],
        expected_sha256: &str,
        family: &str,
        license: &str,
        axis_settings: &BTreeMap<String, f64>,
        feature_settings: &BTreeMap<String, u32>,
    ) -> Result<Self, TextError> {
        let observed = format!("{:x}", Sha256::digest(bytes));
        if observed != expected_sha256 {
            return Err(TextError::FontDigestMismatch {
                expected: expected_sha256.to_owned(),
                observed,
            });
        }
        let inspection = nuif_font::inspect_opentype_variable_metadata(bytes, 0)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        if family.is_empty() || !inspection.font.names.iter().any(|name| name == family) {
            return Err(TextError::InvalidResourceFont(
                "asset family is absent from the inspected variable font".to_owned(),
            ));
        }
        if license.trim().is_empty() {
            return Err(TextError::InvalidResourceFont(
                "asset license expression is empty".to_owned(),
            ));
        }
        let coordinates = nuif_font::normalize_variable_coordinates(bytes, axis_settings)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        let location = coordinates
            .iter()
            .map(|coordinate| NormalizedCoord::from_bits(coordinate.normalized_2_14))
            .collect::<Vec<_>>();
        let harf = HarfFontRef::new(bytes)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        let skrifa = SkrifaFontRef::new(bytes)
            .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
        let instance = ShaperInstance::from_coords(&harf, location.iter().copied());
        let instance_agrees = if location
            .iter()
            .all(|coordinate| *coordinate == NormalizedCoord::ZERO)
        {
            instance.coords().is_empty()
        } else {
            instance.coords() == location
        };
        if !instance_agrees {
            return Err(TextError::InvalidResourceFont(
                "HarfRust changed the normalized coordinate vector".to_owned(),
            ));
        }
        let ascent = skrifa
            .metrics(Size::unscaled(), LocationRef::new(&location))
            .ascent;
        let ascender_font_units = integral_font_metric(ascent, "variable font ascent")?;
        let features = feature_settings
            .iter()
            .map(|(tag, value)| {
                Feature::from_str(&format!("{tag}={value}"))
                    .map_err(|_| TextError::InvalidResourceFont("invalid feature tag".to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            harf,
            skrifa,
            instance,
            location,
            coordinates,
            identity: FontIdentity {
                family: family.to_owned(),
                version: "unreported".to_owned(),
                sha256: expected_sha256.to_owned(),
                byte_length: bytes.len(),
                license: license.to_owned(),
            },
            ascender_font_units,
            features,
            feature_settings: feature_settings.clone(),
        })
    }

    /// Shapes one run with the exact normalized vector retained beside the
    /// result.
    ///
    /// # Errors
    ///
    /// Returns typed request, digest, or shaping failures.
    pub fn shape(&self, request: &ShapeRequest<'_>) -> Result<VariableShapedRun, TextError> {
        if request.font_sha256 != self.identity.sha256 {
            return Err(TextError::FontDigestMismatch {
                expected: self.identity.sha256.clone(),
                observed: request.font_sha256.to_owned(),
            });
        }
        Ok(VariableShapedRun {
            coordinates: self.coordinates.clone(),
            run: shape_with_font(
                request,
                &self.harf,
                Some(&self.instance),
                self.identity.clone(),
                self.ascender_font_units,
                &self.features,
                &self.feature_settings,
            )?,
        })
    }

    /// Returns Skrifa's location-adjusted unscaled advance for one glyph.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the glyph or metric is unavailable or the
    /// resulting value cannot be represented in the profile's integer units.
    pub fn glyph_advance_font_units(&self, glyph_id: u32) -> Result<i32, TextError> {
        let advance = self
            .skrifa
            .glyph_metrics(Size::unscaled(), LocationRef::new(&self.location))
            .advance_width(GlyphId::new(glyph_id))
            .ok_or(TextError::GlyphOutlineUnavailable { glyph_id })?;
        integral_font_metric(advance, "variable glyph advance")
    }

    /// Returns Skrifa's location-adjusted unscaled global metrics.
    ///
    /// This is part of the research-only variable resource surface. In
    /// particular, it does not make MVAR data available to package layout.
    ///
    /// # Errors
    ///
    /// Returns a typed failure if a metric is non-finite or outside the
    /// profile's integer font-unit domain.
    pub fn global_metrics_font_units(&self) -> Result<VariableGlobalMetrics, TextError> {
        let metrics = self
            .skrifa
            .metrics(Size::unscaled(), LocationRef::new(&self.location));
        Ok(VariableGlobalMetrics {
            ascent: integral_font_metric(metrics.ascent, "variable font ascent")?,
            descent: integral_font_metric(metrics.descent, "variable font descent")?,
            line_gap: integral_font_metric(metrics.leading, "variable font line gap")?,
            x_height: metrics
                .x_height
                .map(|value| integral_font_metric(value, "variable font x-height"))
                .transpose()?,
            cap_height: metrics
                .cap_height
                .map(|value| integral_font_metric(value, "variable font cap height"))
                .transpose()?,
        })
    }

    /// Extracts an unhinted outline at the identical normalized location used
    /// by shaping and metrics.
    ///
    /// # Errors
    ///
    /// Returns a typed unavailable-glyph or outline-geometry failure.
    pub fn outline_glyph(&self, glyph_id: u32) -> Result<GlyphOutline, TextError> {
        outline_from_font_at(&self.skrifa, glyph_id, LocationRef::new(&self.location))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the finite rounded value is range-checked before conversion"
)]
fn integral_font_metric(value: f32, name: &str) -> Result<i32, TextError> {
    let rounded = f64::from(value).round();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(TextError::InvalidResourceFont(format!(
            "{name} is non-finite or outside the signed metric domain"
        )));
    }
    Ok(rounded as i32)
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
    let font = HarfFontRef::new(pinned_font_bytes())
        .map_err(|error| TextError::InvalidPinnedFont(error.to_string()))?;
    shape_with_font(
        request,
        &font,
        None,
        pinned_font_identity(),
        PINNED_FONT_ASCENDER,
        &[],
        &BTreeMap::new(),
    )
}

/// Shapes one logical run with an exact, profile-validated packaged static
/// TrueType font. The caller supplies the already reviewed family and license
/// metadata carried by the NUIF font asset; this function verifies the bytes,
/// digest and decoder profile before shaping.
///
/// # Errors
///
/// Returns a typed error for digest mismatch, profile rejection, inconsistent
/// family metadata, invalid request inputs or shaping failure.
pub fn shape_resource(
    request: &ShapeRequest<'_>,
    bytes: &[u8],
    family: &str,
    license: &str,
) -> Result<ShapedRun, TextError> {
    ResourceFont::new(bytes, request.font_sha256, family, license)?.shape(request)
}

/// Shapes every supported hard line with one exact packaged font resource.
///
/// # Errors
///
/// Returns the same typed failures as [`shape_resource`].
pub fn shape_hard_lines_resource(
    request: &ShapeRequest<'_>,
    bytes: &[u8],
    family: &str,
    license: &str,
) -> Result<Vec<ShapedRun>, TextError> {
    ResourceFont::new(bytes, request.font_sha256, family, license)?.shape_hard_lines(request)
}

fn shape_with_font(
    request: &ShapeRequest<'_>,
    font: &HarfFontRef<'_>,
    instance: Option<&ShaperInstance>,
    identity: FontIdentity,
    ascender_font_units: i32,
    features: &[Feature],
    feature_settings: &BTreeMap<String, u32>,
) -> Result<ShapedRun, TextError> {
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
    let shaper_data = ShaperData::new(font);
    let shaper = shaper_data.shaper(font).instance(instance).build();
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
    let output = shaper.shape(buffer, ShapeOptions::new().features(features));
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
        font: identity,
        font_size: request.font_size,
        units_per_em: u16::try_from(shaper.units_per_em()).unwrap_or(u16::MAX),
        ascender_font_units,
        features: feature_settings.clone(),
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

/// Extracts one unhinted profile-0 glyph outline in signed 26.6 font units.
///
/// The `HarfBuzz` point-stream interpretation is selected explicitly so the
/// serialized command stream can be compared with `hb-vector` output.
///
/// # Errors
///
/// Returns a typed error for malformed pinned bytes, unavailable glyphs,
/// extraction failures, or coordinates outside the signed 26.6 domain.
pub fn outline_glyph(glyph_id: u32) -> Result<GlyphOutline, TextError> {
    let font = SkrifaFontRef::new(pinned_font_bytes())
        .map_err(|error| TextError::InvalidPinnedFont(error.to_string()))?;
    outline_from_font(&font, glyph_id)
}

/// Extracts one unhinted glyph outline from an exact profile-validated static
/// font resource.
///
/// # Errors
///
/// Returns a typed error for digest mismatch, profile rejection, unavailable
/// glyphs or invalid outline geometry.
pub fn outline_resource_glyph(
    bytes: &[u8],
    expected_sha256: &str,
    glyph_id: u32,
) -> Result<GlyphOutline, TextError> {
    let observed = format!("{:x}", Sha256::digest(bytes));
    if observed != expected_sha256 {
        return Err(TextError::FontDigestMismatch {
            expected: expected_sha256.to_owned(),
            observed,
        });
    }
    nuif_font::inspect_opentype_static(bytes, 0)
        .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
    let font = SkrifaFontRef::new(bytes)
        .map_err(|error| TextError::InvalidResourceFont(error.to_string()))?;
    outline_from_font(&font, glyph_id)
}

fn outline_from_font(font: &SkrifaFontRef<'_>, glyph_id: u32) -> Result<GlyphOutline, TextError> {
    outline_from_font_at(font, glyph_id, LocationRef::default())
}

fn outline_from_font_at(
    font: &SkrifaFontRef<'_>,
    glyph_id: u32,
    location: LocationRef<'_>,
) -> Result<GlyphOutline, TextError> {
    let glyph = font
        .outline_glyphs()
        .get(GlyphId::new(glyph_id))
        .ok_or(TextError::GlyphOutlineUnavailable { glyph_id })?;
    let settings =
        DrawSettings::unhinted(Size::unscaled(), location).with_path_style(PathStyle::HarfBuzz);
    let mut elements = Vec::<PathElement>::new();
    glyph
        .draw(settings, &mut elements)
        .map_err(|error| TextError::OutlineExtraction {
            glyph_id,
            reason: error.to_string(),
        })?;
    let commands = elements
        .into_iter()
        .map(|element| outline_command(glyph_id, element))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GlyphOutline {
        glyph_id,
        extractor: OUTLINE_EXTRACTOR_NAME.to_owned(),
        extractor_version: OUTLINE_EXTRACTOR_VERSION.to_owned(),
        coordinate_denominator: OUTLINE_COORDINATE_DENOMINATOR,
        serialized_path: serialize_outline(&commands),
        commands,
    })
}

fn outline_command(glyph_id: u32, element: PathElement) -> Result<OutlineCommand, TextError> {
    Ok(match element {
        PathElement::MoveTo { x, y } => OutlineCommand::MoveTo {
            to: outline_point(glyph_id, x, y)?,
        },
        PathElement::LineTo { x, y } => OutlineCommand::LineTo {
            to: outline_point(glyph_id, x, y)?,
        },
        PathElement::QuadTo { cx0, cy0, x, y } => OutlineCommand::QuadTo {
            control: outline_point(glyph_id, cx0, cy0)?,
            to: outline_point(glyph_id, x, y)?,
        },
        PathElement::CurveTo {
            cx0,
            cy0,
            cx1,
            cy1,
            x,
            y,
        } => OutlineCommand::CurveTo {
            control_0: outline_point(glyph_id, cx0, cy0)?,
            control_1: outline_point(glyph_id, cx1, cy1)?,
            to: outline_point(glyph_id, x, y)?,
        },
        PathElement::Close => OutlineCommand::Close,
    })
}

fn outline_point(glyph_id: u32, x: f32, y: f32) -> Result<OutlinePoint, TextError> {
    Ok(OutlinePoint {
        x: quantize_outline_coordinate(glyph_id, x)?,
        y: quantize_outline_coordinate(glyph_id, y)?,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the scaled value is checked against the complete i32 domain before conversion"
)]
fn quantize_outline_coordinate(glyph_id: u32, value: f32) -> Result<i32, TextError> {
    let scaled = f64::from(value) * f64::from(OUTLINE_COORDINATE_DENOMINATOR);
    if !scaled.is_finite() || scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return Err(TextError::InvalidOutlineCoordinate { glyph_id });
    }
    Ok(scaled.round() as i32)
}

fn serialize_outline(commands: &[OutlineCommand]) -> String {
    let mut output = String::new();
    for command in commands {
        match command {
            OutlineCommand::MoveTo { to } => write!(output, "M{},{}", to.x, to.y),
            OutlineCommand::LineTo { to } => write!(output, "L{},{}", to.x, to.y),
            OutlineCommand::QuadTo { control, to } => {
                write!(output, "Q{},{} {},{}", control.x, control.y, to.x, to.y)
            }
            OutlineCommand::CurveTo {
                control_0,
                control_1,
                to,
            } => write!(
                output,
                "C{},{} {},{} {},{}",
                control_0.x, control_0.y, control_1.x, control_1.y, to.x, to.y
            ),
            OutlineCommand::Close => output.write_char('Z'),
        }
        .expect("writing to a String cannot fail");
    }
    output
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

    const VARIABLE_FIXTURE_SHA256: &str =
        "fdd9bade0cde742725168298e39291309c95a826acb979cef1142063f17f44ab";

    fn request(text: &str, direction: TextDirection) -> ShapeRequest<'_> {
        ShapeRequest {
            text,
            font_sha256: PINNED_FONT_SHA256,
            font_size: 18.0,
            direction,
            language: "en",
        }
    }

    fn variable_axes(
        fill: f64,
        grade: f64,
        optical_size: f64,
        weight: f64,
    ) -> BTreeMap<String, f64> {
        BTreeMap::from([
            ("FILL".to_owned(), fill),
            ("GRAD".to_owned(), grade),
            ("opsz".to_owned(), optical_size),
            ("wght".to_owned(), weight),
        ])
    }

    fn variable_request(text: &str) -> ShapeRequest<'_> {
        ShapeRequest {
            text,
            font_sha256: VARIABLE_FIXTURE_SHA256,
            font_size: 24.0,
            direction: TextDirection::LeftToRight,
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
    fn unhinted_outlines_match_harfbuzz_vector_goldens() {
        assert_eq!(outline_glyph(3).unwrap().serialized_path, "");
        assert_eq!(
            outline_glyph(35).unwrap().serialized_path,
            "M0,51200L64000,51200L64000,-12800L0,-12800Z"
        );
        assert_eq!(
            outline_glyph(82).unwrap().serialized_path,
            "M0,0L64000,0L64000,-12800L0,-12800Z"
        );
        assert_eq!(
            outline_glyph(100).unwrap().serialized_path,
            "M0,51200L64000,51200L64000,0L0,0Z"
        );
        assert_eq!(
            outline_glyph(275).unwrap().serialized_path,
            "M0,38400L64000,38400L64000,25600L0,25600Z"
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

    #[test]
    fn hard_lines_preserve_empty_lines_and_coalesce_crlf() {
        assert_eq!(hard_lines(""), vec![""]);
        assert_eq!(
            hard_lines("A\r\nB\rC\nD\u{0085}E\u{2028}F\u{2029}"),
            vec!["A", "B", "C", "D", "E", "F", ""]
        );
        assert_eq!(hard_lines("A\n\nB"), vec!["A", "", "B"]);
    }

    #[test]
    fn hard_lines_shape_as_independent_runs() {
        let runs = shape_hard_lines(&request("A\r\nB", TextDirection::LeftToRight)).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].serialized_glyphs, "[35=0+1000]");
        assert_eq!(runs[1].serialized_glyphs, "[36=0+1000]");
    }

    #[test]
    fn exact_static_resource_is_reused_for_shaping_metrics_and_outlines() {
        let bytes = font_test_data::TINOS_SUBSET;
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let inspection = nuif_font::inspect_opentype_static(bytes, 0).unwrap();
        let family = inspection.names.first().unwrap();
        let font = ResourceFont::new(bytes, &sha256, family, "Apache-2.0").unwrap();
        let request = ShapeRequest {
            text: "A\nB",
            font_sha256: &sha256,
            font_size: 18.0,
            direction: TextDirection::LeftToRight,
            language: "en",
        };
        let runs = font.shape_hard_lines(&request).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].font.sha256, sha256);
        assert_eq!(runs[0].font.family, *family);
        assert_ne!(runs[0].ascender_font_units, PINNED_FONT_ASCENDER);
        assert!(runs[0].ascender_font_units > 0);
        let outline = font.outline_glyph(runs[0].glyphs[0].glyph_id).unwrap();
        assert!(!outline.commands.is_empty());
    }

    #[test]
    fn variable_trial_reuses_one_location_for_shaping_metrics_and_outlines() {
        let default = VariableResourceTrial::new_with_features(
            font_test_data::MATERIAL_SYMBOLS_SUBSET,
            VARIABLE_FIXTURE_SHA256,
            "Material Symbols Outlined",
            "font-test-data package: MIT OR Apache-2.0; publisher review not asserted",
            &variable_axes(0.0, 0.0, 24.0, 400.0),
            &BTreeMap::new(),
        )
        .unwrap();
        let filled = VariableResourceTrial::new_with_features(
            font_test_data::MATERIAL_SYMBOLS_SUBSET,
            VARIABLE_FIXTURE_SHA256,
            "Material Symbols Outlined",
            "font-test-data package: MIT OR Apache-2.0; publisher review not asserted",
            &variable_axes(1.0, 200.0, 48.0, 700.0),
            &BTreeMap::new(),
        )
        .unwrap();
        let default_run = default.shape(&variable_request("mail")).unwrap();
        let filled_run = filled.shape(&variable_request("mail")).unwrap();
        assert_eq!(default_run.run.serialized_glyphs, "[1=0+960]");
        assert_eq!(filled_run.run.serialized_glyphs, "[2=0+960]");
        assert_eq!(default_run.coordinates.len(), 4);
        assert_eq!(filled_run.coordinates.len(), 4);
        assert_eq!(
            default.glyph_advance_font_units(1).unwrap(),
            default_run.run.glyphs[0].x_advance
        );
        assert_eq!(
            filled.glyph_advance_font_units(2).unwrap(),
            filled_run.run.glyphs[0].x_advance
        );
        let default_outline = default.outline_glyph(1).unwrap();
        let filled_outline = filled.outline_glyph(2).unwrap();
        assert!(!default_outline.commands.is_empty());
        assert!(!filled_outline.commands.is_empty());
        assert_ne!(
            default_outline.serialized_path,
            filled_outline.serialized_path
        );
    }

    #[test]
    fn variable_trial_rejects_partial_coordinates_before_shaping() {
        assert!(
            VariableResourceTrial::new_with_features(
                font_test_data::MATERIAL_SYMBOLS_SUBSET,
                VARIABLE_FIXTURE_SHA256,
                "Material Symbols Outlined",
                "test-only",
                &BTreeMap::from([("wght".to_owned(), 400.0)]),
                &BTreeMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn exact_static_resource_rejects_a_false_digest() {
        assert!(matches!(
            ResourceFont::new(
                font_test_data::TINOS_SUBSET,
                PINNED_FONT_SHA256,
                "Tinos",
                "Apache-2.0",
            ),
            Err(TextError::FontDigestMismatch { .. })
        ));
    }

    #[test]
    fn exact_static_resource_applies_and_records_global_features() {
        let bytes = font_test_data::TINOS_SUBSET;
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let inspection = nuif_font::inspect_opentype_static(bytes, 0).unwrap();
        let family = inspection.names.first().unwrap();
        let request = ShapeRequest {
            text: "A B",
            font_sha256: &sha256,
            font_size: 18.0,
            direction: TextDirection::LeftToRight,
            language: "en",
        };
        let defaults = ResourceFont::new(bytes, &sha256, family, "Apache-2.0")
            .unwrap()
            .shape(&request)
            .unwrap();
        let features = BTreeMap::from([("kern".to_owned(), 0)]);
        let disabled =
            ResourceFont::new_with_features(bytes, &sha256, family, "Apache-2.0", &features)
                .unwrap()
                .shape(&request)
                .unwrap();
        assert_ne!(defaults.serialized_glyphs, disabled.serialized_glyphs);
        assert_eq!(disabled.features, features);
    }
}
