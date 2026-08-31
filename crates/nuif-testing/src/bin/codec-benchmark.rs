use nuif_codec::{CanonicalText, Canonicalizer, CodecError, Decoder, DeterministicCbor, Encoder};
use nuif_core::EntityId;
use serde::Serialize;
use sha2::{Digest, Sha256};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use std::alloc::System;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const CORPUS_SEED: u64 = 0x4e55_4946_434f_4445;
const SCALES: [usize; 4] = [8, 64, 512, 4_096];
const SAMPLES: usize = 11;
const WARMUP_INVOCATIONS: usize = 3;
const MAX_ALLOCATED_BYTES: usize = 512 * 1024 * 1024;
const MAX_P95_NANOSECONDS: u128 = 10_000_000_000;

#[derive(Debug, Serialize)]
struct CodecReport {
    profile: &'static str,
    admitted_for_timing: bool,
    native_partial_load: bool,
    partial_load_measurement: &'static str,
    preflight: PreflightReport,
    scales: Vec<ScaleReport>,
}

#[derive(Debug, Serialize)]
struct PreflightReport {
    checks: Vec<ConformanceCheck>,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct ConformanceCheck {
    name: &'static str,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct ScaleReport {
    entities: usize,
    encoded_bytes: usize,
    bytes_per_entity: String,
    encoded_sha256: String,
    measurements: Vec<Measurement>,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct Measurement {
    operation: &'static str,
    iterations_per_sample: usize,
    samples: usize,
    median_nanoseconds: u128,
    p95_nanoseconds: u128,
    minimum_nanoseconds: u128,
    maximum_nanoseconds: u128,
    allocations_per_invocation: usize,
    allocated_bytes_per_invocation: usize,
    retained_bytes_per_invocation: i128,
    checksum: u64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct CandidateAdmission {
    candidate: &'static str,
    decision: &'static str,
    canonical_form: &'static str,
    unknown_data: &'static str,
    partial_access: &'static str,
    missing_before_timing: Vec<&'static str>,
    rationale: &'static str,
    primary_sources: Vec<&'static str>,
}

fn main() {
    if env::args().any(|argument| matches!(argument.as_str(), "--help" | "-h")) {
        println!("usage: codec-benchmark [--output <json>]");
        return;
    }
    let output = parse_output().unwrap_or_else(|error| fail(&error));
    let text = benchmark_codec("nuif-text-0", CanonicalText);
    let cbor = benchmark_codec("nuif-cbor-0", DeterministicCbor);
    let passed = text.preflight.passed
        && cbor.preflight.passed
        && text.scales.iter().all(|scale| scale.passed)
        && cbor.scales.iter().all(|scale| scale.passed);
    let size_comparison = text
        .scales
        .iter()
        .zip(&cbor.scales)
        .map(|(text_scale, cbor_scale)| {
            serde_json::json!({
                "entities": text_scale.entities,
                "canonical_text_bytes": text_scale.encoded_bytes,
                "deterministic_cbor_bytes": cbor_scale.encoded_bytes,
                "cbor_to_text_basis_points": cbor_scale.encoded_bytes
                    .saturating_mul(10_000)
                    / text_scale.encoded_bytes,
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:codec-benchmark",
        "status": if passed { "passed" } else { "failed" },
        "scope": "implemented codec decision gate; schema candidates remain unbenchmarked until admitted",
        "corpus": {
            "seed": CORPUS_SEED,
            "generator": "nuif_testing::performance_fixture(scale, true)",
            "scales": SCALES,
            "unknown_data_fixture": "nuif_testing::responsive_card_fixture()",
        },
        "measurement": {
            "build_profile": "release",
            "samples": SAMPLES,
            "warmup_invocations": WARMUP_INVOCATIONS,
            "latency_statistic": "median and nearest-rank p95 of per-invocation wall-clock nanoseconds",
            "allocation_statistic": "one warmed invocation through stats_alloc",
            "partial_load_policy": "report native support separately; decode_then_select measures the current whole-document access path",
            "admission_rule": "a complete semantic mapping must pass exact round-trip, canonical fixpoint, and unknown-data preservation before it is timed",
        },
        "engine": {
            "toolchain": rustc_version(),
            "source_revision": command_text("git", &["rev-parse", "HEAD"]),
            "source_dirty": command_text("git", &["status", "--porcelain"])
                .map(|value| !value.is_empty()),
        },
        "platform": {
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
            "available_parallelism": std::thread::available_parallelism().map_or(1, usize::from),
            "cpu_model": cpu_model(),
        },
        "implemented_codecs": [text, cbor],
        "size_comparison": size_comparison,
        "schema_candidate_admission": schema_candidates(),
    });
    let encoded = serde_json::to_vec_pretty(&report).expect("codec benchmark report serializes");
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                fail(&format!("cannot create {}: {error}", parent.display()));
            });
        }
        fs::write(&path, &encoded)
            .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", path.display())));
    }
    println!("{}", String::from_utf8(encoded).expect("JSON is UTF-8"));
    if !passed {
        std::process::exit(1);
    }
}

fn benchmark_codec<C>(profile: &'static str, codec: C) -> CodecReport
where
    C: Copy
        + Encoder<Error = CodecError>
        + Decoder<Error = CodecError>
        + Canonicalizer<Error = CodecError>,
{
    let preflight = preflight(codec);
    if !preflight.passed {
        return CodecReport {
            profile,
            admitted_for_timing: false,
            native_partial_load: false,
            partial_load_measurement: "not run because conformance preflight failed",
            preflight,
            scales: Vec::new(),
        };
    }
    let scales = SCALES
        .into_iter()
        .map(|scale| benchmark_scale(codec, scale))
        .collect();
    CodecReport {
        profile,
        admitted_for_timing: true,
        native_partial_load: false,
        partial_load_measurement: "decode_then_select (full decode followed by indexed entity access)",
        preflight,
        scales,
    }
}

fn preflight<C>(codec: C) -> PreflightReport
where
    C: Copy
        + Encoder<Error = CodecError>
        + Decoder<Error = CodecError>
        + Canonicalizer<Error = CodecError>,
{
    let fixture = nuif_testing::responsive_card_fixture();
    let encoded = codec
        .encode(&fixture)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let decoded = codec
        .decode(&encoded)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let semantic_round_trip = decoded == fixture;
    let canonical_fixpoint = codec
        .encode(&decoded)
        .is_ok_and(|round_trip| round_trip == encoded);
    let canonicalizer_fixpoint = codec
        .canonicalize(&encoded)
        .is_ok_and(|canonical| canonical == encoded);

    let mut edited = fixture.clone();
    edited
        .entities
        .get_mut(&EntityId::new(0x21))
        .expect("responsive fixture media entity")
        .name = Some("Neighboring semantic edit".to_owned());
    let edited_bytes = codec
        .encode(&edited)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let edited_round_trip = codec
        .decode(&edited_bytes)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let unknown_id = EntityId::new(0x25);
    let unknown_data_after_neighbor_edit = edited_round_trip == edited
        && edited_round_trip.entities.get(&unknown_id) == fixture.entities.get(&unknown_id)
        && edited_round_trip.extension_declarations == fixture.extension_declarations
        && edited_round_trip.extensions == fixture.extensions;
    let passed = semantic_round_trip
        && canonical_fixpoint
        && canonicalizer_fixpoint
        && unknown_data_after_neighbor_edit;
    PreflightReport {
        checks: vec![
            ConformanceCheck {
                name: "semantic_round_trip",
                passed: semantic_round_trip,
            },
            ConformanceCheck {
                name: "canonical_fixpoint",
                passed: canonical_fixpoint,
            },
            ConformanceCheck {
                name: "canonicalizer_fixpoint",
                passed: canonicalizer_fixpoint,
            },
            ConformanceCheck {
                name: "unknown_data_after_neighbor_edit",
                passed: unknown_data_after_neighbor_edit,
            },
        ],
        passed,
    }
}

fn benchmark_scale<C>(codec: C, scale: usize) -> ScaleReport
where
    C: Copy
        + Encoder<Error = CodecError>
        + Decoder<Error = CodecError>
        + Canonicalizer<Error = CodecError>,
{
    let document = nuif_testing::performance_fixture(scale, true);
    let encoded = codec
        .encode(&document)
        .unwrap_or_else(|error| fail(&error.to_string()));
    let select_id = EntityId::new(u128::try_from(scale).expect("bounded scale") + 1);
    let iterations = match scale {
        0..=64 => 20,
        65..=512 => 5,
        _ => 1,
    };
    let measurements = vec![
        measure("encode", iterations, || {
            codec
                .encode(black_box(&document))
                .expect("admitted codec encode")
                .len() as u64
        }),
        measure("decode", iterations, || {
            codec
                .decode(black_box(&encoded))
                .expect("admitted codec decode")
                .entities
                .len() as u64
        }),
        measure("canonicalize", iterations, || {
            codec
                .canonicalize(black_box(&encoded))
                .expect("admitted codec canonicalize")
                .len() as u64
        }),
        measure("decode_then_select", iterations, || {
            let decoded = codec
                .decode(black_box(&encoded))
                .expect("admitted codec decode");
            decoded.entities.get(&select_id).map_or(0, |entity| {
                u64::try_from(entity.id.0).expect("bounded fixture identity")
            })
        }),
    ];
    let passed = measurements.iter().all(|measurement| measurement.passed);
    ScaleReport {
        entities: document.entities.len(),
        encoded_bytes: encoded.len(),
        bytes_per_entity: decimal_ratio(encoded.len(), document.entities.len()),
        encoded_sha256: sha256(&encoded),
        measurements,
        passed,
    }
}

fn measure(
    operation: &'static str,
    iterations_per_sample: usize,
    mut action: impl FnMut() -> u64,
) -> Measurement {
    for _ in 0..WARMUP_INVOCATIONS {
        black_box(action());
    }
    let region = Region::new(GLOBAL);
    let allocation_checksum = black_box(action());
    let allocation = region.change();
    let mut values = Vec::with_capacity(SAMPLES);
    let mut checksum = allocation_checksum;
    for sample in 0..SAMPLES {
        let started = Instant::now();
        for iteration in 0..iterations_per_sample {
            checksum =
                checksum.rotate_left(7) ^ black_box(action()) ^ sample as u64 ^ iteration as u64;
        }
        values.push(started.elapsed().as_nanos() / iterations_per_sample as u128);
    }
    values.sort_unstable();
    let median = values[SAMPLES / 2];
    let p95 = values[(SAMPLES * 95).div_ceil(100).saturating_sub(1)];
    let passed = p95 <= MAX_P95_NANOSECONDS && allocation.bytes_allocated <= MAX_ALLOCATED_BYTES;
    Measurement {
        operation,
        iterations_per_sample,
        samples: SAMPLES,
        median_nanoseconds: median,
        p95_nanoseconds: p95,
        minimum_nanoseconds: values[0],
        maximum_nanoseconds: values[SAMPLES - 1],
        allocations_per_invocation: allocation.allocations,
        allocated_bytes_per_invocation: allocation.bytes_allocated,
        retained_bytes_per_invocation: retained_bytes(allocation),
        checksum,
        passed,
    }
}

fn schema_candidates() -> Vec<CandidateAdmission> {
    vec![
        CandidateAdmission {
            candidate: "Protocol Buffers",
            decision: "not_admitted",
            canonical_form: "not specified for the binary wire format; deterministic output is explicitly not canonical",
            unknown_data: "binary runtimes can retain fields, but reconstruction and JSON conversion can lose them",
            partial_access: "message decode rather than a NUIF-native partial-load contract",
            missing_before_timing: vec![
                "complete NUIF semantic schema and migration",
                "custom cross-language canonicalization profile",
                "old-reader edit and unknown-field preservation fixtures",
            ],
            rationale: "Timing generated structs now would measure an incomplete model and would not satisfy NUIF hash stability.",
            primary_sources: vec![
                "https://protobuf.dev/programming-guides/serialization-not-canonical/",
                "https://protobuf.dev/programming-guides/proto3/#unknown-fields",
            ],
        },
        CandidateAdmission {
            candidate: "FlatBuffers",
            decision: "not_admitted",
            canonical_form: "no canonical byte profile identified in the official format documentation",
            unknown_data: "old readers ignore newly added fields, so rebuilding through an old object model does not prove retention",
            partial_access: "native in-place field access is a material potential advantage",
            missing_before_timing: vec![
                "complete NUIF semantic schema and migration",
                "normative canonical writer profile",
                "retentive edit path for fields unknown to an older schema",
                "bounded verifier and hostile-input profile",
            ],
            rationale: "Zero-copy access is promising, but size and speed are irrelevant if an edit silently drops future semantics.",
            primary_sources: vec![
                "https://flatbuffers.dev/evolution/",
                "https://flatbuffers.dev/white_paper/",
            ],
        },
        CandidateAdmission {
            candidate: "Cap'n Proto",
            decision: "next_candidate",
            canonical_form: "schema-agnostic canonical form is specified, although default encoders rarely emit it",
            unknown_data: "struct layout supports schema evolution; NUIF still needs cross-version retentive edit proof",
            partial_access: "pointer-based traversal can avoid a separate decoded object graph",
            missing_before_timing: vec![
                "complete NUIF semantic schema and migration",
                "canonical writer verification in at least two implementations",
                "old-reader edit and unknown-data preservation fixtures",
                "NUIF traversal and nesting limits",
            ],
            rationale: "It is the strongest next schema candidate because canonicalization and bounded traversal are specified, but it is not yet a conforming NUIF codec.",
            primary_sources: vec!["https://capnproto.org/encoding.html"],
        },
    ]
}

fn retained_bytes(stats: Stats) -> i128 {
    stats.bytes_allocated as i128 - stats.bytes_deallocated as i128
}

fn decimal_ratio(numerator: usize, denominator: usize) -> String {
    let whole = numerator / denominator;
    let thousandths = numerator % denominator * 1_000 / denominator;
    format!("{whole}.{thousandths:03}")
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_output() -> Result<Option<PathBuf>, String> {
    let mut args = env::args().skip(1);
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--output" => {
                output = Some(
                    args.next()
                        .ok_or_else(|| "--output requires a path".to_owned())?
                        .into(),
                );
            }
            unknown => return Err(format!("unknown argument {unknown:?}")),
        }
    }
    Ok(output)
}

fn rustc_version() -> String {
    command_text("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned())
}

fn command_text(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_owned())
}

fn cpu_model() -> Option<String> {
    command_text("sysctl", &["-n", "machdep.cpu.brand_string"]).or_else(|| {
        fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|cpuinfo| {
                cpuinfo.lines().find_map(|line| {
                    line.strip_prefix("model name\t:")
                        .map(|value| value.trim().to_owned())
                })
            })
    })
}

fn fail(message: &str) -> ! {
    eprintln!("codec-benchmark: {message}");
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn implemented_codecs_pass_admission_preflight() {
        assert!(preflight(CanonicalText).passed);
        assert!(preflight(DeterministicCbor).passed);
    }

    #[test]
    fn schema_candidates_are_not_timed_without_a_complete_mapping() {
        let candidates = schema_candidates();
        assert_eq!(candidates.len(), 3);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.decision != "admitted")
        );
        assert_eq!(candidates[2].decision, "next_candidate");
    }

    #[test]
    fn output_parent_can_be_empty_or_concrete() {
        assert_eq!(Path::new("report.json").parent(), Some(Path::new("")));
        assert_eq!(
            Path::new("target/report.json").parent(),
            Some(Path::new("target"))
        );
    }
}
