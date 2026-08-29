#![doc = "Seeded conformance trials, fixtures, reports and failure reduction."]

pub mod layout_differential;

use nuif_api::{Session, Snapshot};
use nuif_codec::{
    CanonicalText, Canonicalizer, Decoder, DeterministicCbor, Encoder, canonical_hash,
};
use nuif_core::{
    Align, Color, ContextPredicate, Diagnostic, Document, Edges, Entity, EntityId, EntityKind,
    ExtensionDeclarations, Fidelity, FlowDirection, LayoutFamily, LayoutStyle, OpaqueEncoding,
    OpaquePayload, PropertyValue, ResponsiveOverride, Severity, ShapeKind, SizeIntent, TextContent,
    Token, UnknownKind, validate,
};
use nuif_layout::EvaluationContext;
use nuif_protocol::{Axis, Operation, Patch, Transaction, apply_patch, apply_patch_with_inverse};
use nuif_render::{RenderTarget, render_cpu};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialConfig {
    pub seed: u64,
    pub iterations: u32,
    pub operations_per_iteration: u32,
    pub snapshot_interval: u32,
    pub viewports: Vec<(u32, u32)>,
}

impl Default for TrialConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            iterations: 100,
            operations_per_iteration: 16,
            snapshot_interval: 1,
            viewports: vec![(360, 640), (768, 768), (1440, 900)],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReport {
    pub schema_version: u32,
    pub engine: EngineIdentity,
    pub profile: ProfileIdentity,
    pub trial: TrialIdentity,
    pub contexts: Vec<ContextReport>,
    pub issues: IssueSummary,
    pub fidelity: Vec<FidelityReport>,
    pub artifacts: Vec<String>,
    pub reproduction: Option<Reproduction>,
}

impl RunReport {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.issues.errors == 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub version: String,
    pub toolchain: String,
    pub source_revision: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileIdentity {
    pub capabilities: Vec<String>,
    pub encodings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialIdentity {
    pub seed: u64,
    pub iterations: u32,
    pub operations_per_iteration: u32,
    pub snapshot_interval: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReport {
    pub viewport: (u32, u32),
    pub canonical_hash: String,
    pub layout_boxes: usize,
    pub render_commands: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FidelityReport {
    pub context: (u32, u32),
    pub entity: EntityId,
    pub status: Fidelity,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueSummary {
    pub errors: u32,
    pub warnings: u32,
    pub information: u32,
    pub hints: u32,
    pub messages: Vec<Issue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub iteration: Option<u32>,
    pub context: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproduction {
    pub seed: u64,
    pub iteration: u32,
    pub minimized_operations: Vec<Operation>,
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the hard fixture is intentionally explicit and reviewable as one construction"
)]
pub fn responsive_card_fixture() -> Document {
    const SURFACE: EntityId = EntityId::new(0x10);
    const CARD: EntityId = EntityId::new(0x20);
    const MEDIA: EntityId = EntityId::new(0x21);
    const COPY: EntityId = EntityId::new(0x22);
    const BUTTON_COMPONENT: EntityId = EntityId::new(0x23);
    const BUTTON_INSTANCE: EntityId = EntityId::new(0x24);
    const UNKNOWN: EntityId = EntityId::new(0x25);
    const ICON: EntityId = EntityId::new(0x26);
    const COLOR_TOKEN: EntityId = EntityId::new(0x100);
    const SPACE_TOKEN: EntityId = EntityId::new(0x101);
    const RADIUS_TOKEN: EntityId = EntityId::new(0x102);

    let mut document = Document::empty(EntityId::new(1));
    document.extension_declarations = ExtensionDeclarations {
        used: BTreeSet::from(["vendor.probe".to_owned()]),
        required: BTreeSet::new(),
        fallback_kind: BTreeMap::from([("vendor.probe".to_owned(), "container".to_owned())]),
    };
    document.tokens.insert(
        COLOR_TOKEN,
        Token {
            id: COLOR_TOKEN,
            name: "color.card".to_owned(),
            value: PropertyValue::String("#f2f3f5".to_owned()),
        },
    );
    document.tokens.insert(
        SPACE_TOKEN,
        Token {
            id: SPACE_TOKEN,
            name: "space.card".to_owned(),
            value: PropertyValue::Real(24.0),
        },
    );
    document.tokens.insert(
        RADIUS_TOKEN,
        Token {
            id: RADIUS_TOKEN,
            name: "radius.card".to_owned(),
            value: PropertyValue::Real(8.0),
        },
    );

    let mut surface = Entity::new(SURFACE, EntityKind::Surface);
    surface.name = Some("Responsive card fixture".to_owned());
    surface.authored.width = SizeIntent::Fill;
    surface.authored.height = SizeIntent::Fill;
    surface.authored.layout = LayoutStyle {
        family: LayoutFamily::Stack,
        direction: FlowDirection::Column,
        gap: 16.0,
        padding: Edges {
            top: 16.0,
            right: 16.0,
            bottom: 16.0,
            left: 16.0,
        },
        align: Align::Stretch,
    };
    surface.children = vec![CARD, BUTTON_COMPONENT];

    let mut card = Entity::new(CARD, EntityKind::Component);
    card.name = Some("Card".to_owned());
    card.authored.width = SizeIntent::Fill;
    card.authored.height = SizeIntent::Fixed(280.0);
    card.authored.layout = LayoutStyle {
        family: LayoutFamily::Stack,
        direction: FlowDirection::Column,
        gap: 16.0,
        padding: Edges {
            top: 24.0,
            right: 24.0,
            bottom: 24.0,
            left: 24.0,
        },
        align: Align::Stretch,
    };
    card.authored.fill = Some(Color {
        red: 0.95,
        green: 0.96,
        blue: 0.98,
        alpha: 1.0,
    });
    card.authored
        .values
        .insert("token.color".to_owned(), PropertyValue::Token(COLOR_TOKEN));
    card.authored.values.insert(
        "token.spacing".to_owned(),
        PropertyValue::Token(SPACE_TOKEN),
    );
    card.authored.values.insert(
        "variant".to_owned(),
        PropertyValue::String("default".to_owned()),
    );
    card.authored
        .values
        .insert("enabled".to_owned(), PropertyValue::Boolean(true));
    card.authored.responsive.push(ResponsiveOverride {
        when: ContextPredicate {
            min_width: Some(768.0),
            max_width: None,
            theme: None,
        },
        direction: Some(FlowDirection::Row),
        gap: Some(24.0),
        width: None,
        height: None,
    });
    card.children = vec![MEDIA, COPY, BUTTON_INSTANCE, UNKNOWN];

    let mut media = Entity::new(MEDIA, EntityKind::Shape(ShapeKind::Rectangle));
    media.name = Some("Media".to_owned());
    media.authored.width = SizeIntent::Fill;
    media.authored.height = SizeIntent::Fill;
    media.authored.fill = Some(Color {
        red: 0.2,
        green: 0.35,
        blue: 0.8,
        alpha: 1.0,
    });

    let mut copy = Entity::new(COPY, EntityKind::Text);
    copy.name = Some("Copy".to_owned());
    copy.authored.width = SizeIntent::Fill;
    copy.authored.height = SizeIntent::Intrinsic;
    copy.authored.text = Some(TextContent {
        content: "Portable authored intent".to_owned(),
        font: nuif_text::PINNED_FONT_NAME.to_owned(),
        font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
        size: 18.0,
        line_height: 24.0,
    });

    let mut button_component = Entity::new(BUTTON_COMPONENT, EntityKind::Component);
    button_component.name = Some("Button".to_owned());
    button_component.authored.width = SizeIntent::Fixed(120.0);
    button_component.authored.height = SizeIntent::Fixed(40.0);
    button_component
        .authored
        .values
        .insert("state.hover".to_owned(), PropertyValue::Boolean(false));
    button_component
        .authored
        .values
        .insert("state.pressed".to_owned(), PropertyValue::Boolean(false));
    button_component.authored.values.insert(
        "token.radius".to_owned(),
        PropertyValue::Token(RADIUS_TOKEN),
    );
    button_component.children.push(ICON);

    let mut icon = Entity::new(ICON, EntityKind::Shape(ShapeKind::Path));
    icon.name = Some("Button icon".to_owned());
    icon.authored.width = SizeIntent::Fixed(16.0);
    icon.authored.height = SizeIntent::Fixed(16.0);
    icon.authored.fill = Some(Color {
        red: 0.1,
        green: 0.1,
        blue: 0.1,
        alpha: 1.0,
    });

    let mut button_instance = Entity::new(
        BUTTON_INSTANCE,
        EntityKind::Instance {
            component: BUTTON_COMPONENT,
        },
    );
    button_instance.name = Some("Card action".to_owned());
    button_instance.authored.width = SizeIntent::Fixed(120.0);
    button_instance.authored.height = SizeIntent::Fixed(40.0);

    let payload = OpaquePayload {
        encoding: OpaqueEncoding::Octets,
        bytes: vec![0, 1, 2, 3, 0xff],
    };
    let mut unknown = Entity::new(
        UNKNOWN,
        EntityKind::Unknown(UnknownKind {
            namespace: "vendor.probe".to_owned(),
            kind: "future_widget".to_owned(),
            schema_version: 7,
            payload: payload.clone(),
        }),
    );
    unknown.name = Some("Opaque probe".to_owned());
    unknown.authored.width = SizeIntent::Fixed(1.0);
    unknown.authored.height = SizeIntent::Fixed(1.0);
    unknown
        .extensions
        .0
        .insert("vendor.probe".to_owned(), payload);

    document.roots.push(SURFACE);
    for entity in [
        surface,
        card,
        media,
        copy,
        button_component,
        button_instance,
        unknown,
        icon,
    ] {
        document.entities.insert(entity.id, entity);
    }
    document
}

#[must_use]
pub fn run_trials(config: &TrialConfig) -> RunReport {
    let mut report = RunReport {
        schema_version: 1,
        engine: engine_identity(),
        profile: ProfileIdentity {
            capabilities: vec![
                "model".to_owned(),
                "operations".to_owned(),
                "layout-profile-0".to_owned(),
                "render-cpu-profile-0".to_owned(),
            ],
            encodings: vec!["nuif-text-0".to_owned(), "nuif-cbor-0".to_owned()],
        },
        trial: TrialIdentity {
            seed: config.seed,
            iterations: config.iterations,
            operations_per_iteration: config.operations_per_iteration,
            snapshot_interval: config.snapshot_interval.max(1),
        },
        contexts: Vec::new(),
        issues: IssueSummary::default(),
        fidelity: Vec::new(),
        artifacts: Vec::new(),
        reproduction: None,
    };
    let mut rng = TrialRng::new(config.seed);
    for iteration in 0..config.iterations {
        if let Err((code, message, operations)) = run_iteration(config, &mut rng, iteration) {
            report.issues.errors += 1;
            report.issues.messages.push(Issue {
                code,
                severity: Severity::Error,
                message,
                iteration: Some(iteration),
                context: None,
            });
            report.reproduction = Some(Reproduction {
                seed: config.seed,
                iteration,
                minimized_operations: operations,
            });
            return report;
        }
    }

    let document = responsive_card_fixture();
    let canonical = canonical_hash(&document).unwrap_or_else(|error| format!("error:{error}"));
    for diagnostic in validate(&document) {
        record_diagnostic(&mut report.issues, diagnostic, None);
    }
    for &(width, height) in &config.viewports {
        let context = fixture_context(width, height);
        let session = Session::new(document.clone());
        match session.snapshot(&context) {
            Ok(snapshot) => {
                for diagnostic in &snapshot.layout.diagnostics {
                    record_diagnostic(
                        &mut report.issues,
                        diagnostic.clone(),
                        Some((width, height)),
                    );
                }
                report
                    .fidelity
                    .extend(snapshot.scene.fidelity.iter().map(|entry| FidelityReport {
                        context: (width, height),
                        entity: entry.entity,
                        status: entry.status.clone(),
                    }));
                report
                    .contexts
                    .push(context_report((width, height), &snapshot));
            }
            Err(error) => {
                report.issues.errors += 1;
                report.issues.messages.push(Issue {
                    code: "SNAPSHOT_FAILED".to_owned(),
                    severity: Severity::Error,
                    message: error.to_string(),
                    iteration: None,
                    context: Some((width, height)),
                });
            }
        }
    }
    if report.contexts.is_empty() {
        report.contexts.push(ContextReport {
            viewport: (0, 0),
            canonical_hash: canonical,
            layout_boxes: 0,
            render_commands: 0,
        });
    }
    report
}

fn record_diagnostic(
    issues: &mut IssueSummary,
    diagnostic: Diagnostic,
    context: Option<(u32, u32)>,
) {
    match diagnostic.severity {
        Severity::Error => issues.errors += 1,
        Severity::Warning => issues.warnings += 1,
        Severity::Information => issues.information += 1,
        Severity::Hint => issues.hints += 1,
    }
    issues.messages.push(Issue {
        code: diagnostic.code,
        severity: diagnostic.severity,
        message: diagnostic.message,
        iteration: None,
        context,
    });
}

fn engine_identity() -> EngineIdentity {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source_revision =
        command_output("git", &["-C", path_text(&repository), "rev-parse", "HEAD"]);
    let dirty = Command::new("git")
        .args(["-C", path_text(&repository), "status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    EngineIdentity {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        toolchain: command_output("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned()),
        source_revision,
        dirty,
    }
}

fn command_output(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn path_text(path: &Path) -> &str {
    path.to_str().unwrap_or(".")
}

fn run_iteration(
    config: &TrialConfig,
    rng: &mut TrialRng,
    iteration: u32,
) -> Result<(), (String, String, Vec<Operation>)> {
    let operations = generate_operations(rng, config.operations_per_iteration, iteration);
    let trial_width = 360 + u32::try_from(rng.range(1081)).expect("the trial width range fits u32");
    let verify_snapshot = iteration.is_multiple_of(config.snapshot_interval.max(1));
    match verify_iteration(
        config.seed,
        iteration,
        &operations,
        trial_width,
        verify_snapshot,
    ) {
        Ok(()) => Ok(()),
        Err((code, message)) => {
            let minimized = minimize_operations(&operations, |candidate| {
                matches!(
                    verify_iteration(
                        config.seed,
                        iteration,
                        candidate,
                        trial_width,
                        verify_snapshot,
                    ),
                    Err((candidate_code, _)) if candidate_code == code
                )
            });
            Err((code, message, minimized))
        }
    }
}

fn verify_iteration(
    seed: u64,
    iteration: u32,
    operations: &[Operation],
    trial_width: u32,
    verify_snapshot: bool,
) -> Result<(), (String, String)> {
    let base = responsive_card_fixture();
    let patch = Patch {
        base_revision: canonical_hash(&base).ok(),
        transactions: vec![Transaction {
            id: (u128::from(seed) << 64) | u128::from(iteration),
            operations: operations.to_vec(),
        }],
    };
    let mut changed = base.clone();
    let inverse = apply_patch_with_inverse(&mut changed, &patch)
        .map_err(|error| ("TRIAL_APPLY_FAILED".to_owned(), error.to_string()))?;
    let mut replayed = base.clone();
    apply_patch(&mut replayed, &patch)
        .map_err(|error| ("TRIAL_REPLAY_FAILED".to_owned(), error.to_string()))?;
    if canonical_hash(&changed) != canonical_hash(&replayed) {
        return Err((
            "TRIAL_REPLAY_HASH_MISMATCH".to_owned(),
            "the same patch produced different canonical hashes".to_owned(),
        ));
    }
    apply_patch(&mut changed, &inverse)
        .map_err(|error| ("TRIAL_INVERSE_FAILED".to_owned(), error.to_string()))?;
    if changed != base {
        return Err((
            "TRIAL_INVERSE_MISMATCH".to_owned(),
            "applying the inverse did not restore the base document".to_owned(),
        ));
    }

    let text = CanonicalText
        .encode(&replayed)
        .map_err(|error| ("TRIAL_TEXT_ENCODE_FAILED".to_owned(), error.to_string()))?;
    if CanonicalText.canonicalize(&text).as_deref() != Ok(text.as_slice()) {
        return Err((
            "TRIAL_TEXT_FIXPOINT_FAILED".to_owned(),
            "canonical text did not reach an encode fixpoint".to_owned(),
        ));
    }
    let cbor = DeterministicCbor
        .encode(&replayed)
        .map_err(|error| ("TRIAL_CBOR_ENCODE_FAILED".to_owned(), error.to_string()))?;
    if DeterministicCbor.canonicalize(&cbor).as_deref() != Ok(cbor.as_slice())
        || DeterministicCbor.decode(&cbor).as_ref() != Ok(&replayed)
    {
        return Err((
            "TRIAL_CBOR_FIXPOINT_FAILED".to_owned(),
            "deterministic CBOR did not reach a lossless fixpoint".to_owned(),
        ));
    }
    if !verify_snapshot {
        return Ok(());
    }
    let context = fixture_context(trial_width, 640);
    let snapshot = Session::new(replayed)
        .snapshot(&context)
        .map_err(|error| ("TRIAL_SNAPSHOT_FAILED".to_owned(), error.to_string()))?;
    let target = RenderTarget {
        width: trial_width,
        height: 640,
        scale_factor: 1.0,
    };
    let rerender = render_cpu(&snapshot.scene, target)
        .map_err(|error| ("TRIAL_RERENDER_FAILED".to_owned(), error.to_string()))?;
    if snapshot.raster != rerender {
        return Err((
            "TRIAL_RASTER_NONDETERMINISTIC".to_owned(),
            "two CPU renders of one scene differ".to_owned(),
        ));
    }
    Ok(())
}

fn generate_operations(rng: &mut TrialRng, count: u32, iteration: u32) -> Vec<Operation> {
    let candidates = [
        EntityId::new(0x20),
        EntityId::new(0x21),
        EntityId::new(0x22),
        EntityId::new(0x24),
    ];
    (0..count)
        .map(|index| {
            let candidate_count =
                u64::try_from(candidates.len()).expect("candidate count fits u64");
            let candidate_index = usize::try_from(rng.range(candidate_count))
                .expect("generated candidate index fits usize");
            let entity = candidates[candidate_index];
            match rng.range(4) {
                0 => Operation::Rename {
                    entity,
                    name: Some(format!("trial-{iteration}-{index}-{}", rng.next())),
                },
                1 => Operation::SetValue {
                    entity,
                    key: "trial.counter".to_owned(),
                    value: PropertyValue::Integer(
                        i64::try_from(rng.range(10_000)).expect("trial integer fits i64"),
                    ),
                },
                2 => Operation::SetSize {
                    entity,
                    axis: Axis::Horizontal,
                    value: SizeIntent::Fixed(
                        1.0 + f64::from(
                            u32::try_from(rng.range(400)).expect("trial width fits u32"),
                        ),
                    ),
                },
                _ => Operation::SetSize {
                    entity,
                    axis: Axis::Vertical,
                    value: SizeIntent::Fixed(
                        1.0 + f64::from(
                            u32::try_from(rng.range(300)).expect("trial height fits u32"),
                        ),
                    ),
                },
            }
        })
        .collect()
}

fn fixture_context(width: u32, height: u32) -> EvaluationContext {
    nuif_api::profile_zero_context(f64::from(width), f64::from(height))
}

fn context_report(viewport: (u32, u32), snapshot: &Snapshot) -> ContextReport {
    ContextReport {
        viewport,
        canonical_hash: snapshot.canonical_hash.clone(),
        layout_boxes: snapshot.layout.boxes.len(),
        render_commands: snapshot.scene.commands.len(),
    }
}

#[derive(Clone, Copy, Debug)]
struct TrialRng(u64);

impl TrialRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, upper: u64) -> u64 {
        self.next() % upper.max(1)
    }
}

/// Reduces an operation list with classic complement-based delta debugging.
/// The predicate must return true while the failure remains reproducible.
#[must_use]
pub fn minimize_operations<F>(operations: &[Operation], mut still_fails: F) -> Vec<Operation>
where
    F: FnMut(&[Operation]) -> bool,
{
    if still_fails(&[]) {
        return Vec::new();
    }
    let mut current = operations.to_vec();
    let mut granularity = 2;
    while current.len() >= 2 {
        let chunk = current.len().div_ceil(granularity);
        let mut reduced = false;
        for start in (0..current.len()).step_by(chunk) {
            let end = (start + chunk).min(current.len());
            let candidate = current
                .iter()
                .enumerate()
                .filter(|(index, _)| *index < start || *index >= end)
                .map(|(_, operation)| operation.clone())
                .collect::<Vec<_>>();
            if !candidate.is_empty() && still_fails(&candidate) {
                current = candidate;
                granularity = granularity.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
        }
        if !reduced {
            if granularity >= current.len() {
                break;
            }
            granularity = (granularity * 2).min(current.len());
        }
    }
    current
}

#[must_use]
pub fn fixture_diagnostics() -> Vec<String> {
    validate(&responsive_card_fixture())
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_fixture_is_valid_and_preserves_opaque_bytes() {
        let document = responsive_card_fixture();
        assert!(fixture_diagnostics().is_empty());
        let bytes = DeterministicCbor.encode(&document).unwrap();
        let decoded = DeterministicCbor.decode(&bytes).unwrap();
        assert_eq!(decoded, document);
    }

    #[test]
    fn trial_loop_is_reproducible() {
        let config = TrialConfig {
            seed: 42,
            iterations: 5,
            operations_per_iteration: 8,
            ..TrialConfig::default()
        };
        let first = run_trials(&config);
        let second = run_trials(&config);
        assert!(first.passed(), "{:?}", first.issues.messages);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn reducer_finds_one_required_operation() {
        let operations = generate_operations(&mut TrialRng::new(7), 8, 0);
        let target = operations[3].clone();
        let reduced = minimize_operations(&operations, |candidate| candidate.contains(&target));
        assert_eq!(reduced, [target]);
    }

    #[test]
    fn reducer_can_find_an_operation_independent_failure() {
        let operations = generate_operations(&mut TrialRng::new(9), 4, 0);
        assert!(minimize_operations(&operations, |_| true).is_empty());
    }
}
