use nuif_font::{
    MAX_GVAR_EXPLICIT_DELTAS, MAX_GVAR_SHARED_TUPLES, MAX_GVAR_TUPLES, MAX_MVAR_VALUE_RECORDS,
    MAX_STAT_AXIS_VALUES, MAX_VARIATION_DATA_SUBTABLES, MAX_VARIATION_DELTA_SETS,
    MAX_VARIATION_REGION_REFERENCES, MAX_VARIATION_REGIONS, inspect_opentype_variable_metadata,
};
use serde_json::{Value, json};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use std::alloc::System;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const ROBOTO_FLEX: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/roboto-flex-mvar-subset/RobotoFlex-MVAR-subset.ttf"
);
const CHECKSUM_MAGIC: u32 = 0xb1b0_afba;
const MAX_INSPECTION_ALLOCATED_BYTES: usize = 8 * 1024 * 1024;
const MAX_INSPECTION_RETAINED_BYTES: usize = 2 * 1024 * 1024;
const MAX_INSPECTION_MICROSECONDS: u128 = 500_000;
const MAX_EARLY_REJECTION_ALLOCATED_BYTES: usize = 256 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-security: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let material = font_test_data::MATERIAL_SYMBOLS_SUBSET;
    let hvar = font_test_data::HVAR_WITH_TRUNCATED_ADVANCE_INDEX_MAP;
    for bytes in [material, hvar, ROBOTO_FLEX] {
        inspect_opentype_variable_metadata(bytes, 0).map_err(|error| error.to_string())?;
    }

    let hostile_trials = hostile_trials(material, hvar)?;
    let allocation_trials = allocation_trials(material, hvar)?;
    let passed = hostile_trials
        .iter()
        .chain(&allocation_trials)
        .all(passed_trial);
    let report = report(&hostile_trials, &allocation_trials, passed);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "variable font security: {} hostile, {} allocation trials, status {}",
        hostile_trials.len(),
        allocation_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn hostile_trials(material: &[u8], hvar: &[u8]) -> Result<Vec<Value>, String> {
    let mut trials = gvar_trials(material)?;
    trials.extend(hvar_trials(hvar)?);
    trials.extend(mvar_trials()?);
    trials.extend(stat_trials()?);
    Ok(trials)
}

fn gvar_trials(material: &[u8]) -> Result<Vec<Value>, String> {
    Ok(vec![
        rejection_trial("gvar_unknown_version", material, |bytes| {
            write_table(bytes, *b"gvar", 0, &2_u16.to_be_bytes())
        })?,
        rejection_trial("gvar_axis_count_mismatch", material, |bytes| {
            write_table(bytes, *b"gvar", 4, &1_u16.to_be_bytes())
        })?,
        rejection_trial("gvar_glyph_count_mismatch", material, |bytes| {
            write_table(bytes, *b"gvar", 12, &u16::MAX.to_be_bytes())
        })?,
        rejection_trial("gvar_reserved_flags", material, |bytes| {
            write_table(bytes, *b"gvar", 14, &2_u16.to_be_bytes())
        })?,
        rejection_trial("gvar_first_offset_nonzero", material, |bytes| {
            write_table(bytes, *b"gvar", 20, &1_u16.to_be_bytes())
        })?,
        rejection_trial(
            "gvar_nonempty_data_without_tuples",
            material,
            zero_first_gvar_tuple_count,
        )?,
        rejection_trial(
            "gvar_tuple_count_reserved_flag",
            material,
            mutate_gvar_tuple_count_reserved,
        )?,
        rejection_trial(
            "gvar_serialized_data_overlaps_headers",
            material,
            mutate_gvar_data_offset,
        )?,
        rejection_trial(
            "gvar_tuple_index_reserved_flag",
            material,
            mutate_gvar_tuple_index_reserved,
        )?,
        rejection_trial(
            "gvar_shared_tuple_out_of_range",
            material,
            mutate_gvar_shared_tuple,
        )?,
        rejection_trial("gvar_tuple_data_size_zero", material, mutate_gvar_data_size)?,
        rejection_trial(
            "gvar_packed_point_run_exceeds_count",
            material,
            mutate_gvar_point_run,
        )?,
        rejection_trial(
            "gvar_packed_point_out_of_range",
            material,
            mutate_gvar_point_value,
        )?,
        rejection_trial(
            "gvar_unsupported_32_bit_delta_run",
            material,
            mutate_gvar_delta_to_i32,
        )?,
        rejection_trial(
            "gvar_packed_delta_run_exceeds_axis",
            material,
            mutate_gvar_delta_run,
        )?,
    ])
}

fn hvar_trials(hvar: &[u8]) -> Result<Vec<Value>, String> {
    Ok(vec![
        rejection_trial("hvar_unknown_version", hvar, |bytes| {
            write_table(bytes, *b"HVAR", 0, &2_u16.to_be_bytes())
        })?,
        rejection_trial("hvar_store_offset_out_of_range", hvar, |bytes| {
            write_table(bytes, *b"HVAR", 4, &u32::MAX.to_be_bytes())
        })?,
        rejection_trial("hvar_region_axis_count_mismatch", hvar, |bytes| {
            mutate_store_region(bytes, *b"HVAR", 2_u16.to_be_bytes(), 0)
        })?,
        rejection_trial("hvar_region_count_reserved_bit", hvar, |bytes| {
            mutate_store_region(bytes, *b"HVAR", 0x8000_u16.to_be_bytes(), 2)
        })?,
        rejection_trial("hvar_data_offset_out_of_range", hvar, |bytes| {
            mutate_store_data_offset(bytes, *b"HVAR")
        })?,
        rejection_trial("hvar_word_count_exceeds_regions", hvar, |bytes| {
            mutate_first_item_word_count(bytes, *b"HVAR", 0x7fff)
        })?,
        rejection_trial("hvar_mapping_reserved_bits", hvar, mutate_hvar_entry_format)?,
    ])
}

fn mvar_trials() -> Result<Vec<Value>, String> {
    Ok(vec![
        rejection_trial("mvar_unknown_version", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"MVAR", 0, &2_u16.to_be_bytes())
        })?,
        rejection_trial("mvar_reserved_field_nonzero", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"MVAR", 4, &1_u16.to_be_bytes())
        })?,
        rejection_trial("mvar_record_size_invalid", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"MVAR", 6, &7_u16.to_be_bytes())
        })?,
        rejection_trial("mvar_store_offset_null", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"MVAR", 10, &0_u16.to_be_bytes())
        })?,
        rejection_trial("mvar_duplicate_value_tag", ROBOTO_FLEX, duplicate_mvar_tag)?,
        rejection_trial("mvar_delta_set_outer_index_absent", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"MVAR", 16, &0xfffe_u16.to_be_bytes())
        })?,
        rejection_trial("mvar_region_axis_count_mismatch", ROBOTO_FLEX, |bytes| {
            mutate_store_region(bytes, *b"MVAR", 12_u16.to_be_bytes(), 0)
        })?,
        rejection_trial("mvar_long_words_forbidden", ROBOTO_FLEX, |bytes| {
            mutate_first_item_word_count(bytes, *b"MVAR", 0x8000)
        })?,
    ])
}

fn stat_trials() -> Result<Vec<Value>, String> {
    Ok(vec![
        rejection_trial("stat_unknown_major_version", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"STAT", 0, &2_u16.to_be_bytes())
        })?,
        rejection_trial("stat_design_axis_size_invalid", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"STAT", 4, &7_u16.to_be_bytes())
        })?,
        rejection_trial("stat_fewer_axes_than_fvar", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"STAT", 6, &12_u16.to_be_bytes())
        })?,
        rejection_trial(
            "stat_duplicate_design_axis",
            ROBOTO_FLEX,
            duplicate_stat_axis,
        )?,
        rejection_trial("stat_axis_value_count_one_over", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"STAT", 12, &4097_u16.to_be_bytes())
        })?,
        rejection_trial("stat_axis_values_offset_null", ROBOTO_FLEX, |bytes| {
            write_table(bytes, *b"STAT", 14, &0_u32.to_be_bytes())
        })?,
        rejection_trial(
            "stat_axis_value_reserved_flags",
            ROBOTO_FLEX,
            mutate_stat_value_flags,
        )?,
    ])
}

fn report(hostile_trials: &[Value], allocation_trials: &[Value], passed: bool) -> Value {
    json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-graph-security-baseline",
        "status": if passed { "passed" } else { "failed" },
        "limits": {
            "gvar_shared_tuples": MAX_GVAR_SHARED_TUPLES,
            "gvar_tuples": MAX_GVAR_TUPLES,
            "gvar_explicit_deltas": MAX_GVAR_EXPLICIT_DELTAS,
            "variation_regions": MAX_VARIATION_REGIONS,
            "variation_data_subtables": MAX_VARIATION_DATA_SUBTABLES,
            "variation_delta_sets": MAX_VARIATION_DELTA_SETS,
            "variation_region_references": MAX_VARIATION_REGION_REFERENCES,
            "mvar_value_records": MAX_MVAR_VALUE_RECORDS,
            "stat_axis_values": MAX_STAT_AXIS_VALUES,
            "inspection_allocated_bytes": MAX_INSPECTION_ALLOCATED_BYTES,
            "inspection_retained_bytes": MAX_INSPECTION_RETAINED_BYTES,
            "inspection_microseconds": MAX_INSPECTION_MICROSECONDS,
            "early_rejection_allocated_bytes": MAX_EARLY_REJECTION_ALLOCATED_BYTES,
        },
        "hostile_trials": hostile_trials,
        "allocation_trials": allocation_trials,
        "summary": {
            "hostile": hostile_trials.len(),
            "allocation": allocation_trials.len(),
            "blocking_failures": hostile_trials.iter().chain(allocation_trials).filter(|item| !passed_trial(item)).count(),
        },
        "non_claims": [
            "checksum-repaired mutations cover representative packed gvar point and delta failures but not every encoding combination",
            "ceilings are reference-implementation regressions after one warmup rather than portable format semantics",
            "VVAR remains rejected by capability boundary rather than validated as a vertical-text profile",
            "successful research preflight does not enable variable package layout or rendering acceptance",
        ],
    })
}

fn rejection_trial(
    name: &str,
    source: &[u8],
    mutate: impl FnOnce(&mut [u8]) -> Result<(), String>,
) -> Result<Value, String> {
    let mut bytes = source.to_vec();
    mutate(&mut bytes)?;
    repair_checksums(&mut bytes)?;
    let error = inspect_opentype_variable_metadata(&bytes, 0)
        .err()
        .map_or_else(
            || "mutation was accepted".to_owned(),
            |error| error.to_string(),
        );
    Ok(json!({
        "name": name,
        "status": if error != "mutation was accepted" && !error.contains("checksum") { "passed" } else { "failed" },
        "error": error,
        "input_bytes": bytes.len(),
        "checksums_repaired_after_mutation": true,
    }))
}

fn allocation_trials(material: &[u8], hvar: &[u8]) -> Result<Vec<Value>, String> {
    let fixtures = [
        ("material_symbols", material),
        ("truncated_hvar_map", hvar),
        ("roboto_flex_mvar", ROBOTO_FLEX),
    ];
    let mut trials = Vec::new();
    for (name, bytes) in fixtures {
        drop(inspect_opentype_variable_metadata(bytes, 0).map_err(|error| error.to_string())?);
        let region = Region::new(GLOBAL);
        let started = Instant::now();
        let inspection =
            inspect_opentype_variable_metadata(bytes, 0).map_err(|error| error.to_string())?;
        let micros = started.elapsed().as_micros();
        let stats = region.change();
        let retained = retained_bytes(stats);
        trials.push(json!({
            "name": format!("{name}_bounded_inspection"),
            "status": if stats.bytes_allocated <= MAX_INSPECTION_ALLOCATED_BYTES && retained <= MAX_INSPECTION_RETAINED_BYTES && micros <= MAX_INSPECTION_MICROSECONDS { "passed" } else { "failed" },
            "input_bytes": bytes.len(),
            "allocated_bytes": stats.bytes_allocated,
            "retained_bytes": retained,
            "elapsed_microseconds": micros,
            "variation_graph": inspection.variation_graph,
        }));
    }
    let mut early = material.to_vec();
    write_table(&mut early, *b"gvar", 0, &2_u16.to_be_bytes())?;
    repair_checksums(&mut early)?;
    let region = Region::new(GLOBAL);
    let rejected = inspect_opentype_variable_metadata(&early, 0).is_err();
    let stats = region.change();
    trials.push(json!({
        "name": "malformed_graph_rejected_before_large_allocation",
        "status": if rejected && stats.bytes_allocated <= MAX_EARLY_REJECTION_ALLOCATED_BYTES { "passed" } else { "failed" },
        "allocated_bytes": stats.bytes_allocated,
        "allocated_budget": MAX_EARLY_REJECTION_ALLOCATED_BYTES,
    }));
    Ok(trials)
}

fn zero_first_gvar_tuple_count(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"gvar")?;
    let flags = read_u16_at(bytes, start + 14)?;
    let glyph_count = usize::from(read_u16_at(bytes, start + 12)?);
    let data_start = start
        + usize::try_from(read_u32_at(bytes, start + 16)?)
            .map_err(|_| "gvar data offset does not fit usize")?;
    let entry_size = if flags & 1 == 0 { 2 } else { 4 };
    for index in 0..glyph_count {
        let left = gvar_offset(bytes, start + 20 + index * entry_size, entry_size)?;
        let right = gvar_offset(bytes, start + 20 + (index + 1) * entry_size, entry_size)?;
        if right > left {
            let tuple_offset = data_start + left;
            let count = read_u16_at(bytes, tuple_offset)? & 0xf000;
            write_absolute(bytes, tuple_offset, &count.to_be_bytes())?;
            return Ok(());
        }
    }
    Err("gvar fixture has no nonempty glyph data".to_owned())
}

fn gvar_offset(bytes: &[u8], offset: usize, size: usize) -> Result<usize, String> {
    if size == 2 {
        Ok(usize::from(read_u16_at(bytes, offset)?) * 2)
    } else {
        usize::try_from(read_u32_at(bytes, offset)?)
            .map_err(|_| "gvar offset does not fit usize".to_owned())
    }
}

struct GvarGlyphLocation {
    start: usize,
    axis_count: usize,
}

struct PackedPointLocation {
    count: usize,
    byte_len: usize,
    first_control: usize,
    first_value: usize,
}

struct PrivateTupleLocation {
    point_control: usize,
    point_value: usize,
    delta_control: usize,
}

fn mutate_gvar_tuple_count_reserved(bytes: &mut [u8]) -> Result<(), String> {
    let glyph = first_gvar_glyph(bytes)?;
    let count = read_u16_at(bytes, glyph.start)? | 0x1000;
    write_absolute(bytes, glyph.start, &count.to_be_bytes())
}

fn mutate_gvar_data_offset(bytes: &mut [u8]) -> Result<(), String> {
    let glyph = first_gvar_glyph(bytes)?;
    write_absolute(bytes, glyph.start + 2, &4_u16.to_be_bytes())
}

fn mutate_gvar_tuple_index_reserved(bytes: &mut [u8]) -> Result<(), String> {
    let glyph = first_gvar_glyph(bytes)?;
    let tuple_index = read_u16_at(bytes, glyph.start + 6)? | 0x1000;
    write_absolute(bytes, glyph.start + 6, &tuple_index.to_be_bytes())
}

fn mutate_gvar_shared_tuple(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"gvar")?;
    if read_u16_at(bytes, start + 6)? == 0 {
        return Err("gvar fixture has no shared tuples".to_owned());
    }
    let tuple = start
        + usize::try_from(read_u32_at(bytes, start + 8)?)
            .map_err(|_| "gvar shared-tuple offset does not fit usize")?;
    write_absolute(bytes, tuple, &i16::MAX.to_be_bytes())
}

fn mutate_gvar_data_size(bytes: &mut [u8]) -> Result<(), String> {
    let glyph = first_gvar_glyph(bytes)?;
    write_absolute(bytes, glyph.start + 4, &0_u16.to_be_bytes())
}

fn mutate_gvar_point_run(bytes: &mut [u8]) -> Result<(), String> {
    let location = first_private_gvar_tuple(bytes)?;
    let control = *bytes
        .get(location.point_control)
        .ok_or_else(|| "gvar point control is out of range".to_owned())?
        & 0x80
        | 0x7f;
    write_absolute(bytes, location.point_control, &[control])
}

fn mutate_gvar_point_value(bytes: &mut [u8]) -> Result<(), String> {
    let location = first_private_gvar_tuple(bytes)?;
    let control = *bytes
        .get(location.point_control)
        .ok_or_else(|| "gvar point control is out of range".to_owned())?
        | 0x80;
    write_absolute(bytes, location.point_control, &[control])?;
    write_absolute(bytes, location.point_value, &u16::MAX.to_be_bytes())
}

fn mutate_gvar_delta_to_i32(bytes: &mut [u8]) -> Result<(), String> {
    let location = first_private_gvar_tuple(bytes)?;
    let control = *bytes
        .get(location.delta_control)
        .ok_or_else(|| "gvar delta control is out of range".to_owned())?
        | 0xc0;
    write_absolute(bytes, location.delta_control, &[control])
}

fn mutate_gvar_delta_run(bytes: &mut [u8]) -> Result<(), String> {
    let location = first_private_gvar_tuple(bytes)?;
    let control = *bytes
        .get(location.delta_control)
        .ok_or_else(|| "gvar delta control is out of range".to_owned())?
        & 0xc0
        | 0x3f;
    write_absolute(bytes, location.delta_control, &[control])
}

fn first_gvar_glyph(bytes: &[u8]) -> Result<GvarGlyphLocation, String> {
    let (start, _) = table_range(bytes, *b"gvar")?;
    let flags = read_u16_at(bytes, start + 14)?;
    let glyph_count = usize::from(read_u16_at(bytes, start + 12)?);
    let data_start = start
        + usize::try_from(read_u32_at(bytes, start + 16)?)
            .map_err(|_| "gvar data offset does not fit usize")?;
    let entry_size = if flags & 1 == 0 { 2 } else { 4 };
    for index in 0..glyph_count {
        let left = gvar_offset(bytes, start + 20 + index * entry_size, entry_size)?;
        let right = gvar_offset(bytes, start + 20 + (index + 1) * entry_size, entry_size)?;
        if right > left {
            return Ok(GvarGlyphLocation {
                start: data_start + left,
                axis_count: usize::from(read_u16_at(bytes, start + 4)?),
            });
        }
    }
    Err("gvar fixture has no nonempty glyph data".to_owned())
}

fn first_private_gvar_tuple(bytes: &[u8]) -> Result<PrivateTupleLocation, String> {
    let glyph = first_gvar_glyph(bytes)?;
    let raw_count = read_u16_at(bytes, glyph.start)?;
    let tuple_count = usize::from(raw_count & 0x0fff);
    let mut header_cursor = glyph.start + 4;
    let mut headers = Vec::with_capacity(tuple_count);
    for _ in 0..tuple_count {
        let size = usize::from(read_u16_at(bytes, header_cursor)?);
        let index = read_u16_at(bytes, header_cursor + 2)?;
        headers.push((size, index & 0x2000 != 0));
        header_cursor += 4;
        if index & 0x8000 != 0 {
            header_cursor += glyph.axis_count * 2;
        }
        if index & 0x4000 != 0 {
            header_cursor += glyph.axis_count * 4;
        }
    }
    let mut data_cursor = glyph.start + usize::from(read_u16_at(bytes, glyph.start + 2)?);
    if raw_count & 0x8000 != 0 {
        data_cursor += packed_point_location(bytes, data_cursor)?.byte_len;
    }
    for (size, private) in headers {
        if private {
            let points = packed_point_location(bytes, data_cursor)?;
            if (1..=63).contains(&points.count) {
                return Ok(PrivateTupleLocation {
                    point_control: points.first_control,
                    point_value: points.first_value,
                    delta_control: data_cursor + points.byte_len,
                });
            }
        }
        data_cursor += size;
    }
    Err("gvar fixture has no small private point tuple".to_owned())
}

fn packed_point_location(bytes: &[u8], start: usize) -> Result<PackedPointLocation, String> {
    let first = *bytes
        .get(start)
        .ok_or_else(|| "truncated packed-point count".to_owned())?;
    let (count, mut cursor) = if first == 0 {
        return Err("packed-point set represents all points".to_owned());
    } else if first & 0x80 == 0 {
        (usize::from(first), start + 1)
    } else {
        (usize::from(read_u16_at(bytes, start)? & 0x7fff), start + 2)
    };
    let first_control = cursor;
    let first_value = cursor + 1;
    let mut seen = 0_usize;
    while seen < count {
        let control = *bytes
            .get(cursor)
            .ok_or_else(|| "truncated packed-point run".to_owned())?;
        cursor += 1;
        let run = usize::from(control & 0x7f) + 1;
        let width = if control & 0x80 == 0 { 1 } else { 2 };
        cursor = cursor
            .checked_add(run.saturating_mul(width))
            .ok_or_else(|| "packed-point location overflow".to_owned())?;
        bytes
            .get(..cursor)
            .ok_or_else(|| "truncated packed-point values".to_owned())?;
        seen = seen.saturating_add(run);
    }
    Ok(PackedPointLocation {
        count,
        byte_len: cursor - start,
        first_control,
        first_value,
    })
}

fn mutate_store_region(
    bytes: &mut [u8],
    tag: [u8; 4],
    value: [u8; 2],
    relative: usize,
) -> Result<(), String> {
    let store = variation_store_offset(bytes, tag)?;
    let regions = store
        + usize::try_from(read_u32_at(bytes, store + 2)?)
            .map_err(|_| "region-list offset does not fit usize")?;
    write_absolute(bytes, regions + relative, &value)
}

fn mutate_store_data_offset(bytes: &mut [u8], tag: [u8; 4]) -> Result<(), String> {
    let store = variation_store_offset(bytes, tag)?;
    if read_u16_at(bytes, store + 6)? == 0 {
        return Err("fixture item variation store has no data offsets".to_owned());
    }
    write_absolute(bytes, store + 8, &u32::MAX.to_be_bytes())
}

fn mutate_first_item_word_count(
    bytes: &mut [u8],
    tag: [u8; 4],
    word_count: u16,
) -> Result<(), String> {
    let store = variation_store_offset(bytes, tag)?;
    let first = store
        + usize::try_from(read_u32_at(bytes, store + 8)?)
            .map_err(|_| "item-data offset does not fit usize")?;
    write_absolute(bytes, first + 2, &word_count.to_be_bytes())
}

fn variation_store_offset(bytes: &[u8], tag: [u8; 4]) -> Result<usize, String> {
    let (start, _) = table_range(bytes, tag)?;
    let relative = if tag == *b"MVAR" {
        u32::from(read_u16_at(bytes, start + 10)?)
    } else {
        read_u32_at(bytes, start + 4)?
    };
    Ok(start
        + usize::try_from(relative).map_err(|_| "variation-store offset does not fit usize")?)
}

fn mutate_hvar_entry_format(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"HVAR")?;
    let relative = usize::try_from(read_u32_at(bytes, start + 8)?)
        .map_err(|_| "HVAR map offset does not fit usize")?;
    if relative == 0 {
        return Err("fixture HVAR advance map is absent".to_owned());
    }
    write_absolute(bytes, start + relative + 1, &[0xc0])
}

fn duplicate_mvar_tag(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"MVAR")?;
    if read_u16_at(bytes, start + 8)? < 2 {
        return Err("fixture MVAR has fewer than two value records".to_owned());
    }
    let tag = bytes
        .get(start + 12..start + 16)
        .ok_or_else(|| "truncated MVAR value tag".to_owned())?
        .to_vec();
    write_absolute(bytes, start + 20, &tag)
}

fn duplicate_stat_axis(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"STAT")?;
    if read_u16_at(bytes, start + 6)? < 2 {
        return Err("fixture STAT has fewer than two design axes".to_owned());
    }
    let axes = start
        + usize::try_from(read_u32_at(bytes, start + 8)?)
            .map_err(|_| "STAT axes offset does not fit usize")?;
    let tag = bytes
        .get(axes..axes + 4)
        .ok_or_else(|| "truncated STAT design axis".to_owned())?
        .to_vec();
    write_absolute(bytes, axes + 8, &tag)
}

fn mutate_stat_value_flags(bytes: &mut [u8]) -> Result<(), String> {
    let (start, _) = table_range(bytes, *b"STAT")?;
    let values = start
        + usize::try_from(read_u32_at(bytes, start + 14)?)
            .map_err(|_| "STAT values offset does not fit usize")?;
    let first = values + usize::from(read_u16_at(bytes, values)?);
    read_u16_at(bytes, first)?;
    write_absolute(bytes, first + 4, &4_u16.to_be_bytes())
}

fn write_table(
    bytes: &mut [u8],
    tag: [u8; 4],
    relative: usize,
    value: &[u8],
) -> Result<(), String> {
    let (start, length) = table_range(bytes, tag)?;
    if relative.saturating_add(value.len()) > length {
        return Err(format!(
            "mutation leaves {} table",
            String::from_utf8_lossy(&tag)
        ));
    }
    write_absolute(bytes, start + relative, value)
}

fn write_absolute(bytes: &mut [u8], offset: usize, value: &[u8]) -> Result<(), String> {
    let target = bytes
        .get_mut(offset..offset.saturating_add(value.len()))
        .ok_or_else(|| "mutation offset is outside the font".to_owned())?;
    target.copy_from_slice(value);
    Ok(())
}

fn table_range(bytes: &[u8], tag: [u8; 4]) -> Result<(usize, usize), String> {
    let record = directory_record(bytes, tag)?;
    let offset = usize::try_from(read_u32_at(bytes, record + 8)?)
        .map_err(|_| "table offset does not fit usize")?;
    let length = usize::try_from(read_u32_at(bytes, record + 12)?)
        .map_err(|_| "table length does not fit usize")?;
    Ok((offset, length))
}

fn directory_record(bytes: &[u8], tag: [u8; 4]) -> Result<usize, String> {
    let count = usize::from(read_u16_at(bytes, 4)?);
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(tag.as_slice()))
        .ok_or_else(|| format!("{} table is absent", String::from_utf8_lossy(&tag)))
}

fn repair_checksums(bytes: &mut [u8]) -> Result<(), String> {
    let (head, _) = table_range(bytes, *b"head")?;
    write_absolute(bytes, head + 8, &0_u32.to_be_bytes())?;
    let count = usize::from(read_u16_at(bytes, 4)?);
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = bytes
            .get(record..record + 4)
            .ok_or_else(|| "truncated directory tag".to_owned())?
            .try_into()
            .map_err(|_| "invalid directory tag")?;
        let (start, length) = table_range(bytes, tag)?;
        let checksum = checksum(
            bytes
                .get(start..start + length)
                .ok_or_else(|| "truncated table during checksum repair".to_owned())?,
            tag == *b"head",
        );
        write_absolute(bytes, record + 4, &checksum.to_be_bytes())?;
    }
    let adjustment = CHECKSUM_MAGIC.wrapping_sub(checksum(bytes, false));
    write_absolute(bytes, head + 8, &adjustment.to_be_bytes())
}

fn checksum(bytes: &[u8], zero_head_adjustment: bool) -> u32 {
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

fn read_u16_at(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "truncated u16".to_owned())?
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| "truncated u16".to_owned())
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "truncated u32".to_owned())?
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| "truncated u32".to_owned())
}

fn retained_bytes(stats: Stats) -> usize {
    let retained = i128::try_from(stats.bytes_allocated).unwrap_or(i128::MAX)
        - i128::try_from(stats.bytes_deallocated).unwrap_or(i128::MAX)
        + i128::try_from(stats.bytes_reallocated).unwrap_or(i128::MAX);
    usize::try_from(retained.max(0)).unwrap_or(usize::MAX)
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-security-report.json");
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                output = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            "--help" | "-h" => {
                return Err("usage: variable-font-security [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
