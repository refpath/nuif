use nuif_core::{Asset, AssetId, AssetKind, AssetPortability, FontAsset, ResourceDigest};
use nuif_font::{
    OPENTYPE_VARIABLE_TRUETYPE_PROFILE, VariableFontInspection, inspect_opentype_variable_metadata,
    normalize_variable_coordinates, validate_variable_font_asset_candidate,
};
use nuif_text::{ShapeRequest, TextDirection, VariableGlobalMetrics, VariableResourceTrial};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const NOTO_BYTES: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/noto-sans-variable-subset/NotoSans-variable-subset.ttf"
);
const NOTO_LICENSE: &str =
    include_str!("../../../../conformance/font/fixtures/noto-sans-variable-subset/OFL.txt");
const NOTO_GOLDEN: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-noto-sans-variable.json");
const RECURSIVE_BYTES: &[u8] = include_bytes!(
    "../../../../conformance/font/fixtures/recursive-variable-subset/Recursive-variable-subset.ttf"
);
const RECURSIVE_LICENSE: &str =
    include_str!("../../../../conformance/font/fixtures/recursive-variable-subset/OFL.txt");
const RECURSIVE_GOLDEN: &str =
    include_str!("../../../../conformance/font/harfbuzz-14.4.0-recursive-variable.json");

struct Fixture {
    key: &'static str,
    bytes: &'static [u8],
    sha256: &'static str,
    family: &'static str,
    license: &'static str,
    license_sha256: &'static str,
    golden: &'static str,
    upstream: &'static str,
    asset_id: u128,
}

const FIXTURES: [Fixture; 2] = [
    Fixture {
        key: "noto_sans",
        bytes: NOTO_BYTES,
        sha256: "0afd77effc877ff84fa7995a58c396c124514855f8084056846b54b8cb76f3ce",
        family: "Noto Sans",
        license: NOTO_LICENSE,
        license_sha256: "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a",
        golden: NOTO_GOLDEN,
        upstream: "notofonts/latin-greek-cyrillic@c4a321e123e4d4ff315f57f4e0adf294fe3a95be",
        asset_id: 0xf2,
    },
    Fixture {
        key: "recursive",
        bytes: RECURSIVE_BYTES,
        sha256: "11fca6aeeaa73644a2174d2608cab7eb5d9828f5d88a7feca2c299415f3fa604",
        family: "Recursive",
        license: RECURSIVE_LICENSE,
        license_sha256: "f9f539cf7549bd417159dbdb9c400943a5b60a7366c2c6fbde9f095173d82479",
        golden: RECURSIVE_GOLDEN,
        upstream: "arrowtype/recursive@071fc21f217781110d67e8d0bf5021f31cbdcb85",
        asset_id: 0xf3,
    },
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Golden {
    schema_version: u32,
    tool: String,
    version: String,
    capture_command: String,
    fixture: String,
    font_sha256: String,
    axes: Vec<GoldenAxis>,
    named_instance_count: usize,
    cases: Vec<GoldenCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenAxis {
    tag: String,
    minimum: f64,
    default: f64,
    maximum: f64,
    hidden: bool,
    name_id: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenCase {
    label: String,
    user: BTreeMap<String, f64>,
    normalized_2_14: Vec<i16>,
    text: String,
    serialized_glyphs: String,
    glyph_advances_font_units: Vec<i32>,
    outline_glyph_id: u32,
    outline_serialized_path: String,
    global_metrics_font_units: GoldenMetrics,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoldenMetrics {
    hasc: Option<i32>,
    hdsc: Option<i32>,
    hlgp: Option<i32>,
    xhgt: Option<i32>,
    cpht: Option<i32>,
}

struct Evaluation {
    report: Value,
    inspection: VariableFontInspection,
    blocking_failures: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("variable-font-corpus: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let output = output_path()?;
    let evaluations = FIXTURES
        .iter()
        .map(evaluate_fixture)
        .collect::<Result<Vec<_>, _>>()?;
    let corpus_trials = corpus_trials(&evaluations);
    let blocking_failures = evaluations
        .iter()
        .map(|evaluation| evaluation.blocking_failures)
        .sum::<usize>()
        + corpus_trials
            .iter()
            .filter(|trial| !passed_trial(trial))
            .count();
    let passed = blocking_failures == 0;
    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:variable-font-corpus-baseline",
        "status": if passed { "passed" } else { "failed" },
        "candidate_profile": OPENTYPE_VARIABLE_TRUETYPE_PROFILE,
        "fixtures": evaluations.iter().map(|evaluation| &evaluation.report).collect::<Vec<_>>(),
        "corpus_trials": corpus_trials,
        "summary": {
            "fixtures": evaluations.len(),
            "oracle_cases": evaluations.len() * 4,
            "blocking_failures": blocking_failures,
        },
        "non_claims": [
            "two independently authored OFL-derived families broaden evidence but do not exhaust OpenType variable encodings",
            "the HarfBuzz captures are committed offline oracles rather than live CI dependencies",
            "unhinted outline agreement does not claim raster pixel identity across platforms",
            "vector agreement permits at most one 26.6 coordinate unit after identical path topology because independent interpolation can resolve half-unit ties differently",
            "candidate asset validation does not enable typed package layout or rendering",
            "VVAR and vertical text remain a separate rejected capability",
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
        "variable font corpus: {} fixtures, {} oracle cases, status {}",
        evaluations.len(),
        evaluations.len() * 4,
        if passed { "passed" } else { "failed" }
    );
    if passed {
        Ok(())
    } else {
        Err(format!("report failed; inspect {}", output.display()))
    }
}

fn evaluate_fixture(fixture: &Fixture) -> Result<Evaluation, String> {
    let golden: Golden = serde_json::from_str(fixture.golden).map_err(|error| error.to_string())?;
    let inspection = inspect_opentype_variable_metadata(fixture.bytes, 0)
        .map_err(|error| format!("{} inspection failed: {error}", fixture.key))?;
    let identity_trials = identity_trials(fixture, &golden, &inspection);
    let axis_trials = axis_trials(&golden, &inspection);
    let (case_trials, observations) = evaluate_cases(fixture, &golden)?;
    let metamorphic_trials = metamorphic_trials(&observations);
    let blocking_failures = identity_trials
        .iter()
        .chain(&axis_trials)
        .chain(&case_trials)
        .chain(&metamorphic_trials)
        .filter(|trial| !passed_trial(trial))
        .count();
    let report = json!({
        "key": fixture.key,
        "family": fixture.family,
        "sha256": fixture.sha256,
        "license": "OFL-1.1",
        "license_sha256": fixture.license_sha256,
        "upstream": fixture.upstream,
        "inspection": &inspection,
        "identity_trials": identity_trials,
        "axis_trials": axis_trials,
        "case_trials": case_trials,
        "metamorphic_trials": metamorphic_trials,
        "observations": observations,
        "blocking_failures": blocking_failures,
    });
    Ok(Evaluation {
        report,
        inspection,
        blocking_failures,
    })
}

fn identity_trials(
    fixture: &Fixture,
    golden: &Golden,
    inspection: &VariableFontInspection,
) -> Vec<Value> {
    vec![
        trial(
            "pinned_harfbuzz_oracle",
            golden.schema_version == 1
                && golden.tool == "HarfBuzz public C API and hb-shape"
                && golden.version == "14.4.0"
                && !golden.capture_command.is_empty()
                && golden.fixture.contains(fixture.family),
        ),
        trial(
            "exact_font_and_license_identity",
            sha256(fixture.bytes) == fixture.sha256
                && golden.font_sha256 == fixture.sha256
                && sha256(fixture.license.as_bytes()) == fixture.license_sha256,
        ),
        trial(
            "candidate_tables_and_family",
            inspection.font.decoder_profile == OPENTYPE_VARIABLE_TRUETYPE_PROFILE
                && inspection
                    .font
                    .names
                    .iter()
                    .any(|name| name == fixture.family)
                && ["fvar", "avar", "gvar", "HVAR", "MVAR", "STAT"]
                    .iter()
                    .all(|tag| inspection.font.table_tags.iter().any(|item| item == tag))
                && !inspection.font.table_tags.iter().any(|tag| tag == "VVAR"),
        ),
        trial(
            "required_location_corpus",
            ["default", "minimum", "maximum", "interior"]
                .iter()
                .all(|label| golden.cases.iter().any(|case| &case.label == label)),
        ),
        candidate_asset_trial(fixture, inspection),
    ]
}

fn axis_trials(golden: &Golden, inspection: &VariableFontInspection) -> Vec<Value> {
    let mut axis_trials = inspection
        .axes
        .iter()
        .zip(&golden.axes)
        .enumerate()
        .map(|(index, (observed, expected))| {
            trial(
                &format!("harfbuzz_axis_{index}_agrees"),
                observed.tag == expected.tag
                    && observed.minimum_16_16 == to_fixed(expected.minimum)
                    && observed.default_16_16 == to_fixed(expected.default)
                    && observed.maximum_16_16 == to_fixed(expected.maximum)
                    && observed.hidden == expected.hidden
                    && observed.name_id == expected.name_id,
            )
        })
        .collect::<Vec<_>>();
    axis_trials.push(trial(
        "axis_and_instance_counts_agree",
        inspection.axes.len() == golden.axes.len()
            && inspection.named_instance_count == golden.named_instance_count,
    ));
    axis_trials
}

fn evaluate_cases(
    fixture: &Fixture,
    golden: &Golden,
) -> Result<(Vec<Value>, BTreeMap<String, Value>), String> {
    let mut case_trials = Vec::with_capacity(golden.cases.len());
    let mut observations = BTreeMap::new();
    for expected in &golden.cases {
        let (case_trial, observation) = evaluate_case(fixture, expected)?;
        case_trials.push(case_trial);
        observations.insert(expected.label.clone(), observation);
    }
    Ok((case_trials, observations))
}

fn evaluate_case(fixture: &Fixture, expected: &GoldenCase) -> Result<(Value, Value), String> {
    let normalized = normalize_variable_coordinates(fixture.bytes, &expected.user)
        .map_err(|error| error.to_string())?;
    let face = VariableResourceTrial::new_with_features(
        fixture.bytes,
        fixture.sha256,
        fixture.family,
        "OFL-1.1; exact license and derived-fixture provenance retained",
        &expected.user,
        &BTreeMap::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = ShapeRequest {
        text: &expected.text,
        font_sha256: fixture.sha256,
        font_size: 24.0,
        direction: TextDirection::LeftToRight,
        language: "en",
    };
    let first = face.shape(&request).map_err(|error| error.to_string())?;
    let second = face.shape(&request).map_err(|error| error.to_string())?;
    let advances = first
        .run
        .glyphs
        .iter()
        .map(|glyph| face.glyph_advance_font_units(glyph.glyph_id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let outline = face
        .outline_glyph(expected.outline_glyph_id)
        .map_err(|error| error.to_string())?;
    let repeated_outline = face
        .outline_glyph(expected.outline_glyph_id)
        .map_err(|error| error.to_string())?;
    let metrics = face
        .global_metrics_font_units()
        .map_err(|error| error.to_string())?;
    let normalized_bits = normalized
        .iter()
        .map(|coordinate| coordinate.normalized_2_14)
        .collect::<Vec<_>>();
    let outline_delta =
        outline_coordinate_delta(&outline.serialized_path, &expected.outline_serialized_path);
    let passed = first == second
        && first.run.serialized_glyphs == expected.serialized_glyphs
        && first.run.glyphs.len() == expected.glyph_advances_font_units.len()
        && first
            .run
            .glyphs
            .iter()
            .map(|glyph| glyph.x_advance)
            .eq(expected.glyph_advances_font_units.iter().copied())
        && advances == expected.glyph_advances_font_units
        && normalized_bits == expected.normalized_2_14
        && first
            .coordinates
            .iter()
            .map(|coordinate| coordinate.normalized_2_14)
            .eq(expected.normalized_2_14.iter().copied())
        && metrics == expected.global_metrics_font_units.required()?
        && outline == repeated_outline
        && outline_delta.is_some_and(|delta| delta <= 1);
    let case_trial = trial(
        &format!("harfbuzz_{}_pipeline_agrees", expected.label),
        passed,
    );
    let observation = json!({
        "coordinates": first.coordinates,
        "serialized_glyphs": first.run.serialized_glyphs,
        "glyph_advances_font_units": advances,
        "global_metrics_font_units": metrics,
        "outline_sha256": sha256(outline.serialized_path.as_bytes()),
        "outline_exact": outline.serialized_path == expected.outline_serialized_path,
        "outline_max_coordinate_delta_26_6": outline_delta,
        "outline_commands": outline.commands.len(),
    });
    Ok((case_trial, observation))
}

fn outline_coordinate_delta(observed: &str, expected: &str) -> Option<u32> {
    let (observed_topology, observed_coordinates) = parse_outline(observed)?;
    let (expected_topology, expected_coordinates) = parse_outline(expected)?;
    if observed_topology != expected_topology
        || observed_coordinates.len() != expected_coordinates.len()
    {
        return None;
    }
    observed_coordinates
        .iter()
        .zip(expected_coordinates)
        .map(|(left, right)| left.abs_diff(right))
        .max()
        .or(Some(0))
}

fn parse_outline(path: &str) -> Option<(String, Vec<i32>)> {
    let bytes = path.as_bytes();
    let mut topology = String::new();
    let mut coordinates = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let number = bytes[index].is_ascii_digit()
            || (bytes[index] == b'-' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit));
        if !number {
            topology.push(char::from(bytes[index]));
            index += 1;
            continue;
        }
        let start = index;
        index += usize::from(bytes[index] == b'-');
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        topology.push('#');
        coordinates.push(path.get(start..index)?.parse().ok()?);
    }
    Some((topology, coordinates))
}

impl GoldenMetrics {
    fn required(&self) -> Result<VariableGlobalMetrics, String> {
        Ok(VariableGlobalMetrics {
            ascent: self
                .hasc
                .ok_or_else(|| "HarfBuzz ascent metric is absent".to_owned())?,
            descent: self
                .hdsc
                .ok_or_else(|| "HarfBuzz descent metric is absent".to_owned())?,
            line_gap: self
                .hlgp
                .ok_or_else(|| "HarfBuzz line-gap metric is absent".to_owned())?,
            x_height: self.xhgt,
            cap_height: self.cpht,
        })
    }
}

fn candidate_asset_trial(fixture: &Fixture, inspection: &VariableFontInspection) -> Value {
    let asset = Asset {
        schema_version: 1,
        id: AssetId::new(fixture.asset_id),
        name: Some(format!("{} variable corpus candidate", fixture.family)),
        resource: Some(ResourceDigest::from_sha256_hex(fixture.sha256)),
        portability: AssetPortability::Portable,
        kind: AssetKind::Font(FontAsset {
            face_index: 0,
            names: inspection.font.names.clone(),
            axes: inspection
                .axes
                .iter()
                .map(|axis| (axis.tag.clone(), f64::from(axis.default_16_16) / 65_536.0))
                .collect(),
            features: BTreeMap::new(),
            coverage: inspection.font.coverage.clone(),
            policy_evidence: BTreeMap::from([
                (
                    "font.decoder_profile".to_owned(),
                    OPENTYPE_VARIABLE_TRUETYPE_PROFILE.to_owned(),
                ),
                (
                    "opentype.fs_type".to_owned(),
                    format!("0x{:04x}", inspection.font.fs_type),
                ),
                ("license.expression".to_owned(), "OFL-1.1".to_owned()),
                ("license.embedding_review".to_owned(), "approved".to_owned()),
            ]),
        }),
    };
    let result = validate_variable_font_asset_candidate(&asset, fixture.bytes);
    trial(
        "candidate_asset_metadata_and_policy_validate",
        result.is_ok_and(|validated| {
            validated.coordinates.len() == inspection.axes.len()
                && validated
                    .coordinates
                    .iter()
                    .all(|coordinate| coordinate.normalized_2_14 == 0)
        }),
    )
}

fn metamorphic_trials(observations: &BTreeMap<String, Value>) -> Vec<Value> {
    let value = |label: &str, field: &str| observations.get(label)?.get(field);
    let x_height = |label: &str| {
        observations
            .get(label)?
            .get("global_metrics_font_units")?
            .get("x_height")?
            .as_i64()
    };
    vec![
        trial(
            "location_changes_hvar_advances",
            value("minimum", "glyph_advances_font_units")
                != value("maximum", "glyph_advances_font_units"),
        ),
        trial(
            "location_changes_gvar_outline",
            value("minimum", "outline_sha256") != value("maximum", "outline_sha256"),
        ),
        trial(
            "location_changes_mvar_x_height",
            x_height("minimum").is_some() && x_height("minimum") != x_height("maximum"),
        ),
    ]
}

fn corpus_trials(evaluations: &[Evaluation]) -> Vec<Value> {
    let graph = |index: usize| &evaluations[index].inspection.variation_graph;
    vec![
        trial(
            "independent_upstream_owners",
            FIXTURES[0].upstream.starts_with("notofonts/")
                && FIXTURES[1].upstream.starts_with("arrowtype/"),
        ),
        trial(
            "axis_and_instance_shapes_are_distinct",
            evaluations[0].inspection.axes.len() == 2
                && evaluations[1].inspection.axes.len() == 5
                && evaluations[0].inspection.named_instance_count
                    != evaluations[1].inspection.named_instance_count,
        ),
        trial(
            "both_graphs_exercise_hvar_and_mvar",
            evaluations.iter().all(|evaluation| {
                evaluation.inspection.variation_graph.hvar_store.is_some()
                    && evaluation.inspection.variation_graph.mvar_store.is_some()
            }),
        ),
        trial(
            "gvar_graph_shapes_are_distinct",
            graph(0).gvar_tuple_count > 0
                && graph(1).gvar_tuple_count > 0
                && (graph(0).gvar_tuple_count != graph(1).gvar_tuple_count
                    || graph(0).gvar_explicit_delta_count != graph(1).gvar_explicit_delta_count),
        ),
    ]
}

fn trial(name: &str, passed: bool) -> Value {
    json!({ "name": name, "status": if passed { "passed" } else { "failed" } })
}

fn passed_trial(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("passed")
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "oracle axis values are bounded exact OpenType values"
)]
fn to_fixed(value: f64) -> i32 {
    (value * 65_536.0).round() as i32
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn output_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let mut output = PathBuf::from("target/variable-font-corpus-report.json");
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
                return Err("usage: variable-font-corpus [--output <json>]".to_owned());
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    Ok(output)
}
