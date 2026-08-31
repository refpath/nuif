#![doc = "Bounded, policy-explicit OpenType resource profiles."]

use nuif_core::{Asset, AssetKind, AssetPortability, CodepointRange, FontAsset};
use serde::{Deserialize, Serialize};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size},
    string::StringId,
};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const OPENTYPE_STATIC_PROFILE: &str = "nuif-opentype-static-single-0";
pub const OPENTYPE_VARIABLE_TRUETYPE_PROFILE: &str = "nuif-opentype-variable-truetype-single-0";
pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FONT_TABLES: usize = 256;
pub const MAX_FONT_NAMES: usize = 256;
pub const MAX_FONT_COVERAGE_RANGES: usize = 65_536;
pub const MAX_FONT_FEATURES: usize = 64;
pub const MAX_VARIABLE_AXES: usize = 16;
pub const MAX_VARIABLE_INSTANCES: usize = 256;
pub const MAX_AVAR_SEGMENTS_PER_AXIS: usize = 256;

const SFNT_TRUETYPE: [u8; 4] = [0, 1, 0, 0];
const FONT_CHECKSUM_MAGIC: u32 = 0xb1b0_afba;
const REQUIRED_TABLES: [[u8; 4]; 9] = [
    *b"OS/2", *b"cmap", *b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp", *b"name",
];
const REJECTED_TABLES: [[u8; 4]; 11] = [
    *b"CFF ", *b"CFF2", *b"COLR", *b"CPAL", *b"CBDT", *b"CBLC", *b"EBDT", *b"EBLC", *b"SVG ",
    *b"fvar", *b"sbix",
];
const REQUIRED_VARIABLE_TABLES: [[u8; 4]; 3] = [*b"fvar", *b"gvar", *b"STAT"];
const REJECTED_VARIABLE_TABLES: [[u8; 4]; 11] = [
    *b"CFF ", *b"CFF2", *b"COLR", *b"CPAL", *b"CBDT", *b"CBLC", *b"EBDT", *b"EBLC", *b"SVG ",
    *b"sbix", *b"VARC",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingPermission {
    Installable,
    Restricted,
    PreviewAndPrint,
    Editable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontInspection {
    pub decoder_profile: String,
    pub byte_length: usize,
    pub face_index: u32,
    pub units_per_em: u16,
    pub glyph_count: u16,
    pub names: Vec<String>,
    pub coverage: Vec<CodepointRange>,
    pub os2_version: u16,
    pub fs_type: u16,
    pub permission: EmbeddingPermission,
    pub subsetting_allowed: bool,
    pub outline_embedding_allowed: bool,
    pub table_tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableAxisInspection {
    pub tag: String,
    pub minimum_16_16: i32,
    pub default_16_16: i32,
    pub maximum_16_16: i32,
    pub hidden: bool,
    pub name_id: u16,
    pub avar_segments: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableFontInspection {
    pub font: FontInspection,
    pub axes: Vec<VariableAxisInspection>,
    pub named_instance_count: usize,
    pub avar_version: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableCoordinate {
    pub tag: String,
    pub user_16_16: i32,
    pub normalized_2_14: i16,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum FontError {
    #[error("font resource limit exceeded for {resource}: limit {limit}, observed {observed}")]
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    #[error("font does not satisfy the declared bounded OpenType profile: {0}")]
    Unsupported(&'static str),
    #[error("font parser rejected the resource: {0}")]
    Parse(String),
    #[error("font table {tag} checksum is invalid")]
    TableChecksum { tag: String },
    #[error("complete font checksum is invalid")]
    FontChecksum,
    #[error("font asset does not match the exact resource: {0}")]
    AssetMismatch(&'static str),
    #[error("font embedding requires explicit policy evidence: {0}")]
    Policy(&'static str),
}

/// Inspects the exact static, single-face TrueType resource profile.
///
/// The profile is intentionally narrower than OpenType: collections, CFF,
/// variable, colour, bitmap and SVG glyph sources are rejected. Parsing never
/// grants permission to redistribute the bytes.
///
/// # Errors
///
/// Rejects an excessive, malformed, checksum-invalid, ambiguous or unsupported
/// font before returning semantic metadata.
pub fn inspect_opentype_static(bytes: &[u8], face_index: u32) -> Result<FontInspection, FontError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(FontError::ResourceLimit {
            resource: "encoded bytes",
            limit: MAX_FONT_BYTES,
            observed: bytes.len(),
        });
    }
    if face_index != 0 {
        return Err(FontError::Unsupported(
            "single-face profile requires face index zero",
        ));
    }
    let tables = inspect_directory(bytes, DirectoryProfile::Static)?;
    let font = FontRef::new(bytes).map_err(|error| FontError::Parse(error.to_string()))?;
    if !font.axes().is_empty() {
        return Err(FontError::Unsupported("variable font"));
    }
    inspect_font_metadata(bytes, &tables, &font, face_index, OPENTYPE_STATIC_PROFILE)
}

fn inspect_font_metadata(
    bytes: &[u8],
    tables: &[TableRecord],
    font: &FontRef<'_>,
    face_index: u32,
    decoder_profile: &str,
) -> Result<FontInspection, FontError> {
    let head = table_bytes(bytes, tables, *b"head")?;
    let maxp = table_bytes(bytes, tables, *b"maxp")?;
    let os2 = table_bytes(bytes, tables, *b"OS/2")?;
    let units_per_em = read_u16(head, 18)?;
    let glyph_count = read_u16(maxp, 4)?;
    let metrics = font.metrics(Size::unscaled(), LocationRef::default());
    if metrics.units_per_em != units_per_em || metrics.glyph_count != glyph_count {
        return Err(FontError::Parse(
            "sfnt fields and Skrifa metrics disagree".to_owned(),
        ));
    }
    let os2_version = read_u16(os2, 0)?;
    if os2_version > 5 {
        return Err(FontError::Unsupported("unknown OS/2 table version"));
    }
    let fs_type = read_u16(os2, 8)?;
    let permission = classify_fs_type(os2_version, fs_type)?;
    let mut unique_names = BTreeSet::new();
    for id in [StringId::FAMILY_NAME, StringId::TYPOGRAPHIC_FAMILY_NAME] {
        for name in font.localized_strings(id).take(MAX_FONT_NAMES + 1) {
            let name = name.to_string();
            if !name.is_empty() {
                unique_names.insert(name);
            }
            if unique_names.len() > MAX_FONT_NAMES {
                return Err(FontError::ResourceLimit {
                    resource: "family names",
                    limit: MAX_FONT_NAMES,
                    observed: unique_names.len(),
                });
            }
        }
    }
    let mut names = unique_names.into_iter().collect::<Vec<_>>();
    names.shrink_to_fit();
    let coverage = coverage_ranges(font)?;
    Ok(FontInspection {
        decoder_profile: decoder_profile.to_owned(),
        byte_length: bytes.len(),
        face_index,
        units_per_em,
        glyph_count,
        names,
        coverage,
        os2_version,
        fs_type,
        permission,
        subsetting_allowed: fs_type & 0x0100 == 0,
        outline_embedding_allowed: fs_type & 0x0200 == 0,
        table_tags: tables
            .iter()
            .map(|table| String::from_utf8_lossy(&table.tag).into_owned())
            .collect(),
    })
}

/// Inspects the bounded metadata and coordinate surface proposed by RFC 0013.
///
/// This function is research evidence only: it does not authorize package
/// acceptance, shaping, outline extraction or rendering of variable fonts.
///
/// # Errors
///
/// Rejects an excessive, malformed, collection, non-TrueType, color, bitmap,
/// SVG, VARC or unsupported-version input before returning axis metadata.
pub fn inspect_opentype_variable_metadata(
    bytes: &[u8],
    face_index: u32,
) -> Result<VariableFontInspection, FontError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(FontError::ResourceLimit {
            resource: "encoded bytes",
            limit: MAX_FONT_BYTES,
            observed: bytes.len(),
        });
    }
    if face_index != 0 {
        return Err(FontError::Unsupported(
            "single-face profile requires face index zero",
        ));
    }
    let tables = inspect_directory(bytes, DirectoryProfile::VariableMetadata)?;
    let parsed = parse_variable_metadata(bytes, &tables)?;
    let font = FontRef::new(bytes).map_err(|error| FontError::Parse(error.to_string()))?;
    let axes = font.axes();
    if axes.len() != parsed.axes.len() {
        return Err(FontError::Parse(
            "NUIF and Skrifa axis counts disagree".to_owned(),
        ));
    }
    for (index, expected) in parsed.axes.iter().enumerate() {
        let axis_metadata = axes
            .get(index)
            .ok_or_else(|| FontError::Parse("Skrifa axis is absent".to_owned()))?;
        let observed_tag = axis_metadata.tag().to_string();
        if observed_tag != expected.tag
            || axis_metadata.min_value().to_bits() != fixed_to_f32(expected.minimum_16_16).to_bits()
            || axis_metadata.default_value().to_bits()
                != fixed_to_f32(expected.default_16_16).to_bits()
            || axis_metadata.max_value().to_bits() != fixed_to_f32(expected.maximum_16_16).to_bits()
            || axis_metadata.is_hidden() != expected.hidden
            || axis_metadata.name_id().to_u16() != expected.name_id
        {
            return Err(FontError::Parse(
                "NUIF and Skrifa axis metadata disagree".to_owned(),
            ));
        }
    }
    if font.named_instances().len() != parsed.named_instance_count {
        return Err(FontError::Parse(
            "NUIF and Skrifa named-instance counts disagree".to_owned(),
        ));
    }
    Ok(VariableFontInspection {
        font: inspect_font_metadata(
            bytes,
            &tables,
            &font,
            face_index,
            OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        )?,
        axes: parsed.axes,
        named_instance_count: parsed.named_instance_count,
        avar_version: parsed.avar_version,
    })
}

/// Converts one complete user-coordinate tuple through the exact OpenType
/// 16.16 and final 2.14 normalization rules proposed by RFC 0013.
///
/// The NUIF-owned integer implementation is compared with Skrifa before the
/// ordered coordinate record is returned. Unlike general OpenType selection,
/// this research profile rejects omitted, unknown and out-of-range axes rather
/// than clamping or supplying defaults.
///
/// # Errors
///
/// Returns a typed error for invalid variable metadata, an incomplete tuple,
/// non-finite/out-of-range values or a normalization disagreement.
pub fn normalize_variable_coordinates(
    bytes: &[u8],
    coordinates: &BTreeMap<String, f64>,
) -> Result<Vec<VariableCoordinate>, FontError> {
    let tables = inspect_directory(bytes, DirectoryProfile::VariableMetadata)?;
    let parsed = parse_variable_metadata(bytes, &tables)?;
    if coordinates.len() != parsed.axes.len()
        || parsed
            .axes
            .iter()
            .any(|axis| !coordinates.contains_key(&axis.tag))
    {
        return Err(FontError::AssetMismatch(
            "variable profile requires one value for every and only every axis",
        ));
    }
    let mut result = Vec::with_capacity(parsed.axes.len());
    for (index, axis) in parsed.axes.iter().enumerate() {
        let value = coordinates[&axis.tag];
        let user_16_16 = float_to_fixed(value)?;
        if user_16_16 < axis.minimum_16_16 || user_16_16 > axis.maximum_16_16 {
            return Err(FontError::AssetMismatch(
                "variable coordinate is outside the declared axis range",
            ));
        }
        let normalized = normalize_axis(user_16_16, axis)?;
        let remapped = apply_avar(normalized, &parsed.avar_maps[index]);
        let normalized_2_14 = fixed_to_f2dot14(remapped);
        result.push(VariableCoordinate {
            tag: axis.tag.clone(),
            user_16_16,
            normalized_2_14,
        });
    }
    let font = FontRef::new(bytes).map_err(|error| FontError::Parse(error.to_string()))?;
    let settings = result
        .iter()
        .map(|coordinate| (coordinate.tag.as_str(), fixed_to_f32(coordinate.user_16_16)));
    let skrifa = font.axes().location(settings);
    let observed = skrifa
        .coords()
        .iter()
        .map(|coordinate| coordinate.to_bits())
        .collect::<Vec<_>>();
    if observed
        != result
            .iter()
            .map(|coordinate| coordinate.normalized_2_14)
            .collect::<Vec<_>>()
    {
        return Err(FontError::Parse(
            "NUIF and Skrifa coordinate normalization disagree".to_owned(),
        ));
    }
    Ok(result)
}

struct ParsedVariableMetadata {
    axes: Vec<VariableAxisInspection>,
    named_instance_count: usize,
    avar_version: Option<String>,
    avar_maps: Vec<Vec<(i32, i32)>>,
}

fn parse_variable_metadata(
    bytes: &[u8],
    tables: &[TableRecord],
) -> Result<ParsedVariableMetadata, FontError> {
    let fvar = table_bytes(bytes, tables, *b"fvar")?;
    if read_u16(fvar, 0)? != 1 || read_u16(fvar, 2)? != 0 || read_u16(fvar, 6)? != 2 {
        return Err(FontError::Unsupported("requires fvar version 1.0"));
    }
    let axes_offset = usize::from(read_u16(fvar, 4)?);
    let axis_count = usize::from(read_u16(fvar, 8)?);
    let axis_size = usize::from(read_u16(fvar, 10)?);
    let instance_count = usize::from(read_u16(fvar, 12)?);
    let instance_size = usize::from(read_u16(fvar, 14)?);
    if axis_count == 0 || axis_count > MAX_VARIABLE_AXES {
        return Err(FontError::ResourceLimit {
            resource: "variation axes",
            limit: MAX_VARIABLE_AXES,
            observed: axis_count,
        });
    }
    if instance_count > MAX_VARIABLE_INSTANCES {
        return Err(FontError::ResourceLimit {
            resource: "named instances",
            limit: MAX_VARIABLE_INSTANCES,
            observed: instance_count,
        });
    }
    if axes_offset < 16 || axis_size != 20 {
        return Err(FontError::Unsupported("noncanonical fvar axis records"));
    }
    let short_instance_size = axis_count.saturating_mul(4).saturating_add(4);
    if instance_size != short_instance_size && instance_size != short_instance_size + 2 {
        return Err(FontError::Unsupported("invalid fvar instance record size"));
    }
    let axes_end = axes_offset
        .checked_add(axis_count.saturating_mul(axis_size))
        .ok_or(FontError::Unsupported("fvar axis range overflow"))?;
    let instances_end = axes_end
        .checked_add(instance_count.saturating_mul(instance_size))
        .ok_or(FontError::Unsupported("fvar instance range overflow"))?;
    if instances_end != fvar.len() {
        return Err(FontError::Unsupported("noncanonical fvar table length"));
    }
    let mut tags = BTreeSet::new();
    let mut axes = Vec::with_capacity(axis_count);
    for index in 0..axis_count {
        let offset = axes_offset + index * axis_size;
        let tag_bytes: [u8; 4] = fvar[offset..offset + 4]
            .try_into()
            .map_err(|_| FontError::Unsupported("truncated fvar axis tag"))?;
        if !valid_variation_tag(tag_bytes) || !tags.insert(tag_bytes) {
            return Err(FontError::Unsupported(
                "invalid or duplicate variation axis tag",
            ));
        }
        let minimum_16_16 = read_i32(fvar, offset + 4)?;
        let default_16_16 = read_i32(fvar, offset + 8)?;
        let maximum_16_16 = read_i32(fvar, offset + 12)?;
        if minimum_16_16 >= maximum_16_16
            || default_16_16 < minimum_16_16
            || default_16_16 > maximum_16_16
        {
            return Err(FontError::Unsupported("invalid variation axis range"));
        }
        let flags = read_u16(fvar, offset + 16)?;
        let name_id = read_u16(fvar, offset + 18)?;
        if flags & !1 != 0 || !(256..32_768).contains(&name_id) {
            return Err(FontError::Unsupported(
                "invalid variation axis flags or name identifier",
            ));
        }
        axes.push(VariableAxisInspection {
            tag: String::from_utf8_lossy(&tag_bytes).into_owned(),
            minimum_16_16,
            default_16_16,
            maximum_16_16,
            hidden: flags & 1 != 0,
            name_id,
            avar_segments: 0,
        });
    }
    validate_named_instances(fvar, axes_end, instance_count, instance_size, &axes)?;
    let (avar_version, avar_maps) = parse_avar(bytes, tables, axis_count)?;
    for (axis, map) in axes.iter_mut().zip(&avar_maps) {
        axis.avar_segments = map.len();
    }
    Ok(ParsedVariableMetadata {
        axes,
        named_instance_count: instance_count,
        avar_version,
        avar_maps,
    })
}

fn validate_named_instances(
    fvar: &[u8],
    offset: usize,
    count: usize,
    size: usize,
    axes: &[VariableAxisInspection],
) -> Result<(), FontError> {
    let mut coordinates = BTreeSet::new();
    for index in 0..count {
        let start = offset + index * size;
        let name_id = read_u16(fvar, start)?;
        let flags = read_u16(fvar, start + 2)?;
        if flags != 0 || !valid_instance_name_id(name_id) {
            return Err(FontError::Unsupported(
                "invalid named-instance flags or name identifier",
            ));
        }
        let mut tuple = Vec::with_capacity(axes.len());
        for (axis_index, axis) in axes.iter().enumerate() {
            let value = read_i32(fvar, start + 4 + axis_index * 4)?;
            if value < axis.minimum_16_16 || value > axis.maximum_16_16 {
                return Err(FontError::Unsupported(
                    "named-instance coordinate outside axis range",
                ));
            }
            tuple.push(value);
        }
        if !coordinates.insert(tuple) {
            return Err(FontError::Unsupported(
                "duplicate named-instance coordinate tuple",
            ));
        }
        if size == axes.len() * 4 + 6 {
            let postscript_name_id = read_u16(fvar, start + size - 2)?;
            if postscript_name_id != 6
                && postscript_name_id != u16::MAX
                && !(256..32_768).contains(&postscript_name_id)
            {
                return Err(FontError::Unsupported(
                    "invalid named-instance PostScript name identifier",
                ));
            }
        }
    }
    Ok(())
}

fn valid_instance_name_id(value: u16) -> bool {
    matches!(value, 2 | 17) || (256..32_768).contains(&value)
}

fn valid_variation_tag(tag: [u8; 4]) -> bool {
    tag.iter().all(|byte| (0x21..=0x7e).contains(byte))
}

type AvarMaps = Vec<Vec<(i32, i32)>>;

fn parse_avar(
    bytes: &[u8],
    tables: &[TableRecord],
    axis_count: usize,
) -> Result<(Option<String>, AvarMaps), FontError> {
    let Some(table) = tables.iter().find(|table| table.tag == *b"avar") else {
        return Ok((None, vec![Vec::new(); axis_count]));
    };
    let avar = &bytes[table.offset..table.offset + table.length];
    let major = read_u16(avar, 0)?;
    let minor = read_u16(avar, 2)?;
    if major != 1 || minor != 0 || read_u16(avar, 4)? != 0 {
        return Err(FontError::Unsupported("requires avar version 1.0"));
    }
    if usize::from(read_u16(avar, 6)?) != axis_count {
        return Err(FontError::Unsupported("avar and fvar axis counts differ"));
    }
    let mut cursor = 8_usize;
    let mut maps = Vec::with_capacity(axis_count);
    for _ in 0..axis_count {
        let segment_count = usize::from(read_u16(avar, cursor)?);
        cursor = cursor.saturating_add(2);
        if segment_count > MAX_AVAR_SEGMENTS_PER_AXIS {
            return Err(FontError::ResourceLimit {
                resource: "avar segments per axis",
                limit: MAX_AVAR_SEGMENTS_PER_AXIS,
                observed: segment_count,
            });
        }
        if segment_count < 3 {
            return Err(FontError::Unsupported("incomplete avar axis map"));
        }
        let mut map = Vec::with_capacity(segment_count);
        for _ in 0..segment_count {
            let from = i32::from(read_i16(avar, cursor)?) << 2;
            let to = i32::from(read_i16(avar, cursor + 2)?) << 2;
            cursor = cursor.saturating_add(4);
            if !(-65_536..=65_536).contains(&from) || !(-65_536..=65_536).contains(&to) {
                return Err(FontError::Unsupported("avar coordinate outside [-1, 1]"));
            }
            if map.last().is_some_and(|(previous_from, previous_to)| {
                *previous_from >= from || *previous_to > to
            }) {
                return Err(FontError::Unsupported(
                    "avar coordinates are not monotonically increasing",
                ));
            }
            map.push((from, to));
        }
        if map.first() != Some(&(-65_536, -65_536))
            || map.last() != Some(&(65_536, 65_536))
            || !map.contains(&(0, 0))
        {
            return Err(FontError::Unsupported("avar map must preserve -1, 0 and 1"));
        }
        maps.push(map);
    }
    if cursor != avar.len() {
        return Err(FontError::Unsupported("noncanonical avar table length"));
    }
    Ok((Some("1.0".to_owned()), maps))
}

fn normalize_axis(value: i32, axis: &VariableAxisInspection) -> Result<i32, FontError> {
    let normalized = match value.cmp(&axis.default_16_16) {
        std::cmp::Ordering::Less => -fixed_div(
            axis.default_16_16.saturating_sub(value),
            axis.default_16_16.saturating_sub(axis.minimum_16_16),
        )?,
        std::cmp::Ordering::Greater => fixed_div(
            value.saturating_sub(axis.default_16_16),
            axis.maximum_16_16.saturating_sub(axis.default_16_16),
        )?,
        std::cmp::Ordering::Equal => 0,
    };
    Ok(normalized.clamp(-65_536, 65_536))
}

fn apply_avar(value: i32, map: &[(i32, i32)]) -> i32 {
    if map.is_empty() {
        return value;
    }
    if let Some((_, to)) = map.iter().find(|(from, _)| *from == value) {
        return *to;
    }
    let Some(window) = map
        .windows(2)
        .find(|window| window[0].0 < value && value < window[1].0)
    else {
        return value;
    };
    let (before_from, before_to) = window[0];
    let (after_from, after_to) = window[1];
    before_to.saturating_add(fixed_mul_div(
        after_to.saturating_sub(before_to),
        value.saturating_sub(before_from),
        after_from.saturating_sub(before_from),
    ))
}

fn fixed_div(numerator: i32, denominator: i32) -> Result<i32, FontError> {
    if denominator == 0 {
        return Err(FontError::Unsupported("zero-width variation axis segment"));
    }
    let negative = (numerator < 0) ^ (denominator < 0);
    let numerator = u64::from(numerator.unsigned_abs());
    let denominator = u64::from(denominator.unsigned_abs());
    let magnitude = ((numerator << 16) + (denominator >> 1)) / denominator;
    let magnitude = i32::try_from(magnitude)
        .map_err(|_| FontError::Unsupported("variation normalization overflow"))?;
    Ok(if negative { -magnitude } else { magnitude })
}

fn fixed_mul_div(value: i32, numerator: i32, denominator: i32) -> i32 {
    if denominator == 0 {
        return 0;
    }
    let negative = (value < 0) ^ (numerator < 0) ^ (denominator < 0);
    let product = u64::from(value.unsigned_abs()) * u64::from(numerator.unsigned_abs());
    let divisor = u64::from(denominator.unsigned_abs());
    let magnitude = (product + (divisor >> 1)) / divisor;
    let magnitude = i32::try_from(magnitude).unwrap_or(i32::MAX);
    if negative { -magnitude } else { magnitude }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the finite adjusted value is range-checked before conversion"
)]
fn float_to_fixed(value: f64) -> Result<i32, FontError> {
    let scaled = value * 65_536.0;
    let adjusted = scaled + if value.is_sign_positive() { 0.5 } else { -0.5 };
    if !adjusted.is_finite() || adjusted < f64::from(i32::MIN) || adjusted > f64::from(i32::MAX) {
        return Err(FontError::AssetMismatch(
            "variable coordinate is non-finite or outside 16.16",
        ));
    }
    Ok(adjusted as i32)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Skrifa accepts f32 user coordinates; exact 16.16 bits remain authoritative"
)]
fn fixed_to_f32(value: i32) -> f32 {
    value as f32 / 65_536.0
}

fn fixed_to_f2dot14(value: i32) -> i16 {
    i16::try_from((value.clamp(-65_536, 65_536) + 2) >> 2)
        .expect("clamped 2.14 coordinate fits i16")
}

/// Verifies that a packaged font asset exactly describes the inspected bytes
/// and carries an explicit human or organizational embedding review.
///
/// `license.embedding_review = approved` records a caller decision; this
/// function does not interpret licenses or grant redistribution rights.
///
/// # Errors
///
/// Returns parser, model mismatch or conservative policy errors.
pub fn validate_packaged_font(asset: &Asset, bytes: &[u8]) -> Result<FontInspection, FontError> {
    let AssetKind::Font(font) = &asset.kind else {
        return Err(FontError::AssetMismatch("asset kind is not font"));
    };
    let inspection = inspect_opentype_static(bytes, font.face_index)?;
    validate_asset_metadata(font, &inspection)?;
    if matches!(asset.portability, AssetPortability::Unavailable) {
        return Err(FontError::Policy(
            "an unavailable asset cannot be validated against resource bytes",
        ));
    }
    if inspection.permission == EmbeddingPermission::Restricted {
        return Err(FontError::Policy(
            "restricted fsType requires a different explicitly authorized profile",
        ));
    }
    if !inspection.outline_embedding_allowed {
        return Err(FontError::Policy(
            "bitmap-only embedding is incompatible with the outline profile",
        ));
    }
    require_policy(font, "font.decoder_profile", OPENTYPE_STATIC_PROFILE)?;
    require_policy(
        font,
        "opentype.fs_type",
        &format!("0x{:04x}", inspection.fs_type),
    )?;
    require_nonempty_policy(font, "license.expression")?;
    require_policy(font, "license.embedding_review", "approved")?;
    Ok(inspection)
}

fn validate_asset_metadata(font: &FontAsset, inspection: &FontInspection) -> Result<(), FontError> {
    if font.names.is_empty()
        || font
            .names
            .iter()
            .any(|name| !inspection.names.contains(name))
    {
        return Err(FontError::AssetMismatch("family names"));
    }
    if !font.axes.is_empty() {
        return Err(FontError::AssetMismatch("static profile has no axes"));
    }
    if font.coverage != inspection.coverage {
        return Err(FontError::AssetMismatch("Unicode coverage"));
    }
    if font.features.len() > MAX_FONT_FEATURES
        || font
            .features
            .keys()
            .any(|tag| tag.len() != 4 || !tag.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    {
        return Err(FontError::AssetMismatch("OpenType feature tags"));
    }
    Ok(())
}

fn require_policy(font: &FontAsset, key: &'static str, expected: &str) -> Result<(), FontError> {
    if font.policy_evidence.get(key).map(String::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(FontError::Policy(key))
    }
}

fn require_nonempty_policy(font: &FontAsset, key: &'static str) -> Result<(), FontError> {
    if font
        .policy_evidence
        .get(key)
        .is_some_and(|value| !value.trim().is_empty())
    {
        Ok(())
    } else {
        Err(FontError::Policy(key))
    }
}

/// Interprets only the OpenType `OS/2.fsType` bit contract. The result is
/// technical evidence, not a license decision.
///
/// # Errors
///
/// Rejects reserved bits, contradictory usage permissions and bitmap-only
/// evidence in versions that predate those flags.
pub fn classify_fs_type(version: u16, fs_type: u16) -> Result<EmbeddingPermission, FontError> {
    let version_mask = if version <= 1 { 0x000f } else { 0x030f };
    if fs_type & !version_mask != 0 || fs_type & 1 != 0 {
        return Err(FontError::Unsupported("reserved OS/2.fsType bits"));
    }
    let usage = fs_type & 0x000f;
    let permission = match usage {
        0 => EmbeddingPermission::Installable,
        2 => EmbeddingPermission::Restricted,
        4 => EmbeddingPermission::PreviewAndPrint,
        8 => EmbeddingPermission::Editable,
        _ => {
            return Err(FontError::Unsupported(
                "contradictory OS/2.fsType usage bits",
            ));
        }
    };
    Ok(permission)
}

#[derive(Clone, Copy, Debug)]
struct TableRecord {
    tag: [u8; 4],
    checksum: u32,
    offset: usize,
    length: usize,
}

#[derive(Clone, Copy)]
enum DirectoryProfile {
    Static,
    VariableMetadata,
}

fn inspect_directory(
    bytes: &[u8],
    profile: DirectoryProfile,
) -> Result<Vec<TableRecord>, FontError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(FontError::ResourceLimit {
            resource: "encoded bytes",
            limit: MAX_FONT_BYTES,
            observed: bytes.len(),
        });
    }
    if bytes.len() < 12 || bytes[..4] != SFNT_TRUETYPE {
        return Err(FontError::Unsupported("requires one TrueType-outline sfnt"));
    }
    let table_count = usize::from(read_u16(bytes, 4)?);
    if table_count == 0 || table_count > MAX_FONT_TABLES {
        return Err(FontError::ResourceLimit {
            resource: "table count",
            limit: MAX_FONT_TABLES,
            observed: table_count,
        });
    }
    let directory_end = 12_usize
        .checked_add(table_count.saturating_mul(16))
        .ok_or(FontError::Unsupported("table directory overflow"))?;
    if directory_end > bytes.len() {
        return Err(FontError::Unsupported("truncated table directory"));
    }
    let entry_selector = usize::try_from(table_count.ilog2())
        .map_err(|_| FontError::Unsupported("table search parameters"))?;
    let search_range = (1_usize << entry_selector).saturating_mul(16);
    let range_shift = table_count.saturating_mul(16).saturating_sub(search_range);
    if usize::from(read_u16(bytes, 6)?) != search_range
        || usize::from(read_u16(bytes, 8)?) != entry_selector
        || usize::from(read_u16(bytes, 10)?) != range_shift
    {
        return Err(FontError::Unsupported("invalid table search parameters"));
    }
    let mut tables = Vec::with_capacity(table_count);
    let mut tags = BTreeSet::new();
    for index in 0..table_count {
        let start = 12 + index * 16;
        let tag: [u8; 4] = bytes[start..start + 4]
            .try_into()
            .map_err(|_| FontError::Unsupported("truncated table tag"))?;
        if !tags.insert(tag) {
            return Err(FontError::Unsupported("duplicate table tag"));
        }
        if index > 0
            && tables
                .last()
                .is_some_and(|previous: &TableRecord| previous.tag >= tag)
        {
            return Err(FontError::Unsupported("table tags are not strictly sorted"));
        }
        let checksum = read_u32(bytes, start + 4)?;
        let offset = usize::try_from(read_u32(bytes, start + 8)?)
            .map_err(|_| FontError::Unsupported("table offset"))?;
        let length = usize::try_from(read_u32(bytes, start + 12)?)
            .map_err(|_| FontError::Unsupported("table length"))?;
        let end = offset
            .checked_add(length)
            .ok_or(FontError::Unsupported("table range overflow"))?;
        if offset % 4 != 0 || offset < directory_end || end > bytes.len() {
            return Err(FontError::Unsupported("invalid table range"));
        }
        tables.push(TableRecord {
            tag,
            checksum,
            offset,
            length,
        });
    }
    for required in REQUIRED_TABLES {
        if !tags.contains(&required) {
            return Err(FontError::Unsupported("missing required TrueType table"));
        }
    }
    validate_directory_profile(&tags, profile)?;
    validate_table_packing(bytes, &tables, directory_end)?;
    for table in &tables {
        let data = &bytes[table.offset..table.offset + table.length];
        let observed = table_checksum(data, table.tag == *b"head");
        if observed != table.checksum {
            return Err(FontError::TableChecksum {
                tag: String::from_utf8_lossy(&table.tag).into_owned(),
            });
        }
    }
    if table_checksum(bytes, false) != FONT_CHECKSUM_MAGIC {
        return Err(FontError::FontChecksum);
    }
    Ok(tables)
}

fn validate_directory_profile(
    tags: &BTreeSet<[u8; 4]>,
    profile: DirectoryProfile,
) -> Result<(), FontError> {
    match profile {
        DirectoryProfile::Static => {
            if REJECTED_TABLES.iter().any(|tag| tags.contains(tag)) {
                return Err(FontError::Unsupported(
                    "variable, CFF, color, bitmap or SVG table",
                ));
            }
        }
        DirectoryProfile::VariableMetadata => {
            if REQUIRED_VARIABLE_TABLES
                .iter()
                .any(|tag| !tags.contains(tag))
            {
                return Err(FontError::Unsupported(
                    "variable TrueType profile requires fvar, gvar and STAT",
                ));
            }
            if REJECTED_VARIABLE_TABLES
                .iter()
                .any(|tag| tags.contains(tag))
            {
                return Err(FontError::Unsupported(
                    "collection, CFF, color, bitmap, SVG or VARC table",
                ));
            }
        }
    }
    Ok(())
}

fn validate_table_packing(
    bytes: &[u8],
    tables: &[TableRecord],
    directory_end: usize,
) -> Result<(), FontError> {
    let mut ranges = tables
        .iter()
        .map(|table| (table.offset, table.offset + table.length))
        .collect::<Vec<_>>();
    ranges.sort_unstable_by_key(|range| range.0);
    let mut expected_offset = directory_end;
    for (offset, end) in ranges {
        if offset != expected_offset {
            return Err(FontError::Unsupported(
                "noncanonical table packing or padding",
            ));
        }
        let aligned_end = align4(end);
        let padding = bytes
            .get(end..aligned_end)
            .ok_or(FontError::Unsupported("trailing or missing table padding"))?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(FontError::Unsupported(
                "noncanonical table packing or padding",
            ));
        }
        expected_offset = aligned_end;
    }
    if expected_offset != bytes.len() {
        return Err(FontError::Unsupported("trailing or missing table padding"));
    }
    Ok(())
}

fn coverage_ranges(font: &FontRef<'_>) -> Result<Vec<CodepointRange>, FontError> {
    let mut codepoints = vec![false; 0x11_0000];
    for (codepoint, _) in font.charmap().mappings() {
        if let Some(slot) = usize::try_from(codepoint)
            .ok()
            .and_then(|index| codepoints.get_mut(index))
        {
            *slot = true;
        }
    }
    let mut ranges = Vec::new();
    let mut start = None;
    let mut previous = 0_u32;
    for (codepoint, covered) in codepoints.into_iter().enumerate() {
        let codepoint = u32::try_from(codepoint).expect("Unicode bitmap index fits u32");
        let covered = covered && char::from_u32(codepoint).is_some();
        match (start, covered) {
            (None, true) => {
                start = Some(codepoint);
                previous = codepoint;
            }
            (Some(_), true) if codepoint == previous + 1 => previous = codepoint,
            (Some(range_start), true) => {
                ranges.push(CodepointRange {
                    start: range_start,
                    end: previous,
                });
                start = Some(codepoint);
                previous = codepoint;
            }
            (Some(range_start), false) => {
                ranges.push(CodepointRange {
                    start: range_start,
                    end: previous,
                });
                start = None;
            }
            (None, false) => {}
        }
        if ranges.len() > MAX_FONT_COVERAGE_RANGES {
            return Err(FontError::ResourceLimit {
                resource: "coverage ranges",
                limit: MAX_FONT_COVERAGE_RANGES,
                observed: ranges.len(),
            });
        }
    }
    if let Some(range_start) = start {
        ranges.push(CodepointRange {
            start: range_start,
            end: previous,
        });
    }
    Ok(ranges)
}

fn table_bytes<'a>(
    bytes: &'a [u8],
    tables: &[TableRecord],
    tag: [u8; 4],
) -> Result<&'a [u8], FontError> {
    let table = tables
        .iter()
        .find(|table| table.tag == tag)
        .ok_or(FontError::Unsupported("required table absent"))?;
    Ok(&bytes[table.offset..table.offset + table.length])
}

fn table_checksum(bytes: &[u8], zero_head_adjustment: bool) -> u32 {
    let mut sum = 0_u32;
    for offset in (0..bytes.len()).step_by(4) {
        let mut word = [0_u8; 4];
        let end = (offset + 4).min(bytes.len());
        word[..end - offset].copy_from_slice(&bytes[offset..end]);
        if zero_head_adjustment && offset == 8 {
            word = [0; 4];
        }
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

fn align4(value: usize) -> usize {
    value.saturating_add(3) & !3
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, FontError> {
    let end = offset
        .checked_add(2)
        .ok_or(FontError::Unsupported("integer offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or(FontError::Unsupported("truncated integer"))?
        .try_into()
        .map_err(|_| FontError::Unsupported("truncated integer"))?;
    Ok(u16::from_be_bytes(value))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, FontError> {
    let end = offset
        .checked_add(4)
        .ok_or(FontError::Unsupported("integer offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or(FontError::Unsupported("truncated integer"))?
        .try_into()
        .map_err(|_| FontError::Unsupported("truncated integer"))?;
    Ok(u32::from_be_bytes(value))
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, FontError> {
    let end = offset
        .checked_add(2)
        .ok_or(FontError::Unsupported("integer offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or(FontError::Unsupported("truncated integer"))?
        .try_into()
        .map_err(|_| FontError::Unsupported("truncated integer"))?;
    Ok(i16::from_be_bytes(value))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, FontError> {
    let end = offset
        .checked_add(4)
        .ok_or(FontError::Unsupported("integer offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or(FontError::Unsupported("truncated integer"))?
        .try_into()
        .map_err(|_| FontError::Unsupported("truncated integer"))?;
    Ok(i32::from_be_bytes(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ahem_satisfies_the_static_profile() {
        let inspection = inspect_opentype_static(font_test_data::AHEM, 0).unwrap();
        assert_eq!(inspection.units_per_em, 1000);
        assert_eq!(inspection.fs_type, 0);
        assert_eq!(inspection.permission, EmbeddingPermission::Installable);
        assert!(inspection.names.iter().any(|name| name == "Ahem"));
        assert!(!inspection.coverage.is_empty());
    }

    #[test]
    fn real_static_truetype_corpus_satisfies_the_profile() {
        for (name, bytes) in [
            ("ahem", font_test_data::AHEM),
            ("tinos", font_test_data::TINOS_SUBSET),
            ("cousine", font_test_data::COUSINE_HINT_SUBSET),
            ("tthint", font_test_data::TTHINT_SUBSET),
        ] {
            let result = inspect_opentype_static(bytes, 0);
            assert!(result.is_ok(), "{name}: {:?}", result.err());
        }
    }

    #[test]
    fn malformed_and_out_of_profile_fonts_fail_closed() {
        assert!(inspect_opentype_static(&font_test_data::AHEM[..64], 0).is_err());
        assert!(inspect_opentype_static(font_test_data::AHEM, 1).is_err());
        let mut corrupt = font_test_data::AHEM.to_vec();
        corrupt[256] ^= 1;
        assert!(matches!(
            inspect_opentype_static(&corrupt, 0),
            Err(FontError::TableChecksum { .. } | FontError::FontChecksum)
        ));
    }

    #[test]
    fn fs_type_interpretation_is_conservative() {
        assert_eq!(
            classify_fs_type(3, 0).unwrap(),
            EmbeddingPermission::Installable
        );
        assert_eq!(
            classify_fs_type(3, 2).unwrap(),
            EmbeddingPermission::Restricted
        );
        assert!(classify_fs_type(3, 6).is_err());
        assert!(classify_fs_type(3, 1).is_err());
        assert!(classify_fs_type(1, 0x0100).is_err());
    }

    #[test]
    fn policy_requires_explicit_review_beyond_fs_type() {
        let inspection = inspect_opentype_static(font_test_data::AHEM, 0).unwrap();
        let font = FontAsset {
            face_index: 0,
            names: inspection.names.clone(),
            axes: BTreeMap::new(),
            features: BTreeMap::new(),
            coverage: inspection.coverage.clone(),
            policy_evidence: BTreeMap::from([
                (
                    "font.decoder_profile".to_owned(),
                    OPENTYPE_STATIC_PROFILE.to_owned(),
                ),
                ("opentype.fs_type".to_owned(), "0x0000".to_owned()),
                ("license.expression".to_owned(), "CC0-1.0".to_owned()),
            ]),
        };
        let asset = Asset {
            schema_version: nuif_core::CURRENT_SCHEMA_VERSION,
            id: nuif_core::AssetId::new(1),
            name: Some("Ahem".to_owned()),
            resource: None,
            portability: AssetPortability::Portable,
            kind: AssetKind::Font(font.clone()),
        };
        assert!(matches!(
            validate_packaged_font(&asset, font_test_data::AHEM),
            Err(FontError::Policy("license.embedding_review"))
        ));
        let mut reviewed = asset;
        let AssetKind::Font(font) = &mut reviewed.kind else {
            unreachable!();
        };
        font.policy_evidence
            .insert("license.embedding_review".to_owned(), "approved".to_owned());
        validate_packaged_font(&reviewed, font_test_data::AHEM).unwrap();
    }

    #[test]
    fn redistributed_variable_metadata_and_normalization_are_bounded() {
        let inspection =
            inspect_opentype_variable_metadata(font_test_data::MATERIAL_SYMBOLS_SUBSET, 0).unwrap();
        assert_eq!(
            inspection.font.decoder_profile,
            OPENTYPE_VARIABLE_TRUETYPE_PROFILE
        );
        assert!(!inspection.axes.is_empty());
        let defaults = inspection
            .axes
            .iter()
            .map(|axis| (axis.tag.clone(), f64::from(axis.default_16_16) / 65_536.0))
            .collect::<BTreeMap<_, _>>();
        let normalized =
            normalize_variable_coordinates(font_test_data::MATERIAL_SYMBOLS_SUBSET, &defaults)
                .unwrap();
        assert!(
            normalized
                .iter()
                .all(|coordinate| coordinate.normalized_2_14 == 0)
        );

        for selected_index in 0..inspection.axes.len() {
            for position in [0_u8, 1, 2, 3, 4] {
                let coordinates = inspection
                    .axes
                    .iter()
                    .enumerate()
                    .map(|(index, axis)| {
                        let fixed = if index == selected_index {
                            match position {
                                0 => axis.minimum_16_16,
                                1 => {
                                    axis.minimum_16_16
                                        + (axis.default_16_16 - axis.minimum_16_16) / 2
                                }
                                2 => axis.default_16_16,
                                3 => {
                                    axis.default_16_16
                                        + (axis.maximum_16_16 - axis.default_16_16) / 2
                                }
                                4 => axis.maximum_16_16,
                                _ => unreachable!(),
                            }
                        } else {
                            axis.default_16_16
                        };
                        (axis.tag.clone(), f64::from(fixed) / 65_536.0)
                    })
                    .collect::<BTreeMap<_, _>>();
                normalize_variable_coordinates(
                    font_test_data::MATERIAL_SYMBOLS_SUBSET,
                    &coordinates,
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn variable_coordinates_require_a_complete_in_range_tuple() {
        let inspection =
            inspect_opentype_variable_metadata(font_test_data::MATERIAL_SYMBOLS_SUBSET, 0).unwrap();
        assert!(
            normalize_variable_coordinates(
                font_test_data::MATERIAL_SYMBOLS_SUBSET,
                &BTreeMap::new()
            )
            .is_err()
        );
        let outside = inspection
            .axes
            .iter()
            .map(|axis| {
                (
                    axis.tag.clone(),
                    f64::from(axis.maximum_16_16) / 65_536.0 + 1.0,
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert!(
            normalize_variable_coordinates(font_test_data::MATERIAL_SYMBOLS_SUBSET, &outside)
                .is_err()
        );
        let defaults = inspection
            .axes
            .iter()
            .map(|axis| (axis.tag.clone(), f64::from(axis.default_16_16) / 65_536.0))
            .collect::<BTreeMap<_, _>>();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut coordinates = defaults.clone();
            let tag = inspection.axes[0].tag.clone();
            coordinates.insert(tag, invalid);
            assert!(
                normalize_variable_coordinates(
                    font_test_data::MATERIAL_SYMBOLS_SUBSET,
                    &coordinates
                )
                .is_err()
            );
        }
        let mut unknown = defaults;
        unknown.insert("ZZZZ".to_owned(), 0.0);
        assert!(
            normalize_variable_coordinates(font_test_data::MATERIAL_SYMBOLS_SUBSET, &unknown)
                .is_err()
        );
    }
}
