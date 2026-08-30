#![doc = "Source-backed browser capture and deterministic screenshot reconstruction baselines."]

use nuif_codec::{CodecError, canonical_hash};
use nuif_core::{
    AffineTransform, Align, Asset, AssetId, AssetKind, AssetPortability, AuthoredProperties,
    CURRENT_SCHEMA_VERSION, Color, ColorSpace, Document, Edges, Entity, EntityId, EntityKind,
    FlowDirection, ImageAsset, ImageCrop, ImageFit, ImagePaint, ImageSampling, LayoutFamily,
    LayoutStyle, Point, ResourceDigest, ResourceRole, Semantics, SizeIntent, TextContent,
};
use nuif_media::{MediaError, Rgba8Image, decode_png_rgba8};
use nuif_package::{NuifPackage, PackageError, PackageMode};
use nuif_protocol::{Anchor, Operation, Patch, Transaction};
use nuif_reconstruct::{
    Bounds, Confidence, CoordinateSpace, EvidenceClass, InferenceProvenance, OBSERVATION_PROFILE,
    Observation, ObservationBundle, ObservationError, ObservationId, ObservationValue, Omission,
    Proposal, Subject,
};
use nuif_text::PINNED_FONT_SHA256;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const BROWSER_CAPTURE_PROFILE: &str = "nuif-browser-capture-0";
pub const SCREENSHOT_CAPTURE_PROFILE: &str = "nuif-screenshot-baseline-0";
pub const PNG_DECODER_PROFILE: &str = nuif_media::PNG_RGBA8_PROFILE;
pub const MAX_CAPTURE_NODES: usize = 32_768;
pub const MAX_CAPTURE_RESOURCES: usize = 8_192;
pub const MAX_PNG_WIDTH: u32 = nuif_media::MAX_PNG_WIDTH;
pub const MAX_PNG_HEIGHT: u32 = nuif_media::MAX_PNG_HEIGHT;
pub const MAX_PNG_PIXELS: u64 = nuif_media::MAX_PNG_PIXELS;
pub const MAX_PNG_CHUNKS: usize = nuif_media::MAX_PNG_CHUNKS;
pub const MAX_COLOR_REGIONS: usize = 64;
pub const MAX_OCR_SPANS: usize = 8_192;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
    pub device_scale_factor: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub uri: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserNode {
    pub backend_node_id: u64,
    pub parent: Option<u64>,
    pub order: u32,
    pub tag: String,
    pub text: Option<String>,
    pub bounds: Bounds,
    pub background: Option<[f32; 4]>,
    pub accessible_role: Option<String>,
    pub accessible_name: Option<String>,
    pub source_span: Option<SourceSpan>,
    pub resource_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResource {
    pub url: String,
    pub final_url: String,
    pub media_type: String,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
    pub intrinsic_width: Option<u32>,
    pub intrinsic_height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCapture {
    pub schema_version: u32,
    pub profile: String,
    pub capture_id: String,
    pub adapter_version: String,
    pub source_url: String,
    pub viewport: Viewport,
    pub nodes: Vec<BrowserNode>,
    pub resources: Vec<BrowserResource>,
    #[serde(default)]
    pub omitted_runtime: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserCaptureResult {
    pub observations: ObservationBundle,
    pub proposal: Proposal,
}

/// Normalizes a source-backed browser capture and retains exact response
/// bodies in an authoring package. Input deliberately has no cookie, storage,
/// authorization-header, or credential fields.
///
/// # Errors
///
/// Rejects malformed graphs, unsafe URLs, invalid geometry, unsupported
/// package mode, resource overflow, or invalid observations.
pub fn normalize_browser_capture(
    capture: &BrowserCapture,
    package: &mut NuifPackage,
) -> Result<BrowserCaptureResult, CaptureError> {
    validate_browser_capture(capture, package)?;
    let viewport_id = format!("{}-viewport", capture.capture_id);
    let mut observations = Vec::new();
    let mut resource_digests = BTreeMap::new();
    for resource in &capture.resources {
        let original_url = sanitize_url(&resource.url)?;
        let final_url = sanitize_url(&resource.final_url)?;
        let resource_digest = package.add_embedded(
            resource.body.clone(),
            resource.media_type.clone(),
            ResourceRole::Source,
            None,
        )?;
        resource_digests.insert(original_url, resource_digest.clone());
        resource_digests.insert(final_url.clone(), resource_digest.clone());
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("resource-{resource_digest}")),
            evidence: EvidenceClass::ResolvedSource,
            subject: Some(Subject::Resource {
                digest: resource_digest.clone(),
            }),
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Resource {
                digest: Some(resource_digest),
                media_type: Some(resource.media_type.clone()),
                size: Some(u64::try_from(resource.body.len()).unwrap_or(u64::MAX)),
            },
            confidence: None,
            source: final_url,
        });
    }
    let ordered = topological_nodes(&capture.nodes)?;
    for node in &ordered {
        append_browser_node_observations(capture, node, &viewport_id, &mut observations)?;
    }
    let mut omissions = capture
        .omitted_runtime
        .iter()
        .map(|reason| Omission {
            category: "runtime-behavior".to_owned(),
            reason: reason.clone(),
            affected: None,
        })
        .collect::<Vec<_>>();
    if omissions.is_empty() {
        omissions.push(Omission {
            category: "runtime-behavior".to_owned(),
            reason: "event listeners, timers, worklets, video state, and arbitrary script behavior are outside the static capture profile".to_owned(),
            affected: None,
        });
    }
    let bundle = ObservationBundle {
        schema_version: 1,
        profile: OBSERVATION_PROFILE.to_owned(),
        capture_id: capture.capture_id.clone(),
        adapter: "browser-source-capture".to_owned(),
        adapter_version: capture.adapter_version.clone(),
        observations,
        omissions,
    };
    bundle.validate()?;
    let proposal = browser_proposal(capture, &ordered, &resource_digests, &bundle, package)?;
    Ok(BrowserCaptureResult {
        observations: bundle,
        proposal,
    })
}

fn validate_browser_capture(
    capture: &BrowserCapture,
    package: &NuifPackage,
) -> Result<(), CaptureError> {
    if capture.schema_version != 1
        || capture.profile != BROWSER_CAPTURE_PROFILE
        || !nuif_core::is_identifier(&capture.capture_id)
        || capture.adapter_version.is_empty()
    {
        return Err(CaptureError::InvalidCapture(
            "unsupported browser capture identity or profile".to_owned(),
        ));
    }
    validate_viewport(capture.viewport)?;
    sanitize_url(&capture.source_url)?;
    if package.mode != PackageMode::Authoring {
        return Err(CaptureError::InvalidCapture(
            "source capture requires an authoring package until resource export policy is reviewed"
                .to_owned(),
        ));
    }
    if !package.document.entities.is_empty() {
        return Err(CaptureError::InvalidCapture(
            "browser baseline requires an empty destination document".to_owned(),
        ));
    }
    if capture.nodes.len() > MAX_CAPTURE_NODES || capture.resources.len() > MAX_CAPTURE_RESOURCES {
        return Err(CaptureError::ResourceLimit);
    }
    let mut resources = BTreeSet::new();
    for resource in &capture.resources {
        let final_url = sanitize_url(&resource.final_url)?;
        sanitize_url(&resource.url)?;
        if !resources.insert(final_url) {
            return Err(CaptureError::InvalidCapture(
                "capture contains duplicate final resource URLs".to_owned(),
            ));
        }
    }
    for node in &capture.nodes {
        if let Some(background) = node.background
            && !background
                .iter()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
        {
            return Err(CaptureError::InvalidCapture(
                "computed background channels must be finite probabilities".to_owned(),
            ));
        }
        if let Some(span) = &node.source_span {
            sanitize_url(&span.uri)?;
            if span.start > span.end {
                return Err(CaptureError::InvalidCapture(
                    "source span start exceeds end".to_owned(),
                ));
            }
        }
        if let Some(resource_url) = &node.resource_url {
            sanitize_url(resource_url)?;
        }
    }
    topological_nodes(&capture.nodes).map(|_| ())
}

fn topological_nodes(nodes: &[BrowserNode]) -> Result<Vec<BrowserNode>, CaptureError> {
    let by_id = nodes
        .iter()
        .map(|node| (node.backend_node_id, node))
        .collect::<BTreeMap<_, _>>();
    if by_id.len() != nodes.len() || by_id.contains_key(&0) {
        return Err(CaptureError::InvalidCapture(
            "browser node IDs must be unique and non-zero".to_owned(),
        ));
    }
    let mut depths = BTreeMap::new();
    for node in nodes {
        validate_bounds(node.bounds)?;
        let mut depth = 0_usize;
        let mut current = node.parent;
        let mut path = BTreeSet::new();
        while let Some(parent) = current {
            if !path.insert(parent) {
                return Err(CaptureError::InvalidCapture(
                    "browser node hierarchy contains a cycle".to_owned(),
                ));
            }
            let parent_node = by_id.get(&parent).ok_or_else(|| {
                CaptureError::InvalidCapture(format!(
                    "browser node references missing parent {parent}"
                ))
            })?;
            depth = depth.saturating_add(1);
            if depth > 128 {
                return Err(CaptureError::ResourceLimit);
            }
            current = parent_node.parent;
        }
        depths.insert(node.backend_node_id, depth);
    }
    let mut ordered = nodes.to_vec();
    ordered.sort_by_key(|node| {
        (
            depths[&node.backend_node_id],
            node.parent,
            node.order,
            node.backend_node_id,
        )
    });
    Ok(ordered)
}

fn append_browser_node_observations(
    capture: &BrowserCapture,
    node: &BrowserNode,
    viewport_id: &str,
    observations: &mut Vec<Observation>,
) -> Result<(), CaptureError> {
    let prefix = format!("node-{}", node.backend_node_id);
    let coordinate_space = Some(CoordinateSpace::ViewportCssPixels {
        viewport: viewport_id.to_owned(),
    });
    observations.push(Observation {
        id: observation_id(&capture.capture_id, &format!("{prefix}-geometry")),
        evidence: EvidenceClass::ResolvedSource,
        subject: None,
        coordinate_space: coordinate_space.clone(),
        transform: None,
        value: ObservationValue::Geometry {
            bounds: node.bounds,
        },
        confidence: None,
        source: "dom-snapshot".to_owned(),
    });
    observations.push(Observation {
        id: observation_id(&capture.capture_id, &format!("{prefix}-hierarchy")),
        evidence: EvidenceClass::ResolvedSource,
        subject: None,
        coordinate_space: None,
        transform: None,
        value: ObservationValue::Hierarchy {
            parent: node.parent.map(|parent| {
                observation_id(&capture.capture_id, &format!("node-{parent}-geometry"))
            }),
            order: node.order,
        },
        confidence: None,
        source: "dom-snapshot".to_owned(),
    });
    if let Some(text) = &node.text {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("{prefix}-text")),
            evidence: EvidenceClass::ResolvedSource,
            subject: None,
            coordinate_space: coordinate_space.clone(),
            transform: None,
            value: ObservationValue::Text {
                content: text.clone(),
                bounds: Some(node.bounds),
            },
            confidence: None,
            source: "dom-text".to_owned(),
        });
    }
    if let Some(background) = node.background {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("{prefix}-background")),
            evidence: EvidenceClass::ResolvedSource,
            subject: None,
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Color { rgba: background },
            confidence: None,
            source: "computed-style".to_owned(),
        });
    }
    if let Some(role) = &node.accessible_role {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("{prefix}-accessibility")),
            evidence: EvidenceClass::ResolvedSource,
            subject: None,
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Accessibility {
                role: role.clone(),
                name: node.accessible_name.clone(),
            },
            confidence: None,
            source: "accessibility-tree".to_owned(),
        });
    }
    if let Some(span) = &node.source_span {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("{prefix}-source")),
            evidence: EvidenceClass::AuthoredSource,
            subject: None,
            coordinate_space: None,
            transform: None,
            value: ObservationValue::SourceSpan {
                uri: sanitize_url(&span.uri)?,
                start: span.start,
                end: span.end,
            },
            confidence: None,
            source: "source-map".to_owned(),
        });
    }
    Ok(())
}

fn browser_proposal(
    capture: &BrowserCapture,
    nodes: &[BrowserNode],
    resources: &BTreeMap<String, ResourceDigest>,
    bundle: &ObservationBundle,
    package: &NuifPackage,
) -> Result<Proposal, CaptureError> {
    let mut operations = Vec::new();
    let mut last_sibling = BTreeMap::new();
    let by_id = nodes
        .iter()
        .map(|node| (node.backend_node_id, node))
        .collect::<BTreeMap<_, _>>();
    for node in nodes {
        let entity_id = stable_entity_id(
            &capture.capture_id,
            &format!("node-{}", node.backend_node_id),
        );
        let parent_id = node
            .parent
            .map(|parent| stable_entity_id(&capture.capture_id, &format!("node-{parent}")));
        let anchor = last_sibling
            .get(&node.parent)
            .copied()
            .map_or(Anchor::Start, Anchor::After);
        let mut entity = browser_entity(
            node,
            parent_id.and_then(|id| package.document.entities.get(&id)),
            &by_id,
        );
        entity.id = entity_id;
        if let Some(resource_url) = &node.resource_url {
            let sanitized = sanitize_url(resource_url)?;
            if let Some(resource) = resources.get(&sanitized) {
                let asset_id = stable_asset_id(
                    &capture.capture_id,
                    &format!("asset-{}", node.backend_node_id),
                );
                let resource_record = capture
                    .resources
                    .iter()
                    .find(|item| sanitize_url(&item.final_url).ok().as_ref() == Some(&sanitized));
                operations.push(Operation::SetAsset {
                    asset: Asset {
                        schema_version: CURRENT_SCHEMA_VERSION,
                        id: asset_id,
                        name: Some(format!("captured-{}", node.backend_node_id)),
                        resource: Some(resource.clone()),
                        portability: AssetPortability::PrivateAuthoring,
                        kind: AssetKind::Image(ImageAsset {
                            width: resource_record
                                .and_then(|item| item.intrinsic_width)
                                .unwrap_or_else(|| positive_dimension(node.bounds.width)),
                            height: resource_record
                                .and_then(|item| item.intrinsic_height)
                                .unwrap_or_else(|| positive_dimension(node.bounds.height)),
                            decoder_profile: PNG_DECODER_PROFILE.to_owned(),
                        }),
                    },
                });
                entity.kind = EntityKind::Image;
                entity.authored.image = Some(default_image_paint(asset_id));
            }
        }
        operations.push(Operation::Insert {
            parent: parent_id,
            anchor,
            entity: Box::new(entity),
        });
        last_sibling.insert(node.parent, entity_id);
    }
    Ok(Proposal {
        schema_version: 1,
        provenance: InferenceProvenance {
            method: "browser-source-baseline".to_owned(),
            artifact: None,
            observations: bundle.ids(),
            confidence: Confidence::raw(1.0),
        },
        patch: Patch {
            base_revision: Some(canonical_hash(&package.document)?),
            transactions: vec![Transaction { id: 1, operations }],
        },
    })
}

fn browser_entity(
    node: &BrowserNode,
    _parent: Option<&Entity>,
    nodes: &BTreeMap<u64, &BrowserNode>,
) -> Entity {
    let parent_bounds = node
        .parent
        .and_then(|parent| nodes.get(&parent))
        .map(|parent| parent.bounds);
    let kind = if node.parent.is_none() {
        EntityKind::Surface
    } else if node.text.is_some()
        && !nodes
            .values()
            .any(|item| item.parent == Some(node.backend_node_id))
    {
        EntityKind::Text
    } else {
        EntityKind::Container
    };
    let mut entity = Entity::new(EntityId::new(1), kind);
    entity.name = Some(node.tag.to_ascii_lowercase());
    entity.authored = AuthoredProperties {
        width: SizeIntent::Fixed(node.bounds.width),
        height: SizeIntent::Fixed(node.bounds.height),
        position: Point {
            x: node.bounds.x - parent_bounds.map_or(0.0, |bounds| bounds.x),
            y: node.bounds.y - parent_bounds.map_or(0.0, |bounds| bounds.y),
        },
        layout: freeform_layout(),
        grid_placement: nuif_core::GridPlacement::default(),
        fill: node.background.map(model_color),
        text: node.text.as_ref().map(|content| TextContent {
            content: content.clone(),
            font: "Ahem substituted capture baseline".to_owned(),
            font_sha256: PINNED_FONT_SHA256.to_owned(),
            size: 16.0,
            line_height: 20.0,
        }),
        image: None,
        responsive: Vec::new(),
        values: BTreeMap::new(),
    };
    entity.semantics = Semantics {
        role: node.accessible_role.clone(),
        accessible_name: node.accessible_name.clone(),
        states: BTreeMap::new(),
    };
    entity
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrSpan {
    pub id: String,
    pub text: String,
    pub bounds: Bounds,
    pub raw_confidence: f64,
    pub engine: String,
    pub engine_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenshotCapture {
    pub schema_version: u32,
    pub profile: String,
    pub capture_id: String,
    pub viewport: Viewport,
    #[serde(with = "serde_bytes")]
    pub png: Vec<u8>,
    #[serde(default)]
    pub ocr: Vec<OcrSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorRegion {
    pub rgba: [u8; 4],
    pub bounds: Bounds,
    pub pixels: u64,
    pub density: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenshotAnalysis {
    pub observations: ObservationBundle,
    pub proposal: Proposal,
    pub screenshot_digest: ResourceDigest,
    pub regions: Vec<ColorRegion>,
}

/// Decodes a strict RGBA8 PNG, retains it as screenshot evidence, extracts
/// high-density flat-color regions, ingests explicit OCR results, and emits a
/// bounded typed reconstruction proposal.
///
/// # Errors
///
/// Rejects unsupported PNG semantics, malformed OCR, destination state,
/// resource limits, package policy, or invalid observations.
pub fn analyze_screenshot(
    capture: &ScreenshotCapture,
    package: &mut NuifPackage,
) -> Result<ScreenshotAnalysis, CaptureError> {
    validate_screenshot_capture(capture, package)?;
    let image = decode_png_rgba8(&capture.png)?;
    let expected_width = capture.viewport.width * capture.viewport.device_scale_factor;
    let expected_height = capture.viewport.height * capture.viewport.device_scale_factor;
    if (f64::from(image.width) - expected_width).abs() > f64::EPSILON
        || (f64::from(image.height) - expected_height).abs() > f64::EPSILON
    {
        return Err(CaptureError::InvalidCapture(
            "screenshot pixel dimensions must match the baseline viewport".to_owned(),
        ));
    }
    let screenshot_digest =
        package.add_embedded(capture.png.clone(), "image/png", ResourceRole::Source, None)?;
    let regions = extract_color_regions(&image);
    let observations = screenshot_observations(capture, &screenshot_digest, &regions)?;
    let proposal = screenshot_proposal(capture, &observations, &regions, &package.document)?;
    Ok(ScreenshotAnalysis {
        observations,
        proposal,
        screenshot_digest,
        regions,
    })
}

fn validate_screenshot_capture(
    capture: &ScreenshotCapture,
    package: &NuifPackage,
) -> Result<(), CaptureError> {
    if capture.schema_version != 1
        || capture.profile != SCREENSHOT_CAPTURE_PROFILE
        || !nuif_core::is_identifier(&capture.capture_id)
    {
        return Err(CaptureError::InvalidCapture(
            "unsupported screenshot capture identity or profile".to_owned(),
        ));
    }
    validate_viewport(capture.viewport)?;
    if !package.document.entities.is_empty() {
        return Err(CaptureError::InvalidCapture(
            "screenshot baseline requires an empty destination document".to_owned(),
        ));
    }
    if capture.ocr.len() > MAX_OCR_SPANS {
        return Err(CaptureError::ResourceLimit);
    }
    let mut ids = BTreeSet::new();
    for span in &capture.ocr {
        validate_bounds(span.bounds)?;
        if !nuif_core::is_identifier(&span.id)
            || !nuif_core::is_identifier(&span.engine)
            || span.engine_version.is_empty()
            || !span.raw_confidence.is_finite()
            || !(0.0..=1.0).contains(&span.raw_confidence)
            || !ids.insert(&span.id)
        {
            return Err(CaptureError::InvalidCapture(
                "OCR spans require unique identities, engine provenance, finite bounds, and probability confidence"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct RegionStats {
    count: u32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

fn extract_color_regions(image: &Rgba8Image) -> Vec<ColorRegion> {
    let mut colors = BTreeMap::<[u8; 4], RegionStats>::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let index = (y as usize * image.width as usize + x as usize) * 4;
            let rgba: [u8; 4] = image.rgba[index..index + 4].try_into().unwrap();
            colors
                .entry(rgba)
                .and_modify(|stats| {
                    stats.count += 1;
                    stats.min_x = stats.min_x.min(x);
                    stats.min_y = stats.min_y.min(y);
                    stats.max_x = stats.max_x.max(x);
                    stats.max_y = stats.max_y.max(y);
                })
                .or_insert(RegionStats {
                    count: 1,
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                });
        }
    }
    let min_pixels = (u64::from(image.width) * u64::from(image.height) / 10_000).max(16);
    let mut regions = colors
        .into_iter()
        .filter_map(|(rgba, stats)| {
            let width = stats.max_x - stats.min_x + 1;
            let height = stats.max_y - stats.min_y + 1;
            let area = width * height;
            let density = f64::from(stats.count) / f64::from(area);
            (u64::from(stats.count) >= min_pixels && density >= 0.85).then_some(ColorRegion {
                rgba,
                bounds: Bounds {
                    x: f64::from(stats.min_x),
                    y: f64::from(stats.min_y),
                    width: f64::from(width),
                    height: f64::from(height),
                },
                pixels: u64::from(stats.count),
                density,
            })
        })
        .collect::<Vec<_>>();
    regions.sort_by(|left, right| {
        right
            .pixels
            .cmp(&left.pixels)
            .then_with(|| left.rgba.cmp(&right.rgba))
    });
    regions.truncate(MAX_COLOR_REGIONS);
    regions
}

fn screenshot_observations(
    capture: &ScreenshotCapture,
    screenshot: &ResourceDigest,
    regions: &[ColorRegion],
) -> Result<ObservationBundle, CaptureError> {
    let viewport_id = format!("{}-viewport", capture.capture_id);
    let coordinate_space = Some(CoordinateSpace::ViewportCssPixels {
        viewport: viewport_id,
    });
    let mut observations = vec![
        Observation {
            id: observation_id(&capture.capture_id, "screenshot-resource"),
            evidence: EvidenceClass::ObservedPixels,
            subject: Some(Subject::Resource {
                digest: screenshot.clone(),
            }),
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Resource {
                digest: Some(screenshot.clone()),
                media_type: Some("image/png".to_owned()),
                size: Some(capture.png.len() as u64),
            },
            confidence: Some(Confidence::raw(1.0)),
            source: "screenshot-bytes".to_owned(),
        },
        Observation {
            id: observation_id(&capture.capture_id, "viewport-geometry"),
            evidence: EvidenceClass::ObservedPixels,
            subject: None,
            coordinate_space: coordinate_space.clone(),
            transform: None,
            value: ObservationValue::Geometry {
                bounds: Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: capture.viewport.width,
                    height: capture.viewport.height,
                },
            },
            confidence: Some(Confidence::raw(1.0)),
            source: "screenshot-pixels".to_owned(),
        },
    ];
    for (index, region) in regions.iter().enumerate() {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("region-{index}-geometry")),
            evidence: EvidenceClass::ObservedPixels,
            subject: None,
            coordinate_space: coordinate_space.clone(),
            transform: None,
            value: ObservationValue::Geometry {
                bounds: css_bounds(region.bounds, capture.viewport.device_scale_factor),
            },
            confidence: Some(Confidence::raw(region.density)),
            source: "flat-color-segmentation".to_owned(),
        });
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("region-{index}-color")),
            evidence: EvidenceClass::ObservedPixels,
            subject: None,
            coordinate_space: None,
            transform: None,
            value: ObservationValue::Color {
                rgba: region.rgba.map(|channel| f32::from(channel) / 255.0),
            },
            confidence: Some(Confidence::raw(region.density)),
            source: "flat-color-segmentation".to_owned(),
        });
    }
    for span in &capture.ocr {
        observations.push(Observation {
            id: observation_id(&capture.capture_id, &format!("ocr-{}", span.id)),
            evidence: EvidenceClass::Inferred,
            subject: None,
            coordinate_space: coordinate_space.clone(),
            transform: None,
            value: ObservationValue::Text {
                content: span.text.clone(),
                bounds: Some(span.bounds),
            },
            confidence: Some(Confidence::raw(span.raw_confidence)),
            source: format!("{}-{}", span.engine, span.engine_version),
        });
    }
    let bundle = ObservationBundle {
        schema_version: 1,
        profile: OBSERVATION_PROFILE.to_owned(),
        capture_id: capture.capture_id.clone(),
        adapter: "screenshot-baseline".to_owned(),
        adapter_version: "1".to_owned(),
        observations,
        omissions: screenshot_omissions(),
    };
    bundle.validate()?;
    Ok(bundle)
}

fn screenshot_omissions() -> Vec<Omission> {
    vec![
        Omission {
            category: "authored-structure".to_owned(),
            reason: "pixels do not prove original hierarchy, constraints, or component identity"
                .to_owned(),
            affected: None,
        },
        Omission {
            category: "source-resources".to_owned(),
            reason: "pixels do not recover original image or font bytes".to_owned(),
            affected: None,
        },
        Omission {
            category: "responsive-behavior".to_owned(),
            reason: "one viewport cannot prove responsive rules or hidden states".to_owned(),
            affected: None,
        },
        Omission {
            category: "interaction".to_owned(),
            reason: "a still image does not contain executable interaction behavior".to_owned(),
            affected: None,
        },
    ]
}

fn screenshot_proposal(
    capture: &ScreenshotCapture,
    observations: &ObservationBundle,
    regions: &[ColorRegion],
    document: &Document,
) -> Result<Proposal, CaptureError> {
    let root_id = stable_entity_id(&capture.capture_id, "root");
    let background = regions.first().map_or([255; 4], |region| region.rgba);
    let mut root = Entity::new(root_id, EntityKind::Surface);
    root.name = Some("reconstructed viewport".to_owned());
    root.authored.width = SizeIntent::Fixed(capture.viewport.width);
    root.authored.height = SizeIntent::Fixed(capture.viewport.height);
    root.authored.fill = Some(model_color(
        background.map(|value| f32::from(value) / 255.0),
    ));
    let mut operations = vec![Operation::Insert {
        parent: None,
        anchor: Anchor::Start,
        entity: Box::new(root),
    }];
    let mut previous = None;
    for (index, region) in regions.iter().enumerate().skip(1) {
        let bounds = css_bounds(region.bounds, capture.viewport.device_scale_factor);
        let id = stable_entity_id(&capture.capture_id, &format!("region-{index}"));
        let mut entity = Entity::new(id, EntityKind::Shape(nuif_core::ShapeKind::Rectangle));
        entity.name = Some(format!("inferred flat region {index}"));
        entity.authored.position = Point {
            x: bounds.x,
            y: bounds.y,
        };
        entity.authored.width = SizeIntent::Fixed(bounds.width);
        entity.authored.height = SizeIntent::Fixed(bounds.height);
        entity.authored.fill = Some(model_color(
            region.rgba.map(|value| f32::from(value) / 255.0),
        ));
        operations.push(Operation::Insert {
            parent: Some(root_id),
            anchor: previous.map_or(Anchor::Start, Anchor::After),
            entity: Box::new(entity),
        });
        previous = Some(id);
    }
    for span in &capture.ocr {
        let id = stable_entity_id(&capture.capture_id, &format!("ocr-{}", span.id));
        let mut entity = Entity::new(id, EntityKind::Text);
        entity.name = Some("inferred text".to_owned());
        entity.authored.position = Point {
            x: span.bounds.x,
            y: span.bounds.y,
        };
        entity.authored.width = SizeIntent::Fixed(span.bounds.width);
        entity.authored.height = SizeIntent::Fixed(span.bounds.height);
        entity.authored.text = Some(TextContent {
            content: span.text.clone(),
            font: "Ahem substituted screenshot baseline".to_owned(),
            font_sha256: PINNED_FONT_SHA256.to_owned(),
            size: (span.bounds.height * 0.8).max(1.0),
            line_height: span.bounds.height.max(1.0),
        });
        operations.push(Operation::Insert {
            parent: Some(root_id),
            anchor: previous.map_or(Anchor::Start, Anchor::After),
            entity: Box::new(entity),
        });
        previous = Some(id);
    }
    let confidence = capture
        .ocr
        .iter()
        .map(|span| span.raw_confidence)
        .chain(regions.iter().map(|region| region.density))
        .reduce(f64::min)
        .unwrap_or(0.5);
    Ok(Proposal {
        schema_version: 1,
        provenance: InferenceProvenance {
            method: "deterministic-screenshot-baseline".to_owned(),
            artifact: None,
            observations: observations.ids(),
            confidence: Confidence::raw(confidence),
        },
        patch: Patch {
            base_revision: Some(canonical_hash(document)?),
            transactions: vec![Transaction { id: 1, operations }],
        },
    })
}

fn default_image_paint(asset: AssetId) -> ImagePaint {
    ImagePaint {
        asset,
        fit: ImageFit::Fill,
        crop: ImageCrop {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        transform: AffineTransform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        },
        sampling: ImageSampling::Linear,
        opacity: 1.0,
        color_conversion: "srgb".to_owned(),
    }
}

fn freeform_layout() -> LayoutStyle {
    LayoutStyle {
        family: LayoutFamily::Freeform,
        direction: FlowDirection::Row,
        gap: 0.0,
        padding: Edges::default(),
        align: Align::Start,
        ..LayoutStyle::default()
    }
}

fn model_color(rgba: [f32; 4]) -> Color {
    Color {
        space: ColorSpace::Srgb,
        red: rgba[0],
        green: rgba[1],
        blue: rgba[2],
        alpha: rgba[3],
    }
}

fn css_bounds(bounds: Bounds, device_scale_factor: f64) -> Bounds {
    Bounds {
        x: bounds.x / device_scale_factor,
        y: bounds.y / device_scale_factor,
        width: bounds.width / device_scale_factor,
        height: bounds.height / device_scale_factor,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the branch proves the rounded positive value fits u32"
)]
fn positive_dimension(value: f64) -> u32 {
    if value.is_finite() && value >= 1.0 && value <= f64::from(u32::MAX) {
        value.round() as u32
    } else {
        1
    }
}

fn validate_viewport(viewport: Viewport) -> Result<(), CaptureError> {
    if viewport.width.is_finite()
        && viewport.height.is_finite()
        && viewport.device_scale_factor.is_finite()
        && viewport.width > 0.0
        && viewport.height > 0.0
        && viewport.device_scale_factor > 0.0
        && viewport.width * viewport.device_scale_factor <= f64::from(MAX_PNG_WIDTH)
        && viewport.height * viewport.device_scale_factor <= f64::from(MAX_PNG_HEIGHT)
    {
        Ok(())
    } else {
        Err(CaptureError::InvalidCapture(
            "viewport dimensions and scale must be finite, positive, and bounded".to_owned(),
        ))
    }
}

fn validate_bounds(bounds: Bounds) -> Result<(), CaptureError> {
    if [bounds.x, bounds.y, bounds.width, bounds.height]
        .into_iter()
        .all(f64::is_finite)
        && bounds.width >= 0.0
        && bounds.height >= 0.0
    {
        Ok(())
    } else {
        Err(CaptureError::InvalidCapture(
            "capture bounds must be finite and non-negative".to_owned(),
        ))
    }
}

fn sanitize_url(value: &str) -> Result<String, CaptureError> {
    if value.is_empty() || value.len() > 4_096 || !value.is_ascii() {
        return Err(CaptureError::InvalidCapture(
            "source URL is invalid".to_owned(),
        ));
    }
    let base = value.split(['?', '#']).next().unwrap_or(value);
    let authority_has_credentials = base
        .split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .is_some_and(|authority| authority.contains('@'));
    if authority_has_credentials
        || base.bytes().any(|byte| byte.is_ascii_control())
        || !base.contains(':')
    {
        return Err(CaptureError::InvalidCapture(
            "source URL is relative, credential-bearing, or contains controls".to_owned(),
        ));
    }
    Ok(base.to_owned())
}

fn observation_id(capture: &str, item: &str) -> ObservationId {
    ObservationId(format!("{capture}-{item}"))
}

fn stable_entity_id(capture: &str, item: &str) -> EntityId {
    EntityId::new(stable_u128(capture, item))
}

fn stable_asset_id(capture: &str, item: &str) -> AssetId {
    AssetId::new(stable_u128(capture, item))
}

fn stable_u128(capture: &str, item: &str) -> u128 {
    let hash = Sha256::digest(format!("{capture}\0{item}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    u128::from_be_bytes(bytes)
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum CaptureError {
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Observation(#[from] ObservationError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error("invalid capture: {0}")]
    InvalidCapture(String),
    #[error("capture exceeds a configured resource limit")]
    ResourceLimit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_reconstruct::{ProposalPolicy, apply_proposal};
    use png::{BitDepth, ColorType};
    use std::io::Cursor;

    fn png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            encoder.set_filter(png::Filter::NoFilter);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(pixels).unwrap();
        }
        bytes
    }

    #[test]
    fn screenshot_baseline_is_deterministic_and_explicitly_inferred() {
        let pixels = [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ];
        let capture = ScreenshotCapture {
            schema_version: 1,
            profile: SCREENSHOT_CAPTURE_PROFILE.to_owned(),
            capture_id: "shot-1".to_owned(),
            viewport: Viewport {
                width: 2.0,
                height: 2.0,
                device_scale_factor: 1.0,
            },
            png: png(2, 2, &pixels),
            ocr: vec![OcrSpan {
                id: "title".to_owned(),
                text: "Hello".to_owned(),
                bounds: Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 2.0,
                    height: 1.0,
                },
                raw_confidence: 0.9,
                engine: "fixture-ocr".to_owned(),
                engine_version: "1".to_owned(),
            }],
        };
        let mut package =
            NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
        let first = analyze_screenshot(&capture, &mut package).unwrap();
        let mut second_package =
            NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
        let second = analyze_screenshot(&capture, &mut second_package).unwrap();
        assert_eq!(first, second);
        assert!(
            first
                .observations
                .observations
                .iter()
                .any(|observation| observation.evidence == EvidenceClass::Inferred)
        );
        assert!(
            first
                .observations
                .omissions
                .iter()
                .any(|omission| omission.category == "source-resources")
        );
        apply_proposal(
            &mut package.document,
            &first.observations,
            &first.proposal,
            &ProposalPolicy::default(),
        )
        .unwrap();
        assert_eq!(package.document.roots.len(), 1);
        assert!(package.encode().is_ok());
    }

    #[test]
    fn browser_capture_retains_sources_without_runtime_authority() {
        let capture = BrowserCapture {
            schema_version: 1,
            profile: BROWSER_CAPTURE_PROFILE.to_owned(),
            capture_id: "browser-1".to_owned(),
            adapter_version: "1".to_owned(),
            source_url: "https://example.invalid/?token=redacted".to_owned(),
            viewport: Viewport {
                width: 100.0,
                height: 100.0,
                device_scale_factor: 1.0,
            },
            nodes: vec![BrowserNode {
                backend_node_id: 1,
                parent: None,
                order: 0,
                tag: "main".to_owned(),
                text: None,
                bounds: Bounds {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0,
                },
                background: Some([1.0; 4]),
                accessible_role: Some("main".to_owned()),
                accessible_name: None,
                source_span: None,
                resource_url: None,
            }],
            resources: vec![BrowserResource {
                url: "https://example.invalid/app.css".to_owned(),
                final_url: "https://example.invalid/app.css?signature=secret".to_owned(),
                media_type: "text/css".to_owned(),
                body: b"main { background: white; }".to_vec(),
                intrinsic_width: None,
                intrinsic_height: None,
            }],
            omitted_runtime: Vec::new(),
        };
        let mut package =
            NuifPackage::new(Document::empty(EntityId::new(1)), PackageMode::Authoring);
        let result = normalize_browser_capture(&capture, &mut package).unwrap();
        assert_eq!(package.resources.len(), 1);
        assert!(
            result
                .observations
                .omissions
                .iter()
                .any(|omission| omission.category == "runtime-behavior")
        );
        assert!(!format!("{:?}", result.observations).contains("secret"));
        apply_proposal(
            &mut package.document,
            &result.observations,
            &result.proposal,
            &ProposalPolicy::default(),
        )
        .unwrap();
        assert_eq!(package.document.roots.len(), 1);
        assert!(package.encode().is_ok());
    }

    #[test]
    fn png_profile_rejects_color_conversion_ambiguity() {
        let bytes = png(1, 1, &[0, 0, 0, 255]);
        assert!(decode_png_rgba8(&bytes).is_ok());
        let mut indexed = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut indexed), 1, 1);
            encoder.set_color(ColorType::Grayscale);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[0]).unwrap();
        }
        assert!(matches!(
            decode_png_rgba8(&indexed),
            Err(MediaError::UnsupportedPng(_))
        ));
    }
}
