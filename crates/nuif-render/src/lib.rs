#![doc = "Renderer-independent scene lowering and deterministic CPU rasterization."]

use nuif_core::{
    AffineTransform, AssetId, AssetKind, Color as ModelColor, ColorSpace, Document, EntityId,
    EntityKind, Fidelity, ImageCrop, ImageFit, ImagePaint, ImageSampling, ResourceDigest,
    ShapeKind, TextFontBinding, resolve_text_font_binding,
};
use nuif_layout::{EvaluationContext, LayoutSnapshot, Rect, WritingDirection};
use nuif_media::{
    MediaError, PNG_BASIC_RGBA8_PROFILE, PNG_RGBA8_PROFILE, Rgba8Image, decode_png_profile,
    png_profile_decoded_bytes,
};
use nuif_text::{
    CLUSTER_UNIT, GlyphOutline, MAX_SHAPING_CODEPOINTS, OUTLINE_COORDINATE_DENOMINATOR,
    OUTLINE_EXTRACTOR_NAME, OUTLINE_EXTRACTOR_VERSION, OutlineCommand, OutlinePoint,
    PINNED_FONT_SHA256, ResourceFont, ShapeRequest, ShapedRun, TextDirection, TextError,
    outline_glyph, shape_hard_lines,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Cursor;
use thiserror::Error;
use zeno::{Command as ZenoCommand, Fill, Format, Mask, PathBuilder, Point as ZenoPoint};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub space: ColorSpace,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl From<ModelColor> for Color {
    fn from(value: ModelColor) -> Self {
        Self {
            space: value.space,
            red: value.red,
            green: value.green,
            blue: value.blue,
            alpha: value.alpha,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "command")]
pub enum DrawCommand {
    Rect {
        entity: EntityId,
        rect: Rect,
        fill: Color,
    },
    Ellipse {
        entity: EntityId,
        rect: Rect,
        fill: Color,
    },
    Text {
        entity: EntityId,
        rect: Rect,
        run: Box<ShapedRun>,
        outlines: Box<BTreeMap<u32, GlyphOutline>>,
        color: Color,
    },
    Image {
        entity: EntityId,
        rect: Rect,
        asset: AssetId,
        surface: ImageSurfaceId,
        fit: ImageFit,
        crop: ImageCrop,
        transform: AffineTransform,
        sampling: ImageSampling,
        opacity: f32,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageSurfaceId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSurface {
    pub resource: ResourceDigest,
    pub decoder_profile: String,
    pub image: Rgba8Image,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderScene {
    pub commands: Vec<DrawCommand>,
    pub fidelity: Vec<RenderFidelity>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub image_surfaces: BTreeMap<ImageSurfaceId, ImageSurface>,
}

pub const MAX_SCENE_DECODED_IMAGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Default)]
struct ImageSurfaceCache {
    decoded_bytes: usize,
    ids: BTreeMap<AssetId, ImageSurfaceId>,
}

struct ImageSurfaceRequest<'a> {
    entity: EntityId,
    asset: AssetId,
    resource: &'a ResourceDigest,
    decoder_profile: &'a str,
    width: u32,
    height: u32,
    bytes: &'a [u8],
}

enum TextFontLowering<'a> {
    Render {
        sha256: &'a str,
        asset: Option<AssetId>,
        fidelity: Fidelity,
    },
    Skip(Fidelity),
}

enum ResolvedTextFace<'a> {
    Pinned,
    Resource(ResourceFont<'a>),
    Skip(Fidelity),
}

impl ResolvedTextFace<'_> {
    fn shape_hard_lines(&self, request: &ShapeRequest<'_>) -> Result<Vec<ShapedRun>, TextError> {
        match self {
            Self::Pinned => shape_hard_lines(request),
            Self::Resource(font) => font.shape_hard_lines(request),
            Self::Skip(_) => Ok(Vec::new()),
        }
    }

    fn outline_glyph(&self, glyph_id: u32) -> Result<GlyphOutline, TextError> {
        match self {
            Self::Pinned => outline_glyph(glyph_id),
            Self::Resource(font) => font.outline_glyph(glyph_id),
            Self::Skip(_) => Err(TextError::GlyphOutlineUnavailable { glyph_id }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderFidelity {
    pub entity: Option<EntityId>,
    pub pointer: String,
    pub status: Fidelity,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl RasterImage {
    /// Encodes the raster as a deterministic RGBA PNG.
    ///
    /// # Errors
    ///
    /// Returns an encoding error if the PNG writer rejects the image.
    pub fn to_png(&self) -> Result<Vec<u8>, RenderError> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut output), self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_filter(png::Filter::NoFilter);
            let mut writer = encoder
                .write_header()
                .map_err(|error| RenderError::Png(error.to_string()))?;
            writer
                .write_image_data(&self.rgba)
                .map_err(|error| RenderError::Png(error.to_string()))?;
        }
        Ok(output)
    }
}

pub trait Renderer {
    type Output;
    type Error;

    /// Renders a lowered NUIF scene into the requested target.
    ///
    /// # Errors
    ///
    /// Returns an implementation-defined error when the target is
    /// unsupported, resources cannot be allocated, or rendering fails.
    fn render(
        &mut self,
        scene: &RenderScene,
        target: RenderTarget,
    ) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum RenderError {
    #[error("render target is empty or exceeds the reference budget")]
    InvalidTarget,
    #[error("render scene contains invalid geometry or color for entity {entity}")]
    InvalidScene { entity: EntityId },
    #[error("text shaping failed for entity {entity}: {source}")]
    Text {
        entity: EntityId,
        #[source]
        source: TextError,
    },
    #[error("invalid font binding for entity {entity}: {reason}")]
    InvalidFontBinding {
        entity: EntityId,
        reason: &'static str,
    },
    #[error("image decoding failed for entity {entity}: {source}")]
    Image {
        entity: EntityId,
        #[source]
        source: MediaError,
    },
    #[error("image metadata or decoded dimensions are inconsistent for entity {entity}")]
    InvalidImage { entity: EntityId },
    #[error(
        "decoded image-surface budget exceeded for entity {entity}: limit {limit}, observed {observed}"
    )]
    ImageResourceLimit {
        entity: EntityId,
        limit: usize,
        observed: usize,
    },
    #[error("PNG encoding failed: {0}")]
    Png(String),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CpuRenderer;

impl Renderer for CpuRenderer {
    type Output = RasterImage;
    type Error = RenderError;

    fn render(
        &mut self,
        scene: &RenderScene,
        target: RenderTarget,
    ) -> Result<Self::Output, Self::Error> {
        render_cpu(scene, target)
    }
}

/// Lowers resolved layout and text runs into a renderer-independent scene.
///
/// # Errors
///
/// Returns a typed text error when a document requests a font absent from the
/// evaluation context or outside the pinned profile-0 resource set.
pub fn build_scene(
    document: &Document,
    layout: &LayoutSnapshot,
    context: &EvaluationContext,
) -> Result<RenderScene, RenderError> {
    build_scene_with_resources(document, layout, context, |_| None)
}

/// Lowers a scene while resolving exact encoded image resources explicitly.
/// The resolver grants no filesystem or network authority; callers decide how
/// a verified digest maps to bytes.
///
/// # Errors
///
/// Returns typed text, image decode or image metadata errors. Missing resources
/// and unsupported decoder/paint profiles remain item-level fidelity records.
pub fn build_scene_with_resources<'a>(
    document: &Document,
    layout: &LayoutSnapshot,
    context: &EvaluationContext,
    resolve: impl Fn(&ResourceDigest) -> Option<&'a [u8]>,
) -> Result<RenderScene, RenderError> {
    let mut scene = RenderScene {
        commands: Vec::with_capacity(document.entities.len()),
        fidelity: Vec::with_capacity(document.entities.len()),
        image_surfaces: BTreeMap::new(),
    };
    let mut image_cache = ImageSurfaceCache::default();
    for namespace in document.extensions.0.keys() {
        scene.fidelity.push(RenderFidelity {
            entity: None,
            pointer: format!("/extensions/{namespace}"),
            status: Fidelity::PreservedUnrenderable {
                namespace: namespace.clone(),
            },
        });
    }
    for (id, entity) in &document.entities {
        let Some(rect) = layout.boxes.get(id).copied() else {
            continue;
        };
        for namespace in entity.extensions.0.keys() {
            scene.fidelity.push(RenderFidelity {
                entity: Some(*id),
                pointer: format!("/entities/{id}/extensions/{namespace}"),
                status: Fidelity::PreservedUnrenderable {
                    namespace: namespace.clone(),
                },
            });
        }
        if matches!(entity.kind, EntityKind::Image) {
            lower_image(
                &mut scene,
                document,
                *id,
                rect,
                entity.authored.image.as_ref(),
                &resolve,
                &mut image_cache,
            )?;
        } else {
            lower_entity_visual(&mut scene, *id, &entity.kind, rect, entity.authored.fill);
        }
        if let Some(text) = &entity.authored.text {
            lower_text(&mut scene, document, *id, rect, text, context, &resolve)?;
        }
    }
    Ok(scene)
}

fn lower_entity_visual(
    scene: &mut RenderScene,
    id: EntityId,
    kind: &EntityKind,
    rect: Rect,
    fill: Option<ModelColor>,
) {
    match kind {
        EntityKind::Unknown(unknown) => scene.fidelity.push(RenderFidelity {
            entity: Some(id),
            pointer: format!("/entities/{id}/kind"),
            status: Fidelity::PreservedUnrenderable {
                namespace: unknown.namespace.clone(),
            },
        }),
        EntityKind::Shape(ShapeKind::Path) => {
            scene.fidelity.push(RenderFidelity {
                entity: Some(id),
                pointer: format!("/entities/{id}/kind"),
                status: Fidelity::Unsupported {
                    reason: "profile 0 has no authored path-geometry field".to_owned(),
                },
            });
        }
        EntityKind::Instance { .. } => scene.fidelity.push(RenderFidelity {
            entity: Some(id),
            pointer: format!("/entities/{id}/kind"),
            status: Fidelity::Unsupported {
                reason: "profile 0 does not materialize component instances".to_owned(),
            },
        }),
        _ => {
            if let Some(fill) = fill {
                let command = if matches!(kind, EntityKind::Shape(ShapeKind::Ellipse)) {
                    DrawCommand::Ellipse {
                        entity: id,
                        rect,
                        fill: fill.into(),
                    }
                } else {
                    DrawCommand::Rect {
                        entity: id,
                        rect,
                        fill: fill.into(),
                    }
                };
                scene.commands.push(command);
            }
        }
    }
}

fn lower_image<'a>(
    scene: &mut RenderScene,
    document: &Document,
    id: EntityId,
    rect: Rect,
    paint: Option<&ImagePaint>,
    resolve: &impl Fn(&ResourceDigest) -> Option<&'a [u8]>,
    image_cache: &mut ImageSurfaceCache,
) -> Result<(), RenderError> {
    let Some(paint) = paint else {
        image_fidelity(scene, id, "image entity has no authored image paint");
        return Ok(());
    };
    if inverse_image_transform(paint.transform).is_none() {
        image_fidelity(
            scene,
            id,
            "image transform is singular or outside the bounded affine profile",
        );
        return Ok(());
    }
    if paint.color_conversion != "srgb" {
        image_fidelity(
            scene,
            id,
            "nuif-image-rgba8-0 supports only encoded-sRGB passthrough",
        );
        return Ok(());
    }
    let Some(asset) = document.assets.get(&paint.asset) else {
        return Err(RenderError::InvalidImage { entity: id });
    };
    let AssetKind::Image(metadata) = &asset.kind else {
        return Err(RenderError::InvalidImage { entity: id });
    };
    if !image_decoder_profile_is_supported(&metadata.decoder_profile) {
        image_fidelity(scene, id, "image decoder profile is not supported");
        return Ok(());
    }
    let Some(resource) = &asset.resource else {
        image_fidelity(scene, id, "image resource is unavailable");
        return Ok(());
    };
    let Some(bytes) = resolve(resource) else {
        image_fidelity(scene, id, "image resource was not explicitly resolved");
        return Ok(());
    };
    let surface = resolve_image_surface(
        scene,
        image_cache,
        &ImageSurfaceRequest {
            entity: id,
            asset: paint.asset,
            resource,
            decoder_profile: &metadata.decoder_profile,
            width: metadata.width,
            height: metadata.height,
            bytes,
        },
    )?;
    scene.commands.push(DrawCommand::Image {
        entity: id,
        rect,
        asset: paint.asset,
        surface,
        fit: paint.fit,
        crop: paint.crop,
        transform: paint.transform,
        sampling: paint.sampling,
        opacity: paint.opacity,
    });
    scene.fidelity.push(RenderFidelity {
        entity: Some(id),
        pointer: format!("/entities/{id}/authored/image"),
        status: Fidelity::Lossless,
    });
    Ok(())
}

fn resolve_image_surface(
    scene: &mut RenderScene,
    cache: &mut ImageSurfaceCache,
    request: &ImageSurfaceRequest<'_>,
) -> Result<ImageSurfaceId, RenderError> {
    if let Some(surface) = cache.ids.get(&request.asset) {
        return ensure_image_dimensions(scene, *surface, request);
    }
    if let Some(surface) = scene.image_surfaces.iter().find_map(|(id, surface)| {
        (&surface.resource == request.resource
            && surface.decoder_profile == request.decoder_profile)
            .then_some(*id)
    }) {
        cache.ids.insert(request.asset, surface);
        return ensure_image_dimensions(scene, surface, request);
    }
    let additional =
        png_profile_decoded_bytes(request.decoder_profile, request.bytes).map_err(|source| {
            RenderError::Image {
                entity: request.entity,
                source,
            }
        })?;
    let observed = cache.decoded_bytes.saturating_add(additional);
    if observed > MAX_SCENE_DECODED_IMAGE_BYTES {
        return Err(RenderError::ImageResourceLimit {
            entity: request.entity,
            limit: MAX_SCENE_DECODED_IMAGE_BYTES,
            observed,
        });
    }
    let image = decode_png_profile(request.decoder_profile, request.bytes).map_err(|source| {
        RenderError::Image {
            entity: request.entity,
            source,
        }
    })?;
    if image.width != request.width || image.height != request.height {
        return Err(RenderError::InvalidImage {
            entity: request.entity,
        });
    }
    let surface = ImageSurfaceId(u32::try_from(scene.image_surfaces.len()).map_err(|_| {
        RenderError::InvalidImage {
            entity: request.entity,
        }
    })?);
    cache.decoded_bytes = observed;
    scene.image_surfaces.insert(
        surface,
        ImageSurface {
            resource: request.resource.clone(),
            decoder_profile: request.decoder_profile.to_owned(),
            image,
        },
    );
    cache.ids.insert(request.asset, surface);
    Ok(surface)
}

fn ensure_image_dimensions(
    scene: &RenderScene,
    surface: ImageSurfaceId,
    request: &ImageSurfaceRequest<'_>,
) -> Result<ImageSurfaceId, RenderError> {
    if scene.image_surfaces.get(&surface).is_some_and(|entry| {
        entry.image.width == request.width && entry.image.height == request.height
    }) {
        Ok(surface)
    } else {
        Err(RenderError::InvalidImage {
            entity: request.entity,
        })
    }
}

fn image_decoder_profile_is_supported(profile: &str) -> bool {
    matches!(profile, PNG_RGBA8_PROFILE | PNG_BASIC_RGBA8_PROFILE)
}

fn image_fidelity(scene: &mut RenderScene, id: EntityId, reason: &str) {
    scene.fidelity.push(RenderFidelity {
        entity: Some(id),
        pointer: format!("/entities/{id}/authored/image"),
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    });
}

fn lower_text<'a>(
    scene: &mut RenderScene,
    document: &Document,
    id: EntityId,
    rect: Rect,
    text: &nuif_core::TextContent,
    context: &EvaluationContext,
    resolve: &impl Fn(&ResourceDigest) -> Option<&'a [u8]>,
) -> Result<(), RenderError> {
    let (font_sha256, font_asset, status) = match select_text_font(document, text, context, id)? {
        TextFontLowering::Render {
            sha256,
            asset,
            fidelity,
        } => (sha256, asset, fidelity),
        TextFontLowering::Skip(fidelity) => {
            text_fidelity(scene, id, fidelity);
            return Ok(());
        }
    };
    let request = ShapeRequest {
        text: &text.content,
        font_sha256,
        font_size: text.size,
        direction: match context.writing_direction {
            WritingDirection::LeftToRight => TextDirection::LeftToRight,
            WritingDirection::RightToLeft => TextDirection::RightToLeft,
        },
        language: &context.locale,
    };
    let face = resolve_text_face(document, id, font_sha256, font_asset, resolve)?;
    if let ResolvedTextFace::Skip(fidelity) = face {
        text_fidelity(scene, id, fidelity);
        return Ok(());
    }
    let runs = face
        .shape_hard_lines(&request)
        .map_err(|source| RenderError::Text { entity: id, source })?;
    text_fidelity(scene, id, status);
    for (line_index, run) in runs.into_iter().enumerate() {
        let mut outlines = BTreeMap::new();
        for glyph in &run.glyphs {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                outlines.entry(glyph.glyph_id)
            {
                let outline = face
                    .outline_glyph(glyph.glyph_id)
                    .map_err(|source| RenderError::Text { entity: id, source })?;
                entry.insert(outline);
            }
        }
        let line_offset = u32::try_from(line_index).map_or(f64::MAX, f64::from) * text.line_height;
        let line_y = rect.y + line_offset;
        let remaining_height = (rect.y + rect.height - line_y).max(0.0);
        scene.commands.push(DrawCommand::Text {
            entity: id,
            rect: Rect {
                x: rect.x,
                y: line_y,
                width: rect.width,
                height: remaining_height.min(text.line_height),
            },
            run: Box::new(run),
            outlines: Box::new(outlines),
            color: Color {
                space: ColorSpace::Srgb,
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },
        });
    }
    Ok(())
}

fn resolve_text_face<'a>(
    document: &Document,
    entity: EntityId,
    sha256: &str,
    asset_id: Option<AssetId>,
    resolve: &impl Fn(&ResourceDigest) -> Option<&'a [u8]>,
) -> Result<ResolvedTextFace<'a>, RenderError> {
    if sha256 == PINNED_FONT_SHA256 {
        return Ok(ResolvedTextFace::Pinned);
    }
    let Some(asset_id) = asset_id else {
        return Err(RenderError::Text {
            entity,
            source: TextError::FontUnavailable {
                expected: PINNED_FONT_SHA256,
                observed: sha256.to_owned(),
            },
        });
    };
    let Some(asset) = document.assets.get(&asset_id) else {
        return Err(RenderError::InvalidFontBinding {
            entity,
            reason: "font asset is absent",
        });
    };
    let AssetKind::Font(metadata) = &asset.kind else {
        return Err(RenderError::InvalidFontBinding {
            entity,
            reason: "font binding references a non-font asset",
        });
    };
    let Some(resource) = &asset.resource else {
        return Ok(ResolvedTextFace::Skip(Fidelity::Unsupported {
            reason: "font resource is unavailable".to_owned(),
        }));
    };
    let Some(bytes) = resolve(resource) else {
        return Ok(ResolvedTextFace::Skip(Fidelity::Unsupported {
            reason: "font resource was not explicitly resolved".to_owned(),
        }));
    };
    let family = metadata
        .names
        .first()
        .ok_or(RenderError::InvalidFontBinding {
            entity,
            reason: "font asset has no reviewed family name",
        })?;
    let license = metadata.policy_evidence.get("license.expression").ok_or(
        RenderError::InvalidFontBinding {
            entity,
            reason: "font asset has no reviewed license expression",
        },
    )?;
    ResourceFont::new(bytes, sha256, family, license)
        .map(ResolvedTextFace::Resource)
        .map_err(|source| RenderError::Text { entity, source })
}

fn select_text_font<'a>(
    document: &'a Document,
    text: &'a nuif_core::TextContent,
    context: &EvaluationContext,
    id: EntityId,
) -> Result<TextFontLowering<'a>, RenderError> {
    Ok(match resolve_text_font_binding(document, text) {
        TextFontBinding::Unbound { requested_sha256 } => {
            if !context.font_hashes.contains(requested_sha256) {
                return Err(RenderError::Text {
                    entity: id,
                    source: TextError::FontAbsentFromContext {
                        hash: requested_sha256.to_owned(),
                    },
                });
            }
            TextFontLowering::Render {
                sha256: requested_sha256,
                asset: None,
                fidelity: Fidelity::Lossless,
            }
        }
        TextFontBinding::Exact { asset, sha256, .. } => {
            if !context.font_hashes.contains(sha256) {
                return Ok(TextFontLowering::Skip(Fidelity::Unsupported {
                    reason: "bound exact font is absent from the render context".to_owned(),
                }));
            }
            TextFontLowering::Render {
                sha256,
                asset: Some(asset),
                fidelity: Fidelity::Lossless,
            }
        }
        TextFontBinding::Substituted {
            asset,
            requested_sha256,
            replacement_sha256,
            ..
        } => {
            if !context.font_hashes.contains(replacement_sha256) {
                return Ok(TextFontLowering::Skip(Fidelity::Unsupported {
                    reason: "declared replacement font is absent from the render context"
                        .to_owned(),
                }));
            }
            TextFontLowering::Render {
                sha256: replacement_sha256,
                asset: Some(asset),
                fidelity: Fidelity::Approximated {
                    reason: format!(
                        "requested font {requested_sha256} was rendered with declared replacement {replacement_sha256}"
                    ),
                },
            }
        }
        TextFontBinding::Unavailable { .. } => TextFontLowering::Skip(Fidelity::Unsupported {
            reason: "font resource is intentionally unavailable".to_owned(),
        }),
        TextFontBinding::Invalid { reason, .. } => {
            return Err(RenderError::InvalidFontBinding { entity: id, reason });
        }
    })
}

fn text_fidelity(scene: &mut RenderScene, id: EntityId, status: Fidelity) {
    scene.fidelity.push(RenderFidelity {
        entity: Some(id),
        pointer: format!("/entities/{id}/authored/text"),
        status,
    });
}

/// Deterministic profile-0 rasterizer for bounded solid paint, pinned unhinted
/// text outlines and explicitly resolved PNG image commands. Unsupported
/// semantics stay in the scene fidelity report rather than disappearing.
///
/// # Errors
///
/// Returns [`RenderError::InvalidTarget`] for zero-sized targets, non-positive
/// scale factors, or targets larger than 16,777,216 pixels, and
/// [`RenderError::InvalidScene`] for non-finite or negative geometry and
/// non-finite colors.
pub fn render_cpu(scene: &RenderScene, target: RenderTarget) -> Result<RasterImage, RenderError> {
    let pixels = u64::from(target.width) * u64::from(target.height);
    if pixels == 0
        || pixels > 16_777_216
        || !target.scale_factor.is_finite()
        || target.scale_factor <= 0.0
    {
        return Err(RenderError::InvalidTarget);
    }
    for command in &scene.commands {
        let (entity, valid) = draw_command_is_valid(scene, command, target.scale_factor);
        if !valid {
            return Err(RenderError::InvalidScene { entity });
        }
    }
    let mut image = RasterImage {
        width: target.width,
        height: target.height,
        rgba: vec![255; usize::try_from(pixels * 4).map_err(|_| RenderError::InvalidTarget)?],
    };
    for command in &scene.commands {
        match command {
            DrawCommand::Rect { rect, fill, .. } => fill_rect(
                &mut image,
                scale_rect(*rect, f64::from(target.scale_factor)),
                *fill,
            ),
            DrawCommand::Ellipse { rect, fill, .. } => fill_ellipse(
                &mut image,
                scale_rect(*rect, f64::from(target.scale_factor)),
                *fill,
            ),
            DrawCommand::Text {
                rect,
                run,
                outlines,
                color,
                ..
            } => draw_shaped_text_outlines(
                &mut image,
                scale_rect(*rect, f64::from(target.scale_factor)),
                run,
                outlines,
                f64::from(target.scale_factor),
                *color,
            ),
            DrawCommand::Image {
                entity,
                rect,
                surface,
                fit,
                crop,
                transform,
                sampling,
                opacity,
                ..
            } => {
                let Some(surface) = scene.image_surfaces.get(surface) else {
                    return Err(RenderError::InvalidScene { entity: *entity });
                };
                draw_image(
                    &mut image,
                    scale_rect(*rect, f64::from(target.scale_factor)),
                    &surface.image,
                    *fit,
                    *crop,
                    *transform,
                    *sampling,
                    *opacity,
                    f64::from(target.scale_factor),
                );
            }
        }
    }
    Ok(image)
}

fn draw_command_is_valid(
    scene: &RenderScene,
    command: &DrawCommand,
    scale_factor: f32,
) -> (EntityId, bool) {
    match command {
        DrawCommand::Rect { entity, rect, fill } | DrawCommand::Ellipse { entity, rect, fill } => (
            *entity,
            command_rect_is_valid(*rect, scale_factor) && color_is_finite(*fill),
        ),
        DrawCommand::Text {
            entity,
            rect,
            color,
            run,
            outlines,
        } => (
            *entity,
            command_rect_is_valid(*rect, scale_factor)
                && color_is_finite(*color)
                && shaped_run_is_valid(run, outlines, f64::from(scale_factor)),
        ),
        DrawCommand::Image {
            entity,
            rect,
            surface,
            crop,
            transform,
            opacity,
            ..
        } => {
            let surface = scene.image_surfaces.get(surface);
            (
                *entity,
                command_rect_is_valid(*rect, scale_factor)
                    && surface.is_some_and(|surface| {
                        image_decoder_profile_is_supported(&surface.decoder_profile)
                            && image_command_is_valid(&surface.image, *crop, *transform, *opacity)
                    }),
            )
        }
    }
}

fn command_rect_is_valid(rect: Rect, scale_factor: f32) -> bool {
    rect_is_valid(rect) && rect_is_valid(scale_rect(rect, f64::from(scale_factor)))
}

fn image_command_is_valid(
    image: &Rgba8Image,
    crop: ImageCrop,
    transform: AffineTransform,
    opacity: f32,
) -> bool {
    let expected = usize::try_from(u64::from(image.width) * u64::from(image.height) * 4).ok();
    image.width > 0
        && image.height > 0
        && expected == Some(image.rgba.len())
        && [crop.x, crop.y, crop.width, crop.height]
            .into_iter()
            .all(f64::is_finite)
        && crop.x >= 0.0
        && crop.y >= 0.0
        && crop.width > 0.0
        && crop.height > 0.0
        && crop.x + crop.width <= 1.0
        && crop.y + crop.height <= 1.0
        && inverse_image_transform(transform).is_some()
        && opacity.is_finite()
        && (0.0..=1.0).contains(&opacity)
}

fn scale_rect(rect: Rect, scale: f64) -> Rect {
    Rect {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

fn rect_is_valid(rect: Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f64::is_finite)
        && rect.width >= 0.0
        && rect.height >= 0.0
}

fn color_is_finite(color: Color) -> bool {
    [color.red, color.green, color.blue, color.alpha]
        .into_iter()
        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
}

fn shaped_run_is_valid(
    run: &ShapedRun,
    outlines: &BTreeMap<u32, GlyphOutline>,
    target_scale: f64,
) -> bool {
    let codepoints = run.text.chars().count();
    run.font_size.is_finite()
        && run.font_size > 0.0
        && run.font_size * target_scale <= f64::from(u32::MAX)
        && run.units_per_em > 0
        && run.cluster_unit == CLUSTER_UNIT
        && codepoints <= MAX_SHAPING_CODEPOINTS
        && outlines.len() <= run.glyphs.len()
        && run.glyphs.iter().all(|glyph| {
            usize::try_from(glyph.cluster).is_ok_and(|cluster| cluster < codepoints)
                && outlines
                    .get(&glyph.glyph_id)
                    .is_some_and(|outline| outline_is_valid(glyph.glyph_id, outline))
        })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the image command fields stay explicit at the deterministic raster boundary"
)]
fn draw_image(
    destination: &mut RasterImage,
    destination_rect: Rect,
    source: &Rgba8Image,
    fit: ImageFit,
    crop: ImageCrop,
    transform: AffineTransform,
    sampling: ImageSampling,
    opacity: f32,
    target_scale: f64,
) {
    if destination_rect.width == 0.0 || destination_rect.height == 0.0 || opacity == 0.0 {
        return;
    }
    let source_width = f64::from(source.width) * crop.width;
    let source_height = f64::from(source.height) * crop.height;
    let (draw_width, draw_height) = match fit {
        ImageFit::Fill => (destination_rect.width, destination_rect.height),
        ImageFit::Contain => {
            let scale = (destination_rect.width / source_width)
                .min(destination_rect.height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::Cover => {
            let scale = (destination_rect.width / source_width)
                .max(destination_rect.height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::None => (source_width * target_scale, source_height * target_scale),
    };
    let draw = Rect {
        x: destination_rect.x + (destination_rect.width - draw_width) * 0.5,
        y: destination_rect.y + (destination_rect.height - draw_height) * 0.5,
        width: draw_width,
        height: draw_height,
    };
    let Some(inverse) = inverse_image_transform(transform) else {
        return;
    };
    let transformed_bounds = transformed_image_bounds(draw, transform);
    let x0 = floor_coordinate(
        transformed_bounds.x.max(destination_rect.x),
        destination.width,
    );
    let y0 = floor_coordinate(
        transformed_bounds.y.max(destination_rect.y),
        destination.height,
    );
    let x1 = ceil_coordinate(
        (transformed_bounds.x + transformed_bounds.width)
            .min(destination_rect.x + destination_rect.width),
        destination.width,
    );
    let y1 = ceil_coordinate(
        (transformed_bounds.y + transformed_bounds.height)
            .min(destination_rect.y + destination_rect.height),
        destination.height,
    );
    let opacity = channel(opacity);
    for y in y0..y1 {
        for x in x0..x1 {
            let paint_u = (f64::from(x) + 0.5 - draw.x) / draw.width;
            let paint_v = (f64::from(y) + 0.5 - draw.y) / draw.height;
            let (u, v) = transform_point(inverse, paint_u, paint_v);
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let mut pixel = match sampling {
                ImageSampling::Nearest => sample_image_nearest(source, crop, u, v),
                ImageSampling::Linear => sample_image_linear(source, crop, u, v),
            };
            pixel[3] = u8::try_from((u16::from(pixel[3]) * u16::from(opacity) + 127) / 255)
                .expect("product of two u8 alpha values remains in the u8 domain");
            blend_pixel(destination, x, y, pixel);
        }
    }
}

const MAX_IMAGE_TRANSFORM_COMPONENT: f64 = 1_000_000.0;
const MIN_IMAGE_TRANSFORM_DETERMINANT: f64 = 1.0e-12;

fn inverse_image_transform(transform: AffineTransform) -> Option<AffineTransform> {
    let components = [
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.tx,
        transform.ty,
    ];
    if components
        .iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_IMAGE_TRANSFORM_COMPONENT)
    {
        return None;
    }
    let determinant = transform.a * transform.d - transform.b * transform.c;
    if !determinant.is_finite() || determinant.abs() < MIN_IMAGE_TRANSFORM_DETERMINANT {
        return None;
    }
    let inverse = AffineTransform {
        a: transform.d / determinant,
        b: -transform.b / determinant,
        c: -transform.c / determinant,
        d: transform.a / determinant,
        tx: (transform.c * transform.ty - transform.d * transform.tx) / determinant,
        ty: (transform.b * transform.tx - transform.a * transform.ty) / determinant,
    };
    [
        inverse.a, inverse.b, inverse.c, inverse.d, inverse.tx, inverse.ty,
    ]
    .iter()
    .all(|value| value.is_finite() && value.abs() <= MAX_IMAGE_TRANSFORM_COMPONENT)
    .then_some(inverse)
}

fn transform_point(transform: AffineTransform, x: f64, y: f64) -> (f64, f64) {
    (
        transform.a * x + transform.c * y + transform.tx,
        transform.b * x + transform.d * y + transform.ty,
    )
}

fn transformed_image_bounds(draw: Rect, transform: AffineTransform) -> Rect {
    let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in corners {
        let (x, y) = transform_point(transform, x, y);
        let x = draw.x + x * draw.width;
        let y = draw.y + y * draw.height;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Rect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn sample_image_nearest(source: &Rgba8Image, crop: ImageCrop, u: f64, v: f64) -> [u8; 4] {
    let x = (crop.x + u * crop.width) * f64::from(source.width);
    let y = (crop.y + v * crop.height) * f64::from(source.height);
    image_pixel(
        source,
        bounded_sample_index(x.floor(), source.width),
        bounded_sample_index(y.floor(), source.height),
    )
}

fn sample_image_linear(source: &Rgba8Image, crop: ImageCrop, u: f64, v: f64) -> [u8; 4] {
    let x = (crop.x + u * crop.width) * f64::from(source.width) - 0.5;
    let y = (crop.y + v * crop.height) * f64::from(source.height) - 0.5;
    let min_x = bounded_sample_index((crop.x * f64::from(source.width)).floor(), source.width);
    let min_y = bounded_sample_index((crop.y * f64::from(source.height)).floor(), source.height);
    let max_x = bounded_sample_index(
        ((crop.x + crop.width) * f64::from(source.width)).ceil() - 1.0,
        source.width,
    );
    let max_y = bounded_sample_index(
        ((crop.y + crop.height) * f64::from(source.height)).ceil() - 1.0,
        source.height,
    );
    let x_floor = x.floor();
    let y_floor = y.floor();
    let x0 = bounded_sample_index(x_floor, source.width).clamp(min_x, max_x);
    let y0 = bounded_sample_index(y_floor, source.height).clamp(min_y, max_y);
    let x1 = bounded_sample_index(x_floor + 1.0, source.width).clamp(min_x, max_x);
    let y1 = bounded_sample_index(y_floor + 1.0, source.height).clamp(min_y, max_y);
    let wx = sample_fraction(x - x_floor);
    let wy = sample_fraction(y - y_floor);
    let samples = [
        image_pixel(source, x0, y0),
        image_pixel(source, x1, y0),
        image_pixel(source, x0, y1),
        image_pixel(source, x1, y1),
    ];
    let weights = [
        (65_536 - wx) * (65_536 - wy),
        wx * (65_536 - wy),
        (65_536 - wx) * wy,
        wx * wy,
    ];
    std::array::from_fn(|channel| {
        let total = samples
            .iter()
            .zip(weights)
            .map(|(pixel, weight)| u64::from(pixel[channel]) * weight)
            .sum::<u64>();
        u8::try_from((total + (1_u64 << 31)) >> 32)
            .expect("normalized bilinear weights retain the u8 channel domain")
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the finite sample coordinate is clamped to the source image before conversion"
)]
fn bounded_sample_index(value: f64, limit: u32) -> u32 {
    value.clamp(0.0, f64::from(limit.saturating_sub(1))) as u32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the finite fraction is clamped to the fixed 16-bit interpolation domain"
)]
fn sample_fraction(value: f64) -> u64 {
    (value.clamp(0.0, 1.0) * 65_536.0).round() as u64
}

fn image_pixel(image: &Rgba8Image, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * image.width as usize + x as usize) * 4;
    image.rgba[index..index + 4]
        .try_into()
        .expect("validated RGBA image has four channels per pixel")
}

fn outline_is_valid(glyph_id: u32, outline: &GlyphOutline) -> bool {
    outline.glyph_id == glyph_id
        && outline.extractor == OUTLINE_EXTRACTOR_NAME
        && outline.extractor_version == OUTLINE_EXTRACTOR_VERSION
        && outline.coordinate_denominator == OUTLINE_COORDINATE_DENOMINATOR
        && outline.commands.len() <= 4096
}

fn fill_rect(image: &mut RasterImage, rect: Rect, color: Color) {
    let x0 = floor_coordinate(rect.x, image.width);
    let y0 = floor_coordinate(rect.y, image.height);
    let x1 = ceil_coordinate(rect.x + rect.width, image.width);
    let y1 = ceil_coordinate(rect.y + rect.height, image.height);
    let source = [
        channel(color.red),
        channel(color.green),
        channel(color.blue),
        channel(color.alpha),
    ];
    for y in y0..y1 {
        for x in x0..x1 {
            blend_pixel(image, x, y, source);
        }
    }
}

fn fill_ellipse(image: &mut RasterImage, rect: Rect, color: Color) {
    let base_x = floor_coordinate(rect.x, image.width);
    let base_y = floor_coordinate(rect.y, image.height);
    let x1 = ceil_coordinate(rect.x + rect.width, image.width);
    let y1 = ceil_coordinate(rect.y + rect.height, image.height);
    let mask_width = x1.saturating_sub(base_x);
    let mask_height = y1.saturating_sub(base_y);
    if mask_width == 0 || mask_height == 0 {
        return;
    }
    let mut commands = Vec::new();
    commands.add_ellipse(
        ZenoPoint::new(
            ellipse_coordinate(rect.x + rect.width * 0.5 - f64::from(base_x)),
            ellipse_coordinate(rect.y + rect.height * 0.5 - f64::from(base_y)),
        ),
        ellipse_coordinate(rect.width * 0.5),
        ellipse_coordinate(rect.height * 0.5),
    );
    let mask_len = usize::try_from(u64::from(mask_width) * u64::from(mask_height))
        .expect("the validated raster target fits usize");
    let mut coverage = vec![0_u8; mask_len];
    Mask::new(&commands)
        .style(Fill::NonZero)
        .format(Format::Alpha)
        .size(mask_width, mask_height)
        .render_into(&mut coverage, None);
    blend_mask(
        image,
        base_x,
        base_y,
        mask_width,
        mask_height,
        &coverage,
        color,
    );
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated ellipse geometry is finite and bounded by the f32 raster target"
)]
fn ellipse_coordinate(value: f64) -> f32 {
    value as f32
}

fn draw_shaped_text_outlines(
    image: &mut RasterImage,
    rect: Rect,
    run: &ShapedRun,
    outlines: &BTreeMap<u32, GlyphOutline>,
    target_scale: f64,
    color: Color,
) {
    let base_x = floor_coordinate(rect.x, image.width);
    let base_y = floor_coordinate(rect.y, image.height);
    let x1 = ceil_coordinate(rect.x + rect.width, image.width);
    let y1 = ceil_coordinate(rect.y + rect.height, image.height);
    let mask_width = x1.saturating_sub(base_x);
    let mask_height = y1.saturating_sub(base_y);
    if mask_width == 0 || mask_height == 0 {
        return;
    }
    let font_unit_scale = run.font_size * target_scale / f64::from(run.units_per_em);
    let baseline =
        rect.y - f64::from(base_y) + f64::from(run.ascender_font_units) * font_unit_scale;
    let run_width = run
        .glyphs
        .iter()
        .map(|glyph| f64::from(glyph.x_advance))
        .sum::<f64>()
        * font_unit_scale;
    let mut pen_x = match run.direction {
        TextDirection::LeftToRight => rect.x,
        TextDirection::RightToLeft => rect.x + rect.width - run_width,
    };
    let mut commands = Vec::new();
    for glyph in &run.glyphs {
        let Some(outline) = outlines.get(&glyph.glyph_id) else {
            continue;
        };
        let origin_x = pen_x - f64::from(base_x) + f64::from(glyph.x_offset) * font_unit_scale;
        let origin_y = baseline - f64::from(glyph.y_offset) * font_unit_scale;
        for command in &outline.commands {
            commands.push(to_zeno_command(
                *command,
                origin_x,
                origin_y,
                font_unit_scale,
            ));
        }
        pen_x += f64::from(glyph.x_advance) * font_unit_scale;
    }
    if commands.is_empty() {
        return;
    }
    let mask_len = usize::try_from(u64::from(mask_width) * u64::from(mask_height))
        .expect("the validated raster target fits usize");
    let mut coverage = vec![0_u8; mask_len];
    Mask::new(&commands)
        .style(Fill::NonZero)
        .format(Format::Alpha)
        .size(mask_width, mask_height)
        .render_into(&mut coverage, None);
    blend_mask(
        image,
        base_x,
        base_y,
        mask_width,
        mask_height,
        &coverage,
        color,
    );
}

fn blend_mask(
    image: &mut RasterImage,
    base_x: u32,
    base_y: u32,
    mask_width: u32,
    mask_height: u32,
    coverage: &[u8],
    color: Color,
) {
    let source = [
        channel(color.red),
        channel(color.green),
        channel(color.blue),
        channel(color.alpha),
    ];
    for y in 0..mask_height {
        for x in 0..mask_width {
            let mask_index = y as usize * mask_width as usize + x as usize;
            let alpha = coverage[mask_index];
            if alpha == 0 {
                continue;
            }
            let mut covered = source;
            covered[3] = u8::try_from((u16::from(source[3]) * u16::from(alpha) + 127) / 255)
                .expect("the product of two alpha channels remains u8 after division");
            blend_pixel(image, base_x + x, base_y + y, covered);
        }
    }
}

fn to_zeno_command(
    command: OutlineCommand,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
) -> ZenoCommand {
    match command {
        OutlineCommand::MoveTo { to } => {
            ZenoCommand::MoveTo(to_zeno_point(to, origin_x, origin_y, scale))
        }
        OutlineCommand::LineTo { to } => {
            ZenoCommand::LineTo(to_zeno_point(to, origin_x, origin_y, scale))
        }
        OutlineCommand::QuadTo { control, to } => ZenoCommand::QuadTo(
            to_zeno_point(control, origin_x, origin_y, scale),
            to_zeno_point(to, origin_x, origin_y, scale),
        ),
        OutlineCommand::CurveTo {
            control_0,
            control_1,
            to,
        } => ZenoCommand::CurveTo(
            to_zeno_point(control_0, origin_x, origin_y, scale),
            to_zeno_point(control_1, origin_x, origin_y, scale),
            to_zeno_point(to, origin_x, origin_y, scale),
        ),
        OutlineCommand::Close => ZenoCommand::Close,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated target and font geometry are finite and bounded before this f32 raster boundary"
)]
fn to_zeno_point(point: OutlinePoint, origin_x: f64, origin_y: f64, scale: f64) -> ZenoPoint {
    let denominator = f64::from(OUTLINE_COORDINATE_DENOMINATOR);
    ZenoPoint::new(
        (origin_x + f64::from(point.x) / denominator * scale) as f32,
        (origin_y - f64::from(point.y) / denominator * scale) as f32,
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is finite and clamped to the exact u8 domain before conversion"
)]
fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is finite and clamped to the target dimension before conversion"
)]
fn floor_coordinate(value: f64, limit: u32) -> u32 {
    value.floor().clamp(0.0, f64::from(limit)) as u32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is finite and clamped to the target dimension before conversion"
)]
fn ceil_coordinate(value: f64, limit: u32) -> u32 {
    value.ceil().clamp(0.0, f64::from(limit)) as u32
}

fn blend_pixel(image: &mut RasterImage, x: u32, y: u32, source: [u8; 4]) {
    let index = (y as usize * image.width as usize + x as usize) * 4;
    let alpha = u32::from(source[3]);
    let inverse = 255 - alpha;
    for (offset, source_channel) in source[..3].iter().enumerate() {
        let destination = u32::from(image.rgba[index + offset]);
        image.rgba[index + offset] =
            u8::try_from((u32::from(*source_channel) * alpha + destination * inverse + 127) / 255)
                .expect("alpha blend of two u8 channels remains in the u8 domain");
    }
    image.rgba[index + 3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nuif_core::{
        AffineTransform, Asset, AssetPortability, CURRENT_SCHEMA_VERSION, Entity, FontAsset,
        ImageAsset, Point, SizeIntent, TextContent,
    };
    use nuif_text::{PINNED_FONT_NAME, PINNED_FONT_SHA256};

    fn text_document_and_layout() -> (Document, LayoutSnapshot) {
        let mut document = Document::empty(EntityId::new(1));
        let mut text = Entity::new(EntityId::new(2), EntityKind::Text);
        text.authored.width = SizeIntent::Fixed(100.0);
        text.authored.height = SizeIntent::Fixed(20.0);
        text.authored.text = Some(TextContent {
            content: "A B".to_owned(),
            font: PINNED_FONT_NAME.to_owned(),
            font_sha256: PINNED_FONT_SHA256.to_owned(),
            font_asset: None,
            size: 18.0,
            line_height: 20.0,
        });
        document.roots.push(text.id);
        document.entities.insert(text.id, text);
        let layout = nuif_layout::evaluate(&document, &EvaluationContext::viewport(100.0, 20.0));
        (document, layout)
    }

    fn image_bytes() -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut output), 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_filter(png::Filter::NoFilter);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                ])
                .unwrap();
        }
        output
    }

    fn bind_text_font(document: &mut Document, portability: AssetPortability) {
        let asset_id = AssetId::new(0xf0);
        let resource = (portability != AssetPortability::Unavailable)
            .then(|| ResourceDigest::from_sha256_hex(PINNED_FONT_SHA256));
        document.assets.insert(
            asset_id,
            Asset {
                schema_version: CURRENT_SCHEMA_VERSION,
                id: asset_id,
                name: Some(PINNED_FONT_NAME.to_owned()),
                resource,
                portability,
                kind: AssetKind::Font(FontAsset {
                    face_index: 0,
                    names: vec![PINNED_FONT_NAME.to_owned()],
                    axes: BTreeMap::new(),
                    features: BTreeMap::new(),
                    coverage: Vec::new(),
                    policy_evidence: BTreeMap::new(),
                }),
            },
        );
        let text = document
            .entities
            .get_mut(&EntityId::new(2))
            .and_then(|entity| entity.authored.text.as_mut())
            .expect("text fixture");
        text.font_sha256 = "1".repeat(64);
        text.font_asset = Some(asset_id);
    }

    fn image_document() -> (Document, ResourceDigest, Vec<u8>) {
        let bytes = image_bytes();
        let resource = ResourceDigest::from_sha256_hex("a".repeat(64));
        let asset_id = AssetId::new(0xa0);
        let mut document = Document::empty(EntityId::new(1));
        document.assets.insert(
            asset_id,
            Asset {
                schema_version: CURRENT_SCHEMA_VERSION,
                id: asset_id,
                name: Some("pixels".to_owned()),
                resource: Some(resource.clone()),
                portability: AssetPortability::Portable,
                kind: AssetKind::Image(ImageAsset {
                    width: 2,
                    height: 2,
                    decoder_profile: PNG_RGBA8_PROFILE.to_owned(),
                }),
            },
        );
        let mut entity = Entity::new(EntityId::new(2), EntityKind::Image);
        entity.authored.width = SizeIntent::Fixed(4.0);
        entity.authored.height = SizeIntent::Fixed(4.0);
        entity.authored.position = Point { x: 0.0, y: 0.0 };
        entity.authored.image = Some(ImagePaint {
            asset: asset_id,
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
            sampling: ImageSampling::Nearest,
            opacity: 1.0,
            color_conversion: "srgb".to_owned(),
        });
        document.roots.push(entity.id);
        document.entities.insert(entity.id, entity);
        (document, resource, bytes)
    }

    #[test]
    fn scene_contains_resolved_glyphs_and_requires_context_font() {
        let (document, layout) = text_document_and_layout();
        let missing = EvaluationContext::viewport(100.0, 20.0);
        assert!(matches!(
            build_scene(&document, &layout, &missing),
            Err(RenderError::Text {
                source: TextError::FontAbsentFromContext { .. },
                ..
            })
        ));

        let mut available = missing;
        available.font_hashes.insert(PINNED_FONT_SHA256.to_owned());
        let scene = build_scene(&document, &layout, &available).unwrap();
        let DrawCommand::Text { run, .. } = &scene.commands[0] else {
            panic!("text entity must lower to a text command");
        };
        assert_eq!(run.serialized_glyphs, "[35=0+1000|3=1+1000|36=2+1000]");
        assert_eq!(
            scene.fidelity,
            vec![RenderFidelity {
                entity: Some(EntityId::new(2)),
                pointer: "/entities/00000000000000000000000000000002/authored/text".to_owned(),
                status: Fidelity::Lossless,
            }]
        );

        let image = render_cpu(
            &scene,
            RenderTarget {
                width: 100,
                height: 20,
                scale_factor: 1.0,
            },
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * 100 + x) * 4..][..4];
        assert_eq!(pixel(5, 5), [0, 0, 0, 255]);
        assert_eq!(pixel(25, 5), [255, 255, 255, 255]);
        assert_eq!(pixel(40, 5), [0, 0, 0, 255]);
    }

    #[test]
    fn substituted_and_unavailable_fonts_have_item_level_fidelity() {
        let (mut substituted, _) = text_document_and_layout();
        bind_text_font(&mut substituted, AssetPortability::Substituted);
        let mut available = EvaluationContext::viewport(100.0, 20.0);
        available.font_hashes.insert(PINNED_FONT_SHA256.to_owned());
        let layout = nuif_layout::evaluate(&substituted, &available);
        assert!(layout.diagnostics.iter().any(|diagnostic| {
            diagnostic.entity == Some(EntityId::new(2))
                && diagnostic.code == "TEXT_FONT_SUBSTITUTED"
                && matches!(diagnostic.fidelity, Some(Fidelity::Approximated { .. }))
        }));
        let scene = build_scene(&substituted, &layout, &available).unwrap();
        assert_eq!(scene.commands.len(), 1);
        assert!(matches!(
            scene.fidelity.as_slice(),
            [RenderFidelity {
                entity: Some(entity),
                status: Fidelity::Approximated { .. },
                ..
            }] if *entity == EntityId::new(2)
        ));

        let missing = EvaluationContext::viewport(100.0, 20.0);
        let layout = nuif_layout::evaluate(&substituted, &missing);
        let scene = build_scene(&substituted, &layout, &missing).unwrap();
        assert!(scene.commands.is_empty());
        assert!(matches!(
            scene.fidelity[0].status,
            Fidelity::Unsupported { .. }
        ));

        let (mut unavailable, _) = text_document_and_layout();
        bind_text_font(&mut unavailable, AssetPortability::Unavailable);
        let layout = nuif_layout::evaluate(&unavailable, &missing);
        assert!(layout.diagnostics.iter().any(|diagnostic| {
            diagnostic.entity == Some(EntityId::new(2))
                && diagnostic.code == "TEXT_FONT_UNAVAILABLE"
                && matches!(diagnostic.fidelity, Some(Fidelity::Unsupported { .. }))
        }));
        let scene = build_scene(&unavailable, &layout, &missing).unwrap();
        assert!(scene.commands.is_empty());
        assert!(matches!(
            scene.fidelity[0].status,
            Fidelity::Unsupported { .. }
        ));
    }

    #[test]
    fn hard_lines_and_rtl_inline_start_are_rasterized_without_soft_wrap() {
        let (mut document, _) = text_document_and_layout();
        let text = document
            .entities
            .get_mut(&EntityId::new(2))
            .expect("text fixture exists");
        text.authored.height = SizeIntent::Fixed(40.0);
        text.authored
            .text
            .as_mut()
            .expect("text fixture has content")
            .content = "A\nB".to_owned();
        let mut context = EvaluationContext::viewport(100.0, 40.0);
        context.font_hashes.insert(PINNED_FONT_SHA256.to_owned());
        let layout = nuif_layout::evaluate(&document, &context);
        let scene = build_scene(&document, &layout, &context).unwrap();
        let text_commands = scene
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { rect, run, .. } => Some((*rect, run.text.as_str())),
                DrawCommand::Rect { .. }
                | DrawCommand::Ellipse { .. }
                | DrawCommand::Image { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text_commands.len(), 2);
        assert!(text_commands[0].0.y.abs() < f64::EPSILON);
        assert!((text_commands[1].0.y - 20.0).abs() < f64::EPSILON);

        let mut rtl_document = document;
        rtl_document
            .entities
            .get_mut(&EntityId::new(2))
            .and_then(|entity| entity.authored.text.as_mut())
            .expect("text fixture has content")
            .content = "A".to_owned();
        context.writing_direction = WritingDirection::RightToLeft;
        let rtl_layout = nuif_layout::evaluate(&rtl_document, &context);
        let rtl_scene = build_scene(&rtl_document, &rtl_layout, &context).unwrap();
        let image = render_cpu(
            &rtl_scene,
            RenderTarget {
                width: 100,
                height: 40,
                scale_factor: 1.0,
            },
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * 100 + x) * 4..][..4];
        assert_eq!(pixel(5, 5), [255, 255, 255, 255]);
        assert_eq!(pixel(90, 5), [0, 0, 0, 255]);
    }

    #[test]
    fn cpu_raster_is_repeatable() {
        let scene = RenderScene {
            commands: vec![DrawCommand::Rect {
                entity: EntityId::new(1),
                rect: Rect {
                    x: 1.0,
                    y: 1.0,
                    width: 2.0,
                    height: 2.0,
                },
                fill: Color {
                    space: ColorSpace::Srgb,
                    red: 1.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
            image_surfaces: BTreeMap::new(),
        };
        let target = RenderTarget {
            width: 4,
            height: 4,
            scale_factor: 1.0,
        };
        let first = render_cpu(&scene, target).unwrap();
        let second = render_cpu(&scene, target).unwrap();
        assert_eq!(first, second);
        assert_eq!(&first.rgba[20..24], &[255, 0, 0, 255]);
        assert_eq!(first.to_png().unwrap(), second.to_png().unwrap());
    }

    #[test]
    fn package_image_lowers_only_through_explicit_resource_resolution() {
        let (mut document, resource, bytes) = image_document();
        let mut second = document.entities[&EntityId::new(2)].clone();
        second.id = EntityId::new(3);
        document.roots.push(second.id);
        document.entities.insert(second.id, second);
        let context = EvaluationContext::viewport(4.0, 4.0);
        let layout = nuif_layout::evaluate(&document, &context);
        let unresolved = build_scene(&document, &layout, &context).unwrap();
        assert!(unresolved.commands.is_empty());
        assert!(matches!(
            unresolved.fidelity[0].status,
            Fidelity::Unsupported { .. }
        ));

        let scene = build_scene_with_resources(&document, &layout, &context, |digest| {
            (digest == &resource).then_some(bytes.as_slice())
        })
        .unwrap();
        assert!(matches!(scene.commands[0], DrawCommand::Image { .. }));
        assert_eq!(scene.commands.len(), 2);
        assert_eq!(scene.image_surfaces.len(), 1);
        let encoded_scene = serde_json::to_vec(&scene).unwrap();
        assert_eq!(
            serde_json::from_slice::<RenderScene>(&encoded_scene).unwrap(),
            scene
        );
        assert_eq!(scene.fidelity[0].status, Fidelity::Lossless);
        let raster = render_cpu(
            &scene,
            RenderTarget {
                width: 4,
                height: 4,
                scale_factor: 1.0,
            },
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &raster.rgba[(y * 4 + x) * 4..][..4];
        assert_eq!(pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(3, 0), [0, 255, 0, 255]);
        assert_eq!(pixel(0, 3), [0, 0, 255, 255]);
        assert_eq!(pixel(3, 3), [255, 255, 255, 255]);

        let AssetKind::Image(metadata) =
            &mut document.assets.get_mut(&AssetId::new(0xa0)).unwrap().kind
        else {
            panic!("fixture asset must be an image");
        };
        metadata.width = 3;
        assert!(matches!(
            build_scene_with_resources(&document, &layout, &context, |_| Some(bytes.as_slice())),
            Err(RenderError::InvalidImage { .. })
        ));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one raster regression keeps the complete crop, fit, sampling and opacity matrix auditable"
    )]
    fn image_sampling_crop_fit_and_opacity_are_fixed() {
        let source = Rgba8Image {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        let surface_id = ImageSurfaceId(0);
        let surface = ImageSurface {
            resource: ResourceDigest::from_sha256_hex("b".repeat(64)),
            decoder_profile: PNG_RGBA8_PROFILE.to_owned(),
            image: source.clone(),
        };
        let command = |rect, fit, crop, sampling, opacity| DrawCommand::Image {
            entity: EntityId::new(2),
            rect,
            asset: AssetId::new(1),
            surface: surface_id,
            fit,
            crop,
            transform: AffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            sampling,
            opacity,
        };
        let render = |command, width, height| {
            render_cpu(
                &RenderScene {
                    commands: vec![command],
                    fidelity: Vec::new(),
                    image_surfaces: BTreeMap::from([(surface_id, surface.clone())]),
                },
                RenderTarget {
                    width,
                    height,
                    scale_factor: 1.0,
                },
            )
            .unwrap()
        };
        let full = ImageCrop {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };
        let linear = render(
            command(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                },
                ImageFit::Fill,
                full,
                ImageSampling::Linear,
                1.0,
            ),
            1,
            1,
        );
        assert_eq!(linear.rgba, [128, 128, 128, 255]);

        let cropped = render(
            command(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 2.0,
                },
                ImageFit::Fill,
                ImageCrop {
                    x: 0.5,
                    y: 0.0,
                    width: 0.5,
                    height: 1.0,
                },
                ImageSampling::Nearest,
                1.0,
            ),
            1,
            2,
        );
        assert_eq!(cropped.rgba, [0, 255, 0, 255, 255, 255, 255, 255]);

        let contained = render(
            command(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                    height: 6.0,
                },
                ImageFit::Contain,
                full,
                ImageSampling::Nearest,
                0.5,
            ),
            4,
            6,
        );
        let pixel = |x: usize, y: usize| &contained.rgba[(y * 4 + x) * 4..][..4];
        assert_eq!(pixel(0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel(0, 1), [255, 127, 127, 255]);
        assert_eq!(pixel(3, 4), [255, 255, 255, 255]);
        assert_eq!(pixel(0, 5), [255, 255, 255, 255]);

        let transformed = |transform| {
            let mut value = command(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 2.0,
                    height: 2.0,
                },
                ImageFit::Fill,
                full,
                ImageSampling::Nearest,
                1.0,
            );
            let DrawCommand::Image {
                transform: authored,
                ..
            } = &mut value
            else {
                unreachable!("fixture command is an image")
            };
            *authored = transform;
            value
        };
        let horizontal_flip = render(
            transformed(AffineTransform {
                a: -1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 1.0,
                ty: 0.0,
            }),
            2,
            2,
        );
        assert_eq!(
            horizontal_flip.rgba,
            [
                0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255, 0, 0, 255, 255
            ]
        );
        let clockwise = render(
            transformed(AffineTransform {
                a: 0.0,
                b: 1.0,
                c: -1.0,
                d: 0.0,
                tx: 1.0,
                ty: 0.0,
            }),
            2,
            2,
        );
        assert_eq!(
            clockwise.rgba,
            [
                0, 0, 255, 255, 255, 0, 0, 255, 255, 255, 255, 255, 0, 255, 0, 255
            ]
        );
        let translated = render(
            transformed(AffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.5,
                ty: 0.0,
            }),
            2,
            2,
        );
        assert_eq!(
            translated.rgba,
            [
                255, 255, 255, 255, 255, 0, 0, 255, 255, 255, 255, 255, 0, 0, 255, 255
            ]
        );
        assert!(matches!(
            render_cpu(
                &RenderScene {
                    commands: vec![transformed(AffineTransform {
                        a: 1.0,
                        b: 2.0,
                        c: 2.0,
                        d: 4.0,
                        tx: 0.0,
                        ty: 0.0,
                    })],
                    fidelity: Vec::new(),
                    image_surfaces: BTreeMap::from([(surface_id, surface.clone())]),
                },
                RenderTarget {
                    width: 2,
                    height: 2,
                    scale_factor: 1.0,
                }
            ),
            Err(RenderError::InvalidScene { .. })
        ));
    }

    #[test]
    fn bounded_affine_inverse_roundtrips_and_rejects_numeric_edges() {
        for transform in [
            AffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            AffineTransform {
                a: 1.5,
                b: -0.25,
                c: 0.5,
                d: 0.75,
                tx: -0.125,
                ty: 0.625,
            },
            AffineTransform {
                a: 0.0,
                b: 1.0,
                c: -1.0,
                d: 0.0,
                tx: 1.0,
                ty: 0.0,
            },
        ] {
            let inverse = inverse_image_transform(transform).unwrap();
            for (x, y) in [(0.0, 0.0), (0.25, 0.75), (1.0, 1.0)] {
                let (mapped_x, mapped_y) = transform_point(transform, x, y);
                let (actual_x, actual_y) = transform_point(inverse, mapped_x, mapped_y);
                assert!((actual_x - x).abs() < 1.0e-12);
                assert!((actual_y - y).abs() < 1.0e-12);
            }
        }
        assert!(
            inverse_image_transform(AffineTransform {
                a: MAX_IMAGE_TRANSFORM_COMPONENT + 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
            })
            .is_none()
        );
        assert!(
            inverse_image_transform(AffineTransform {
                a: 1.0e-13,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                tx: 0.0,
                ty: 0.0,
            })
            .is_none()
        );
    }

    #[test]
    fn ellipse_masks_and_unsupported_kinds_are_explicit() {
        let ellipse = RenderScene {
            commands: vec![DrawCommand::Ellipse {
                entity: EntityId::new(1),
                rect: Rect {
                    x: 1.0,
                    y: 1.0,
                    width: 4.0,
                    height: 4.0,
                },
                fill: Color {
                    space: ColorSpace::Srgb,
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
            image_surfaces: BTreeMap::new(),
        };
        let image = render_cpu(
            &ellipse,
            RenderTarget {
                width: 6,
                height: 6,
                scale_factor: 1.0,
            },
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * 6 + x) * 4..][..4];
        assert_eq!(pixel(3, 3), [0, 0, 0, 255]);
        assert_eq!(pixel(0, 0), [255, 255, 255, 255]);

        let mut document = Document::empty(EntityId::new(10));
        for entity in [
            Entity::new(EntityId::new(2), EntityKind::Shape(ShapeKind::Path)),
            Entity::new(EntityId::new(3), EntityKind::Image),
            Entity::new(
                EntityId::new(4),
                EntityKind::Instance {
                    component: EntityId::new(9),
                },
            ),
        ] {
            document.entities.insert(entity.id, entity);
        }
        let layout = LayoutSnapshot {
            context_fingerprint: "test".to_owned(),
            boxes: [EntityId::new(2), EntityId::new(3), EntityId::new(4)]
                .into_iter()
                .map(|entity| {
                    (
                        entity,
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 1.0,
                            height: 1.0,
                        },
                    )
                })
                .collect(),
            diagnostics: Vec::new(),
        };
        let scene =
            build_scene(&document, &layout, &EvaluationContext::viewport(1.0, 1.0)).unwrap();
        assert!(scene.commands.is_empty());
        assert_eq!(scene.fidelity.len(), 3);
        assert!(
            scene
                .fidelity
                .iter()
                .all(|entry| matches!(entry.status, Fidelity::Unsupported { .. }))
        );
    }

    #[test]
    fn invalid_target_and_scene_are_rejected() {
        let target = RenderTarget {
            width: 1,
            height: 1,
            scale_factor: -1.0,
        };
        assert_eq!(
            render_cpu(&RenderScene::default(), target),
            Err(RenderError::InvalidTarget)
        );

        let scene = RenderScene {
            commands: vec![DrawCommand::Rect {
                entity: EntityId::new(9),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: f64::NAN,
                    height: 1.0,
                },
                fill: Color {
                    space: ColorSpace::Srgb,
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
            image_surfaces: BTreeMap::new(),
        };
        assert_eq!(
            render_cpu(
                &scene,
                RenderTarget {
                    width: 1,
                    height: 1,
                    scale_factor: 1.0,
                }
            ),
            Err(RenderError::InvalidScene {
                entity: EntityId::new(9)
            })
        );
    }

    #[test]
    fn scale_factor_maps_logical_geometry_to_device_pixels() {
        let scene = RenderScene {
            commands: vec![DrawCommand::Rect {
                entity: EntityId::new(1),
                rect: Rect {
                    x: 1.0,
                    y: 1.0,
                    width: 1.0,
                    height: 1.0,
                },
                fill: Color {
                    space: ColorSpace::Srgb,
                    red: 1.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
            image_surfaces: BTreeMap::new(),
        };
        let image = render_cpu(
            &scene,
            RenderTarget {
                width: 4,
                height: 4,
                scale_factor: 2.0,
            },
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * 4 + x) * 4..][..4];
        assert_eq!(pixel(1, 1), [255, 255, 255, 255]);
        assert_eq!(pixel(2, 2), [255, 0, 0, 255]);
        assert_eq!(pixel(3, 3), [255, 0, 0, 255]);
    }
}
