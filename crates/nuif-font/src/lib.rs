#![doc = "Bounded, policy-explicit OpenType resource profiles."]

use nuif_core::{Asset, AssetKind, AssetPortability, CodepointRange, FontAsset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;
use ttf_parser::{Face, Permissions, name_id};

pub const OPENTYPE_STATIC_PROFILE: &str = "nuif-opentype-static-single-0";
pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FONT_TABLES: usize = 256;
pub const MAX_FONT_NAMES: usize = 256;
pub const MAX_FONT_COVERAGE_RANGES: usize = 65_536;
pub const MAX_FONT_FEATURES: usize = 64;

const SFNT_TRUETYPE: [u8; 4] = [0, 1, 0, 0];
const FONT_CHECKSUM_MAGIC: u32 = 0xb1b0_afba;
const REQUIRED_TABLES: [[u8; 4]; 9] = [
    *b"OS/2", *b"cmap", *b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp", *b"name",
];
const REJECTED_TABLES: [[u8; 4]; 11] = [
    *b"CFF ", *b"CFF2", *b"COLR", *b"CPAL", *b"CBDT", *b"CBLC", *b"EBDT", *b"EBLC", *b"SVG ",
    *b"fvar", *b"sbix",
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

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum FontError {
    #[error("font resource limit exceeded for {resource}: limit {limit}, observed {observed}")]
    ResourceLimit {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    #[error("font does not satisfy {OPENTYPE_STATIC_PROFILE}: {0}")]
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
    let tables = inspect_directory(bytes)?;
    let face =
        Face::parse(bytes, face_index).map_err(|error| FontError::Parse(error.to_string()))?;
    if face.is_variable() {
        return Err(FontError::Unsupported("variable font"));
    }
    let os2 = table_bytes(bytes, &tables, *b"OS/2")?;
    let os2_version = read_u16(os2, 0)?;
    if os2_version > 5 {
        return Err(FontError::Unsupported("unknown OS/2 table version"));
    }
    let fs_type = read_u16(os2, 8)?;
    let permission = classify_fs_type(os2_version, fs_type)?;
    let parser_permission = face
        .permissions()
        .ok_or(FontError::Unsupported("invalid OS/2 embedding permission"))?;
    if permission != permission_from_parser(parser_permission) {
        return Err(FontError::Parse(
            "independent fsType interpretations disagree".to_owned(),
        ));
    }
    let mut names = face
        .names()
        .into_iter()
        .filter(|name| matches!(name.name_id, name_id::FAMILY | name_id::TYPOGRAPHIC_FAMILY))
        .filter_map(|name| name.to_string())
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if names.len() > MAX_FONT_NAMES {
        return Err(FontError::ResourceLimit {
            resource: "family names",
            limit: MAX_FONT_NAMES,
            observed: names.len(),
        });
    }
    names.shrink_to_fit();
    let coverage = coverage_ranges(&face)?;
    Ok(FontInspection {
        decoder_profile: OPENTYPE_STATIC_PROFILE.to_owned(),
        byte_length: bytes.len(),
        face_index,
        units_per_em: face.units_per_em(),
        glyph_count: face.number_of_glyphs(),
        names,
        coverage,
        os2_version,
        fs_type,
        permission,
        subsetting_allowed: face.is_subsetting_allowed(),
        outline_embedding_allowed: face.is_outline_embedding_allowed(),
        table_tags: tables
            .iter()
            .map(|table| String::from_utf8_lossy(&table.tag).into_owned())
            .collect(),
    })
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

fn permission_from_parser(permission: Permissions) -> EmbeddingPermission {
    match permission {
        Permissions::Installable => EmbeddingPermission::Installable,
        Permissions::Restricted => EmbeddingPermission::Restricted,
        Permissions::PreviewAndPrint => EmbeddingPermission::PreviewAndPrint,
        Permissions::Editable => EmbeddingPermission::Editable,
    }
}

#[derive(Clone, Copy, Debug)]
struct TableRecord {
    tag: [u8; 4],
    checksum: u32,
    offset: usize,
    length: usize,
}

fn inspect_directory(bytes: &[u8]) -> Result<Vec<TableRecord>, FontError> {
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
    if REJECTED_TABLES.iter().any(|tag| tags.contains(tag)) {
        return Err(FontError::Unsupported(
            "variable, CFF, color, bitmap or SVG table",
        ));
    }
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

fn coverage_ranges(face: &Face<'_>) -> Result<Vec<CodepointRange>, FontError> {
    let mut codepoints = vec![false; 0x11_0000];
    let cmap = face
        .tables()
        .cmap
        .ok_or(FontError::Unsupported("missing parsed cmap"))?;
    for subtable in cmap.subtables {
        if subtable.is_unicode() {
            subtable.codepoints(|codepoint| {
                if let Some(slot) = usize::try_from(codepoint)
                    .ok()
                    .and_then(|index| codepoints.get_mut(index))
                    && subtable.glyph_index(codepoint).is_some()
                {
                    *slot = true;
                }
            });
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
}
