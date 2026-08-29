use nuif_codec::{
    CanonicalText, CodecError, DeterministicCbor, Encoder, MAX_INPUT_BYTES, MAX_SYNTAX_DEPTH,
};
use nuif_core::{
    ContextPredicate, Document, Entity, EntityId, EntityKind, OpaqueEncoding, OpaquePayload,
    PROFILE0_RESOURCE_LIMITS, PropertyValue, Relation, ResponsiveOverride, Token,
};
use serde_json::{Value, json};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use std::alloc::System;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const MAX_CASE_MICROSECONDS: u128 = 2_000_000;
const MAX_CASE_ALLOCATED_BYTES: usize = 64 * 1024 * 1024;
const MAX_CASE_RETAINED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
enum Encoding {
    Text,
    Cbor,
}

#[derive(Clone, Copy)]
enum Expected {
    Accepted,
    Malformed,
    ResourceLimit(&'static str),
}

struct Case {
    name: &'static str,
    encoding: Encoding,
    input: Vec<u8>,
    expected: Expected,
}

fn main() {
    let output = output_path().unwrap_or_else(|error| fail(&error));
    let cases = cases().unwrap_or_else(|error| fail(&error));
    let mut reports = Vec::new();
    let mut passed = true;

    // Warm parser initialization and allocator metadata before measuring each
    // region. Inputs and report construction are intentionally outside regions.
    let warm = CanonicalText
        .encode(&nuif_testing::responsive_card_fixture())
        .and_then(|bytes| CanonicalText.decode_for_validation(&bytes));
    if warm.is_err() {
        fail("hostile-input warmup failed");
    }

    for case in cases {
        let region = Region::new(GLOBAL);
        let started = Instant::now();
        let result = match case.encoding {
            Encoding::Text => CanonicalText.decode_for_validation(&case.input),
            Encoding::Cbor => DeterministicCbor.decode_for_validation(&case.input),
        };
        let elapsed_microseconds = started.elapsed().as_micros();
        let expectation_met = matches_expected(&result, case.expected);
        let stats = region.change();
        let retained_bytes = retained_bytes(stats);
        let within_time = elapsed_microseconds <= MAX_CASE_MICROSECONDS;
        let within_allocations = stats.bytes_allocated <= MAX_CASE_ALLOCATED_BYTES
            && retained_bytes <= MAX_CASE_RETAINED_BYTES;
        let case_passed = expectation_met && within_time && within_allocations;
        passed &= case_passed;
        let observed = describe_result(&result);
        drop(result);

        reports.push(json!({
            "name": case.name,
            "encoding": match case.encoding { Encoding::Text => "text", Encoding::Cbor => "cbor" },
            "input_bytes": case.input.len(),
            "expected": describe_expected(case.expected),
            "observed": observed,
            "elapsed_microseconds": elapsed_microseconds,
            "allocations": stats.allocations,
            "reallocations": stats.reallocations,
            "allocated_bytes": stats.bytes_allocated,
            "retained_bytes": retained_bytes,
            "expectation_met": expectation_met,
            "within_time_budget": within_time,
            "within_allocation_budget": within_allocations,
            "passed": case_passed
        }));
    }

    let report = json!({
        "schema_version": 1,
        "experiment": "nuif:experiment:hostile-input-budgets",
        "status": if passed { "passed" } else { "failed" },
        "toolchain": rustc_version(),
        "build_profile": "release",
        "allocator_measurement": "stats_alloc 0.1.10 instrumented system allocator",
        "warmup": "one canonical v0 fixture text encode/decode before case measurement",
        "platform": {
            "os": env::consts::OS,
            "architecture": env::consts::ARCH,
            "available_parallelism": std::thread::available_parallelism().map_or(1, usize::from),
            "cpu_model": cpu_model()
        },
        "limits": {
            "input_bytes": MAX_INPUT_BYTES,
            "syntax_depth": MAX_SYNTAX_DEPTH,
            "entities": PROFILE0_RESOURCE_LIMITS.entities,
            "roots": PROFILE0_RESOURCE_LIMITS.roots,
            "tokens": PROFILE0_RESOURCE_LIMITS.tokens,
            "relations": PROFILE0_RESOURCE_LIMITS.relations,
            "child_references": PROFILE0_RESOURCE_LIMITS.child_references,
            "responsive_overrides": PROFILE0_RESOURCE_LIMITS.responsive_overrides,
            "property_values": PROFILE0_RESOURCE_LIMITS.property_values,
            "property_depth": PROFILE0_RESOURCE_LIMITS.property_depth,
            "containment_depth": PROFILE0_RESOURCE_LIMITS.containment_depth,
            "binary_bytes": PROFILE0_RESOURCE_LIMITS.binary_bytes,
            "string_bytes": PROFILE0_RESOURCE_LIMITS.string_bytes,
            "single_string_bytes": PROFILE0_RESOURCE_LIMITS.single_string_bytes,
            "case_time_microseconds": MAX_CASE_MICROSECONDS,
            "case_allocated_bytes": MAX_CASE_ALLOCATED_BYTES,
            "case_retained_bytes": MAX_CASE_RETAINED_BYTES
        },
        "cases": reports
    });
    let encoded = serde_json::to_vec_pretty(&report).expect("measurement report serializes");
    if let Some(path) = output {
        fs::write(&path, &encoded)
            .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", path.display())));
    }
    println!("{}", String::from_utf8(encoded).expect("JSON is UTF-8"));
    if !passed {
        std::process::exit(1);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the complete adversarial boundary matrix stays visible as one ordered table"
)]
fn cases() -> Result<Vec<Case>, String> {
    let oversized = vec![b' '; MAX_INPUT_BYTES.saturating_add(1)];
    let deep_text = format!(
        "{}null{}",
        "[".repeat(MAX_SYNTAX_DEPTH.saturating_add(1)),
        "]".repeat(MAX_SYNTAX_DEPTH.saturating_add(1))
    )
    .into_bytes();
    let mut deep_cbor = vec![0x81; MAX_SYNTAX_DEPTH.saturating_add(1)];
    deep_cbor.push(0xf6);

    let mut too_many_roots = Document::empty(EntityId::new(1));
    too_many_roots.roots = vec![EntityId::new(2); PROFILE0_RESOURCE_LIMITS.roots.saturating_add(1)];

    let mut long_string = one_entity_document();
    long_string
        .entities
        .get_mut(&EntityId::new(2))
        .unwrap()
        .name = Some(
        "x".repeat(
            PROFILE0_RESOURCE_LIMITS
                .single_string_bytes
                .saturating_add(1),
        ),
    );

    let mut deep_property = one_entity_document();
    let mut value = PropertyValue::Null;
    for _ in 0..PROFILE0_RESOURCE_LIMITS.property_depth {
        value = PropertyValue::Array(vec![value]);
    }
    deep_property
        .entities
        .get_mut(&EntityId::new(2))
        .unwrap()
        .authored
        .values
        .insert("nested".to_owned(), value);

    let containment_boundary = containment_document(PROFILE0_RESOURCE_LIMITS.containment_depth)?;
    let containment_over =
        containment_document(PROFILE0_RESOURCE_LIMITS.containment_depth.saturating_add(1))?;
    let string_boundary = string_boundary_document()?;
    let binary_boundary = binary_boundary_document();
    let entity_boundary = flat_entity_document(PROFILE0_RESOURCE_LIMITS.entities)?;
    let entity_over = flat_entity_document(PROFILE0_RESOURCE_LIMITS.entities.saturating_add(1))?;
    let property_boundary = property_cardinality_document(PROFILE0_RESOURCE_LIMITS.property_values);
    let property_over =
        property_cardinality_document(PROFILE0_RESOURCE_LIMITS.property_values.saturating_add(1));
    let wide_cbor_map = wide_cbor_map()?;
    let roots_boundary = roots_boundary_document(PROFILE0_RESOURCE_LIMITS.roots)?;
    let tokens_boundary = tokens_boundary_document(PROFILE0_RESOURCE_LIMITS.tokens)?;
    let relations_boundary = relations_boundary_document(PROFILE0_RESOURCE_LIMITS.relations);
    let responsive_boundary =
        responsive_boundary_document(PROFILE0_RESOURCE_LIMITS.responsive_overrides);

    Ok(vec![
        Case {
            name: "oversized-text",
            encoding: Encoding::Text,
            input: oversized,
            expected: Expected::ResourceLimit("input bytes"),
        },
        Case {
            name: "deep-text",
            encoding: Encoding::Text,
            input: deep_text,
            expected: Expected::ResourceLimit("syntax depth"),
        },
        Case {
            name: "deep-cbor",
            encoding: Encoding::Cbor,
            input: deep_cbor,
            expected: Expected::ResourceLimit("syntax depth"),
        },
        Case {
            name: "root-cardinality",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&too_many_roots).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("roots"),
        },
        Case {
            name: "single-string",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&long_string).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("single string bytes"),
        },
        Case {
            name: "property-depth",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&deep_property).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("property depth"),
        },
        Case {
            name: "containment-depth-boundary",
            encoding: Encoding::Text,
            input: CanonicalText
                .encode(&containment_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "containment-depth-over",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&containment_over).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("containment depth"),
        },
        Case {
            name: "string-total-boundary",
            encoding: Encoding::Text,
            input: CanonicalText
                .encode(&string_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "binary-total-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&binary_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "entity-cardinality-boundary",
            encoding: Encoding::Text,
            input: CanonicalText
                .encode(&entity_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "entity-cardinality-over",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&entity_over).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("entities"),
        },
        Case {
            name: "property-cardinality-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&property_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "property-cardinality-over",
            encoding: Encoding::Text,
            input: serde_json::to_vec(&property_over).map_err(|error| error.to_string())?,
            expected: Expected::ResourceLimit("property values"),
        },
        Case {
            name: "wide-cbor-map",
            encoding: Encoding::Cbor,
            input: wide_cbor_map,
            expected: Expected::Malformed,
        },
        Case {
            name: "root-cardinality-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&roots_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "token-cardinality-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&tokens_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "relation-cardinality-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&relations_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
        Case {
            name: "responsive-cardinality-boundary",
            encoding: Encoding::Cbor,
            input: DeterministicCbor
                .encode(&responsive_boundary)
                .map_err(|error| error.to_string())?,
            expected: Expected::Accepted,
        },
    ])
}

fn one_entity_document() -> Document {
    let mut document = Document::empty(EntityId::new(1));
    let root = Entity::new(EntityId::new(2), EntityKind::Container);
    document.roots.push(root.id);
    document.entities.insert(root.id, root);
    document
}

fn containment_document(depth: usize) -> Result<Document, String> {
    let mut document = Document::empty(EntityId::new(1));
    for index in 0..depth {
        let id = EntityId::new(
            u128::try_from(index)
                .map_err(|error| error.to_string())?
                .saturating_add(2),
        );
        let mut entity = Entity::new(id, EntityKind::Container);
        if index.saturating_add(1) < depth {
            entity.children.push(EntityId::new(
                u128::try_from(index)
                    .map_err(|error| error.to_string())?
                    .saturating_add(3),
            ));
        }
        if index == 0 {
            document.roots.push(id);
        }
        document.entities.insert(id, entity);
    }
    Ok(document)
}

fn string_boundary_document() -> Result<Document, String> {
    let mut document = one_entity_document();
    let count = PROFILE0_RESOURCE_LIMITS
        .string_bytes
        .checked_div(PROFILE0_RESOURCE_LIMITS.single_string_bytes)
        .ok_or_else(|| "single-string limit must be non-zero".to_owned())?;
    for index in 0..count {
        let id = EntityId::new(
            u128::try_from(index)
                .map_err(|error| error.to_string())?
                .saturating_add(100),
        );
        document.tokens.insert(
            id,
            Token {
                id,
                name: "x".repeat(PROFILE0_RESOURCE_LIMITS.single_string_bytes),
                value: PropertyValue::Null,
            },
        );
    }
    Ok(document)
}

fn binary_boundary_document() -> Document {
    let mut document = one_entity_document();
    let namespace = "vendor.probe".to_owned();
    document
        .extension_declarations
        .used
        .insert(namespace.clone());
    document.extensions.0.insert(
        namespace,
        OpaquePayload {
            encoding: OpaqueEncoding::Octets,
            bytes: vec![0; PROFILE0_RESOURCE_LIMITS.binary_bytes],
        },
    );
    document
}

fn flat_entity_document(count: usize) -> Result<Document, String> {
    let mut document = Document::empty(EntityId::new(1));
    let root_id = EntityId::new(2);
    let mut root = Entity::new(root_id, EntityKind::Container);
    for index in 1..count {
        let id = EntityId::new(
            u128::try_from(index)
                .map_err(|error| error.to_string())?
                .saturating_add(2),
        );
        root.children.push(id);
        document
            .entities
            .insert(id, Entity::new(id, EntityKind::Container));
    }
    document.roots.push(root_id);
    document.entities.insert(root_id, root);
    Ok(document)
}

fn property_cardinality_document(count: usize) -> Document {
    let mut document = one_entity_document();
    let values = vec![PropertyValue::Null; count.saturating_sub(1)];
    document
        .entities
        .get_mut(&EntityId::new(2))
        .unwrap()
        .authored
        .values
        .insert("values".to_owned(), PropertyValue::Array(values));
    document
}

fn wide_cbor_map() -> Result<Vec<u8>, String> {
    let entries = (0..16_384)
        .map(|index| {
            (
                ciborium::Value::Text(format!("key{index:016x}{}", "x".repeat(45))),
                ciborium::Value::Null,
            )
        })
        .collect::<Vec<_>>();
    let mut bytes = Vec::new();
    ciborium::into_writer(&ciborium::Value::Map(entries), &mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn roots_boundary_document(count: usize) -> Result<Document, String> {
    let mut document = Document::empty(EntityId::new(1));
    for index in 0..count {
        let id = EntityId::new(
            u128::try_from(index)
                .map_err(|error| error.to_string())?
                .saturating_add(2),
        );
        document.roots.push(id);
        document
            .entities
            .insert(id, Entity::new(id, EntityKind::Container));
    }
    Ok(document)
}

fn tokens_boundary_document(count: usize) -> Result<Document, String> {
    let mut document = one_entity_document();
    for index in 0..count {
        let id = EntityId::new(
            u128::try_from(index)
                .map_err(|error| error.to_string())?
                .saturating_add(10_000),
        );
        document.tokens.insert(
            id,
            Token {
                id,
                name: String::new(),
                value: PropertyValue::Null,
            },
        );
    }
    Ok(document)
}

fn relations_boundary_document(count: usize) -> Document {
    let mut document = one_entity_document();
    document.relations = vec![
        Relation {
            kind: "probe".to_owned(),
            source: EntityId::new(2),
            target: EntityId::new(2),
        };
        count
    ];
    document
}

fn responsive_boundary_document(count: usize) -> Document {
    let mut document = one_entity_document();
    document
        .entities
        .get_mut(&EntityId::new(2))
        .unwrap()
        .authored
        .responsive = vec![
        ResponsiveOverride {
            when: ContextPredicate {
                min_width: None,
                max_width: None,
                theme: None,
            },
            direction: None,
            gap: None,
            width: None,
            height: None,
        };
        count
    ];
    document
}

fn matches_expected(result: &Result<Document, CodecError>, expected: Expected) -> bool {
    match (result, expected) {
        (Ok(_), Expected::Accepted) | (Err(CodecError::Malformed(_)), Expected::Malformed) => true,
        (
            Err(CodecError::ResourceLimit { resource, .. }),
            Expected::ResourceLimit(expected_resource),
        ) => resource == &expected_resource,
        _ => false,
    }
}

fn describe_expected(expected: Expected) -> Value {
    match expected {
        Expected::Accepted => json!({"class": "accepted"}),
        Expected::Malformed => json!({"class": "malformed"}),
        Expected::ResourceLimit(resource) => {
            json!({"class": "resource_limit", "resource": resource})
        }
    }
}

fn describe_result(result: &Result<Document, CodecError>) -> Value {
    match result {
        Ok(document) => json!({
            "class": "accepted",
            "entities": document.entities.len(),
            "roots": document.roots.len()
        }),
        Err(CodecError::ResourceLimit {
            resource,
            limit,
            observed,
        }) => json!({
            "class": "resource_limit",
            "resource": resource,
            "limit": limit,
            "observed": observed
        }),
        Err(error) => json!({"class": "error", "message": error.to_string()}),
    }
}

fn retained_bytes(stats: Stats) -> usize {
    let retained = i128::try_from(stats.bytes_allocated).unwrap_or(i128::MAX)
        - i128::try_from(stats.bytes_deallocated).unwrap_or(i128::MAX);
    usize::try_from(retained.max(0)).unwrap_or(usize::MAX)
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or_else(
            || "unknown".to_owned(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )
}

fn cpu_model() -> String {
    if env::consts::OS == "macos"
        && let Some(model) = command_line("sysctl", &["-n", "machdep.cpu.brand_string"])
    {
        return model;
    }
    if env::consts::OS == "linux"
        && let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo")
        && let Some(model) = cpuinfo.lines().find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|line| line.split_once(':'))
                .map(|(_, value)| value.trim().to_owned())
        })
    {
        return model;
    }
    "unknown".to_owned()
}

fn command_line(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|output| !output.is_empty())
}

fn output_path() -> Result<Option<PathBuf>, String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => Ok(None),
        Some("--output") => args
            .next()
            .map(PathBuf::from)
            .map(Some)
            .ok_or_else(|| "--output requires a path".to_owned()),
        Some(argument) => Err(format!("unknown argument {argument}")),
    }
}

fn fail(message: &str) -> ! {
    eprintln!("hostile-inputs: {message}");
    std::process::exit(1);
}
