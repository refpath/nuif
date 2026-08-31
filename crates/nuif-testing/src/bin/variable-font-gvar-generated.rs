use nuif_font::inspect_opentype_variable_metadata;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const SOURCE: &[u8] = font_test_data::MATERIAL_SYMBOLS_SUBSET;
const CHECKSUM_MAGIC: u32 = 0xb1b0_afba;
const SYNTHETIC_POINTS: usize = 300;
const MAX_GENERATED_FONT_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy)]
enum DeltaKind {
    Zero,
    Byte,
    Word,
    Mixed,
}

#[derive(Clone, Copy)]
enum PointKind {
    Minimal,
    Words,
    Alternating,
}

#[derive(Clone)]
struct Points {
    values: Vec<u16>,
    kind: PointKind,
}

#[derive(Clone)]
struct TuplePlan {
    private_points: Option<Points>,
    x: DeltaKind,
    y: DeltaKind,
}

struct Case {
    name: &'static str,
    shared_points: Option<Points>,
    tuples: Vec<TuplePlan>,
}

#[derive(Clone)]
struct Table {
    tag: [u8; 4],
    data: Vec<u8>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-gvar-generated: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let (axis_count, glyph_count, loca_is_long) = source_shape(SOURCE)?;
    let target_count = SYNTHETIC_POINTS + 4;
    let cases = valid_cases();
    let valid_trials = cases
        .iter()
        .map(|case| valid_trial(case, axis_count, glyph_count, loca_is_long, target_count))
        .collect::<Result<Vec<_>, _>>()?;
    let invalid_trials = invalid_trials(axis_count, glyph_count, loca_is_long)?;
    let passed = valid_trials.iter().chain(&invalid_trials).all(passed_trial);
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-gvar-generated",
        "status": if passed { "passed" } else { "failed" },
        "source": {
            "fixture": "font-test-data 0.9.1 material_symbols_subset.ttf",
            "sha256": sha256(SOURCE),
            "package_license": "MIT OR Apache-2.0",
        },
        "synthetic_glyph": {
            "outline_points": SYNTHETIC_POINTS,
            "phantom_points": 4,
            "target_count": target_count,
            "axis_count": axis_count,
            "glyph_count": glyph_count,
        },
        "valid_trials": valid_trials,
        "invalid_trials": invalid_trials,
        "limits": {
            "maximum_generated_font_bytes": MAX_GENERATED_FONT_BYTES,
            "maximum_packed_point_count": 32767,
            "point_run_maximum": 128,
            "delta_run_maximum": 64,
        },
        "summary": {
            "valid": cases.len(),
            "invalid": 3,
            "blocking_failures": valid_trials.iter().chain(&invalid_trials).filter(|trial| !passed_trial(trial)).count(),
        },
        "non_claims": [
            "generated encodings exercise production whole-font admission but not every possible byte sequence",
            "the synthetic degenerate outline is parser evidence rather than rendering or typography evidence",
            "the independent encoder and production validator share the OpenType specification as their semantic source",
            "the generated fonts are ephemeral test values and are not distributable fixture assets",
        ],
    });
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
        "generated gvar: {} valid, {} invalid trials, status {}",
        valid_trials.len(),
        invalid_trials.len(),
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn valid_cases() -> Vec<Case> {
    let mut cases = basic_cases();
    cases.extend(count_cases());
    cases.extend(point_run_cases());
    cases.extend(shared_and_tuple_cases());
    cases
}

fn basic_cases() -> Vec<Case> {
    vec![
        case(
            "all_points_zero_byte",
            None,
            vec![all_tuple(DeltaKind::Zero, DeltaKind::Byte)],
        ),
        case(
            "all_points_word_mixed",
            None,
            vec![all_tuple(DeltaKind::Word, DeltaKind::Mixed)],
        ),
        case(
            "private_all_points",
            None,
            vec![private_tuple(
                Vec::new(),
                PointKind::Minimal,
                DeltaKind::Mixed,
                DeltaKind::Zero,
            )],
        ),
        case(
            "private_single_point",
            None,
            vec![private_tuple(
                vec![0],
                PointKind::Minimal,
                DeltaKind::Byte,
                DeltaKind::Word,
            )],
        ),
        case(
            "private_repeated_points",
            None,
            vec![private_tuple(
                vec![0, 0, 1, 1],
                PointKind::Minimal,
                DeltaKind::Mixed,
                DeltaKind::Byte,
            )],
        ),
    ]
}

fn count_cases() -> Vec<Case> {
    vec![
        case(
            "private_127_point_count",
            None,
            vec![private_tuple(
                sequential_points(127),
                PointKind::Minimal,
                DeltaKind::Zero,
                DeltaKind::Byte,
            )],
        ),
        case(
            "private_128_point_count",
            None,
            vec![private_tuple(
                sequential_points(128),
                PointKind::Minimal,
                DeltaKind::Byte,
                DeltaKind::Word,
            )],
        ),
        case(
            "private_256_point_runs",
            None,
            vec![private_tuple(
                sequential_points(256),
                PointKind::Minimal,
                DeltaKind::Mixed,
                DeltaKind::Zero,
            )],
        ),
        case(
            "maximum_repeated_point_count",
            None,
            vec![private_tuple(
                vec![0; 32_767],
                PointKind::Minimal,
                DeltaKind::Zero,
                DeltaKind::Zero,
            )],
        ),
    ]
}

fn point_run_cases() -> Vec<Case> {
    vec![
        case(
            "private_word_point_delta",
            None,
            vec![private_tuple(
                vec![0, 300, 303],
                PointKind::Minimal,
                DeltaKind::Word,
                DeltaKind::Byte,
            )],
        ),
        case(
            "private_forced_word_points",
            None,
            vec![private_tuple(
                vec![0, 1, 2, 3],
                PointKind::Words,
                DeltaKind::Byte,
                DeltaKind::Mixed,
            )],
        ),
        case(
            "private_alternating_point_runs",
            None,
            vec![private_tuple(
                vec![0, 1, 2, 3, 4, 5],
                PointKind::Alternating,
                DeltaKind::Mixed,
                DeltaKind::Word,
            )],
        ),
    ]
}

fn shared_and_tuple_cases() -> Vec<Case> {
    vec![
        case(
            "shared_all_points",
            Some(Points {
                values: Vec::new(),
                kind: PointKind::Minimal,
            }),
            vec![all_tuple(DeltaKind::Byte, DeltaKind::Zero)],
        ),
        case(
            "shared_sparse_points",
            Some(Points {
                values: vec![0, 17, 300, 303],
                kind: PointKind::Alternating,
            }),
            vec![all_tuple(DeltaKind::Mixed, DeltaKind::Word)],
        ),
        case(
            "private_overrides_shared",
            Some(Points {
                values: vec![0, 303],
                kind: PointKind::Minimal,
            }),
            vec![private_tuple(
                vec![0, 1, 2],
                PointKind::Minimal,
                DeltaKind::Byte,
                DeltaKind::Zero,
            )],
        ),
        case(
            "two_tuple_cursor",
            None,
            vec![
                private_tuple(
                    vec![0, 5, 11],
                    PointKind::Minimal,
                    DeltaKind::Byte,
                    DeltaKind::Word,
                ),
                all_tuple(DeltaKind::Mixed, DeltaKind::Zero),
            ],
        ),
    ]
}

fn private_tuple(values: Vec<u16>, kind: PointKind, x: DeltaKind, y: DeltaKind) -> TuplePlan {
    TuplePlan {
        private_points: Some(Points { values, kind }),
        x,
        y,
    }
}

fn all_tuple(x: DeltaKind, y: DeltaKind) -> TuplePlan {
    TuplePlan {
        private_points: None,
        x,
        y,
    }
}

fn sequential_points(count: u16) -> Vec<u16> {
    (0..count).collect()
}

fn case(name: &'static str, shared_points: Option<Points>, tuples: Vec<TuplePlan>) -> Case {
    Case {
        name,
        shared_points,
        tuples,
    }
}

fn valid_trial(
    case: &Case,
    axis_count: usize,
    glyph_count: usize,
    loca_is_long: bool,
    target_count: usize,
) -> Result<Value, String> {
    let glyf = synthetic_glyf(SYNTHETIC_POINTS)?;
    let loca = synthetic_loca(glyph_count, glyf.len(), loca_is_long)?;
    let gvar = build_gvar(axis_count, glyph_count, target_count, case)?;
    let replacements = BTreeMap::from([(*b"glyf", glyf), (*b"loca", loca), (*b"gvar", gvar)]);
    let first = rebuild_font(SOURCE, &replacements)?;
    let second = rebuild_font(SOURCE, &replacements)?;
    let expected_deltas = case
        .tuples
        .iter()
        .map(|tuple| {
            point_count(
                tuple.private_points.as_ref(),
                case.shared_points.as_ref(),
                target_count,
            )
        })
        .sum::<usize>();
    let inspection = inspect_opentype_variable_metadata(&first, 0)
        .map_err(|error| format!("{} was rejected: {error}", case.name))?;
    let graph = &inspection.variation_graph;
    let passed = first == second
        && first.len() <= MAX_GENERATED_FONT_BYTES
        && graph.gvar_glyph_data_count == 1
        && graph.gvar_tuple_count == case.tuples.len()
        && graph.gvar_explicit_delta_count == expected_deltas;
    Ok(json!({
        "name": case.name,
        "status": if passed { "passed" } else { "failed" },
        "font_bytes": first.len(),
        "font_sha256": sha256(&first),
        "tuples": case.tuples.len(),
        "shared_points": case.shared_points.as_ref().map(|points| encoded_point_count(points, target_count)),
        "explicit_delta_count": expected_deltas,
        "deterministic_rebuild": first == second,
    }))
}

fn invalid_trials(
    axis_count: usize,
    glyph_count: usize,
    loca_is_long: bool,
) -> Result<Vec<Value>, String> {
    let cases = [
        (
            "noncanonical_two_byte_count",
            vec![0x80, 0x01, 0, 0],
            "non-canonical",
        ),
        ("two_byte_count_zero", vec![0x80, 0x00], "non-canonical"),
        ("truncated_two_byte_count", vec![0x80], "truncated"),
    ];
    cases
        .into_iter()
        .map(|(name, point_bytes, expected)| {
            invalid_trial(
                name,
                point_bytes,
                expected,
                axis_count,
                glyph_count,
                loca_is_long,
            )
        })
        .collect()
}

fn invalid_trial(
    name: &str,
    point_bytes: Vec<u8>,
    expected_error: &str,
    axis_count: usize,
    glyph_count: usize,
    loca_is_long: bool,
) -> Result<Value, String> {
    let glyf = synthetic_glyf(SYNTHETIC_POINTS)?;
    let loca = synthetic_loca(glyph_count, glyf.len(), loca_is_long)?;
    let gvar = build_raw_private_gvar(axis_count, glyph_count, point_bytes)?;
    let font = rebuild_font(
        SOURCE,
        &BTreeMap::from([(*b"glyf", glyf), (*b"loca", loca), (*b"gvar", gvar)]),
    )?;
    let error = inspect_opentype_variable_metadata(&font, 0)
        .err()
        .map_or_else(
            || "generated invalid font was accepted".to_owned(),
            |error| error.to_string(),
        );
    Ok(json!({
        "name": name,
        "status": if error.contains(expected_error) { "passed" } else { "failed" },
        "font_bytes": font.len(),
        "expected_error": expected_error,
        "error": error,
    }))
}

fn build_gvar(
    axis_count: usize,
    glyph_count: usize,
    target_count: usize,
    case: &Case,
) -> Result<Vec<u8>, String> {
    let shared = case.shared_points.as_ref().map(encode_points).transpose()?;
    let mut headers = Vec::new();
    let mut tuple_data = Vec::new();
    for (index, tuple) in case.tuples.iter().enumerate() {
        let points = tuple
            .private_points
            .as_ref()
            .map(encode_points)
            .transpose()?;
        let count = point_count(
            tuple.private_points.as_ref(),
            case.shared_points.as_ref(),
            target_count,
        );
        let mut data = points.unwrap_or_default();
        data.extend(encode_deltas(count, tuple.x));
        data.extend(encode_deltas(count, tuple.y));
        push_u16(
            &mut headers,
            u16::try_from(data.len()).map_err(|_| "tuple data exceeds Offset16")?,
        );
        push_u16(
            &mut headers,
            0x8000
                | if tuple.private_points.is_some() {
                    0x2000
                } else {
                    0
                },
        );
        for axis in 0..axis_count {
            let value = if axis == 0 {
                if index % 2 == 0 { 8192 } else { -8192 }
            } else {
                0
            };
            push_i16(&mut headers, value);
        }
        tuple_data.extend(data);
    }
    let mut glyph_data = Vec::new();
    let raw_count = u16::try_from(case.tuples.len()).map_err(|_| "too many generated tuples")?
        | if shared.is_some() { 0x8000 } else { 0 };
    push_u16(&mut glyph_data, raw_count);
    push_u16(
        &mut glyph_data,
        u16::try_from(4 + headers.len()).map_err(|_| "tuple headers exceed Offset16")?,
    );
    glyph_data.extend(headers);
    glyph_data.extend(shared.unwrap_or_default());
    glyph_data.extend(tuple_data);
    build_gvar_table(axis_count, glyph_count, glyph_data)
}

fn build_raw_private_gvar(
    axis_count: usize,
    glyph_count: usize,
    point_bytes: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let header_size = 8 + axis_count * 2;
    let mut glyph_data = Vec::new();
    push_u16(&mut glyph_data, 1);
    push_u16(
        &mut glyph_data,
        u16::try_from(header_size).map_err(|_| "header overflow")?,
    );
    push_u16(
        &mut glyph_data,
        u16::try_from(point_bytes.len()).map_err(|_| "raw data overflow")?,
    );
    push_u16(&mut glyph_data, 0xa000);
    for axis in 0..axis_count {
        push_i16(&mut glyph_data, if axis == 0 { 8192 } else { 0 });
    }
    glyph_data.extend(point_bytes);
    build_gvar_table(axis_count, glyph_count, glyph_data)
}

fn build_gvar_table(
    axis_count: usize,
    glyph_count: usize,
    glyph_data: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let data_offset = 20_usize
        .checked_add((glyph_count + 1) * 4)
        .ok_or_else(|| "gvar offset array overflow".to_owned())?;
    let mut table = Vec::with_capacity(data_offset + glyph_data.len());
    push_u16(&mut table, 1);
    push_u16(&mut table, 0);
    push_u16(
        &mut table,
        u16::try_from(axis_count).map_err(|_| "axis count overflow")?,
    );
    push_u16(&mut table, 0);
    push_u32(
        &mut table,
        u32::try_from(data_offset).map_err(|_| "gvar offset overflow")?,
    );
    push_u16(
        &mut table,
        u16::try_from(glyph_count).map_err(|_| "glyph count overflow")?,
    );
    push_u16(&mut table, 1);
    push_u32(
        &mut table,
        u32::try_from(data_offset).map_err(|_| "gvar data overflow")?,
    );
    push_u32(&mut table, 0);
    let end = u32::try_from(glyph_data.len()).map_err(|_| "glyph data overflow")?;
    for _ in 1..=glyph_count {
        push_u32(&mut table, end);
    }
    table.extend(glyph_data);
    Ok(table)
}

fn encode_points(points: &Points) -> Result<Vec<u8>, String> {
    if points.values.is_empty() {
        return Ok(vec![0]);
    }
    if points.values.len() > 32_767 {
        return Err("packed point count exceeds 15 bits".to_owned());
    }
    let mut output = Vec::new();
    if points.values.len() <= 127 {
        output.push(u8::try_from(points.values.len()).expect("point count fits one byte"));
    } else {
        push_u16(
            &mut output,
            0x8000 | u16::try_from(points.values.len()).expect("point count fits 15 bits"),
        );
    }
    let mut previous = 0_u16;
    let deltas = points
        .values
        .iter()
        .map(|point| {
            let delta = point
                .checked_sub(previous)
                .ok_or_else(|| "point list decreases".to_owned())?;
            previous = *point;
            Ok(delta)
        })
        .collect::<Result<Vec<_>, String>>()?;
    encode_point_runs(&mut output, &deltas, points.kind);
    Ok(output)
}

fn encode_point_runs(output: &mut Vec<u8>, deltas: &[u16], kind: PointKind) {
    let mut index = 0;
    let mut alternating_word = false;
    while index < deltas.len() {
        let word = match kind {
            PointKind::Minimal => deltas[index] > u16::from(u8::MAX),
            PointKind::Words => true,
            PointKind::Alternating => {
                alternating_word = !alternating_word;
                alternating_word || deltas[index] > u16::from(u8::MAX)
            }
        };
        let mut run = 1;
        while index + run < deltas.len() && run < 128 {
            let next_word = match kind {
                PointKind::Minimal => deltas[index + run] > u16::from(u8::MAX),
                PointKind::Words => true,
                PointKind::Alternating => !word,
            };
            if next_word != word {
                break;
            }
            run += 1;
        }
        output.push(
            (if word { 0x80 } else { 0 })
                | u8::try_from(run - 1).expect("point run is at most 128"),
        );
        for delta in &deltas[index..index + run] {
            if word {
                push_u16(output, *delta);
            } else {
                output.push(u8::try_from(*delta).expect("byte point delta was preflighted"));
            }
        }
        index += run;
    }
}

fn encode_deltas(count: usize, kind: DeltaKind) -> Vec<u8> {
    let mut output = Vec::new();
    let mut remaining = count;
    let mut run_index = 0_usize;
    while remaining > 0 {
        let run = remaining.min(64);
        let selected = match kind {
            DeltaKind::Mixed => match run_index % 3 {
                0 => DeltaKind::Zero,
                1 => DeltaKind::Byte,
                _ => DeltaKind::Word,
            },
            other => other,
        };
        output.push(
            match selected {
                DeltaKind::Zero => 0x80,
                DeltaKind::Byte => 0,
                DeltaKind::Word => 0x40,
                DeltaKind::Mixed => unreachable!(),
            } | u8::try_from(run - 1).expect("delta run is at most 64"),
        );
        match selected {
            DeltaKind::Zero => {}
            DeltaKind::Byte => {
                output.extend(
                    (0..run).map(|index| [i8::MIN, -1, 0, 1, i8::MAX][index % 5].cast_unsigned()),
                );
            }
            DeltaKind::Word => {
                for index in 0..run {
                    push_i16(&mut output, [i16::MIN, -256, 256, i16::MAX][index % 4]);
                }
            }
            DeltaKind::Mixed => unreachable!(),
        }
        remaining -= run;
        run_index += 1;
    }
    output
}

fn point_count(private: Option<&Points>, shared: Option<&Points>, target: usize) -> usize {
    private
        .or(shared)
        .map_or(target, |points| encoded_point_count(points, target))
}

fn encoded_point_count(points: &Points, target: usize) -> usize {
    if points.values.is_empty() {
        target
    } else {
        points.values.len()
    }
}

fn synthetic_glyf(point_count: usize) -> Result<Vec<u8>, String> {
    if point_count == 0 || point_count > usize::from(u16::MAX) {
        return Err("synthetic point count is outside glyf bounds".to_owned());
    }
    let mut glyph = Vec::new();
    push_i16(&mut glyph, 1);
    for _ in 0..4 {
        push_i16(&mut glyph, 0);
    }
    push_u16(
        &mut glyph,
        u16::try_from(point_count - 1).map_err(|_| "point count overflow")?,
    );
    push_u16(&mut glyph, 0);
    let mut remaining = point_count;
    while remaining > 0 {
        let run = remaining.min(256);
        if run == 1 {
            glyph.push(0x31);
        } else {
            glyph.push(0x39);
            glyph.push(u8::try_from(run - 1).expect("glyf flag run is at most 256"));
        }
        remaining -= run;
    }
    if glyph.len() % 2 != 0 {
        glyph.push(0);
    }
    Ok(glyph)
}

fn synthetic_loca(glyph_count: usize, glyph_bytes: usize, long: bool) -> Result<Vec<u8>, String> {
    let mut loca = Vec::new();
    if long {
        push_u32(&mut loca, 0);
        let end = u32::try_from(glyph_bytes).map_err(|_| "glyf length overflow")?;
        for _ in 0..glyph_count {
            push_u32(&mut loca, end);
        }
    } else {
        if !glyph_bytes.is_multiple_of(2) {
            return Err("short loca requires an even glyf length".to_owned());
        }
        push_u16(&mut loca, 0);
        let end = u16::try_from(glyph_bytes / 2).map_err(|_| "short loca overflow")?;
        for _ in 0..glyph_count {
            push_u16(&mut loca, end);
        }
    }
    Ok(loca)
}

fn source_shape(bytes: &[u8]) -> Result<(usize, usize, bool), String> {
    let fvar = table_bytes(bytes, *b"fvar")?;
    let maxp = table_bytes(bytes, *b"maxp")?;
    let head = table_bytes(bytes, *b"head")?;
    Ok((
        usize::from(read_u16(fvar, 8)?),
        usize::from(read_u16(maxp, 4)?),
        read_i16(head, 50)? == 1,
    ))
}

fn rebuild_font(
    source: &[u8],
    replacements: &BTreeMap<[u8; 4], Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let mut tables = read_tables(source)?;
    for table in &mut tables {
        if let Some(replacement) = replacements.get(&table.tag) {
            table.data.clone_from(replacement);
        }
        if table.tag == *b"head" {
            write_u32_at(&mut table.data, 8, 0)?;
        }
    }
    for tag in replacements.keys() {
        if !tables.iter().any(|table| table.tag == *tag) {
            return Err(format!(
                "replacement table {} is absent",
                String::from_utf8_lossy(tag)
            ));
        }
    }
    tables.sort_unstable_by_key(|table| table.tag);
    build_sfnt(
        source
            .get(..4)
            .ok_or_else(|| "truncated sfnt version".to_owned())?,
        &tables,
    )
}

fn build_sfnt(version: &[u8], tables: &[Table]) -> Result<Vec<u8>, String> {
    let count = u16::try_from(tables.len()).map_err(|_| "table count overflow")?;
    let power = 1_u16 << count.ilog2();
    let search_range = power * 16;
    let mut output = Vec::new();
    output.extend(version);
    push_u16(&mut output, count);
    push_u16(&mut output, search_range);
    push_u16(
        &mut output,
        u16::try_from(power.ilog2()).expect("sfnt selector fits u16"),
    );
    push_u16(&mut output, count * 16 - search_range);
    let directory_start = output.len();
    output.resize(directory_start + tables.len() * 16, 0);
    let mut head_offset = None;
    for (index, table) in tables.iter().enumerate() {
        let offset = output.len();
        output.extend(&table.data);
        while output.len() % 4 != 0 {
            output.push(0);
        }
        let record = directory_start + index * 16;
        output[record..record + 4].copy_from_slice(&table.tag);
        write_u32_at(
            &mut output,
            record + 4,
            checksum(&table.data, table.tag == *b"head"),
        )?;
        write_u32_at(
            &mut output,
            record + 8,
            u32::try_from(offset).map_err(|_| "table offset overflow")?,
        )?;
        write_u32_at(
            &mut output,
            record + 12,
            u32::try_from(table.data.len()).map_err(|_| "table length overflow")?,
        )?;
        if table.tag == *b"head" {
            head_offset = Some(offset);
        }
    }
    let adjustment = CHECKSUM_MAGIC.wrapping_sub(checksum(&output, false));
    write_u32_at(
        &mut output,
        head_offset.ok_or_else(|| "head table is absent".to_owned())? + 8,
        adjustment,
    )?;
    Ok(output)
}

fn read_tables(bytes: &[u8]) -> Result<Vec<Table>, String> {
    let count = usize::from(read_u16(bytes, 4)?);
    (0..count)
        .map(|index| {
            let record = 12 + index * 16;
            let tag = bytes
                .get(record..record + 4)
                .ok_or_else(|| "truncated table tag".to_owned())?
                .try_into()
                .map_err(|_| "invalid table tag")?;
            let offset = usize::try_from(read_u32(bytes, record + 8)?)
                .map_err(|_| "table offset overflow")?;
            let length = usize::try_from(read_u32(bytes, record + 12)?)
                .map_err(|_| "table length overflow")?;
            let data = bytes
                .get(offset..offset.saturating_add(length))
                .ok_or_else(|| "truncated table".to_owned())?
                .to_vec();
            Ok(Table { tag, data })
        })
        .collect()
}

fn table_bytes(bytes: &[u8], tag: [u8; 4]) -> Result<&[u8], String> {
    read_tables(bytes)?;
    let count = usize::from(read_u16(bytes, 4)?);
    for index in 0..count {
        let record = 12 + index * 16;
        if bytes.get(record..record + 4) == Some(tag.as_slice()) {
            let offset = usize::try_from(read_u32(bytes, record + 8)?)
                .map_err(|_| "table offset overflow")?;
            let length = usize::try_from(read_u32(bytes, record + 12)?)
                .map_err(|_| "table length overflow")?;
            return bytes
                .get(offset..offset.saturating_add(length))
                .ok_or_else(|| "truncated table".to_owned());
        }
    }
    Err(format!("{} table is absent", String::from_utf8_lossy(&tag)))
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

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "truncated u16".to_owned())?
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| "truncated u16".to_owned())
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, String> {
    read_u16(bytes, offset).map(|value| i16::from_be_bytes(value.to_be_bytes()))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "truncated u32".to_owned())?
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| "truncated u32".to_owned())
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend(value.to_be_bytes());
}

fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.extend(value.to_be_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_be_bytes());
}

fn write_u32_at(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), String> {
    let target = bytes
        .get_mut(offset..offset + 4)
        .ok_or_else(|| "u32 write is out of bounds".to_owned())?;
    target.copy_from_slice(&value.to_be_bytes());
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-gvar-generated-report.json");
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
                return Err("usage: variable-font-gvar-generated [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
