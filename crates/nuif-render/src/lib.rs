#![doc = "Renderer-independent scene lowering and deterministic CPU rasterization."]

use nuif_core::{Color as ModelColor, Document, EntityId, EntityKind, Fidelity, ShapeKind};
use nuif_layout::{EvaluationContext, LayoutSnapshot, Rect, WritingDirection};
use nuif_text::{
    CLUSTER_UNIT, MAX_SHAPING_CODEPOINTS, ShapeRequest, ShapedRun, TextDirection, TextError, shape,
};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl From<ModelColor> for Color {
    fn from(value: ModelColor) -> Self {
        Self {
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
    Text {
        entity: EntityId,
        rect: Rect,
        run: Box<ShapedRun>,
        color: Color,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderScene {
    pub commands: Vec<DrawCommand>,
    pub fidelity: Vec<RenderFidelity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderFidelity {
    pub entity: EntityId,
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
    let mut scene = RenderScene::default();
    for (id, entity) in &document.entities {
        let Some(rect) = layout.boxes.get(id).copied() else {
            continue;
        };
        if let EntityKind::Unknown(unknown) = &entity.kind {
            scene.fidelity.push(RenderFidelity {
                entity: *id,
                status: Fidelity::PreservedUnrenderable {
                    namespace: unknown.namespace.clone(),
                },
            });
        } else if let Some(fill) = entity.authored.fill {
            if matches!(
                entity.kind,
                EntityKind::Shape(ShapeKind::Ellipse | ShapeKind::Path)
            ) {
                scene.fidelity.push(RenderFidelity {
                    entity: *id,
                    status: Fidelity::Approximated {
                        reason: "profile 0 rasterizes ellipse and path bounds as rectangles"
                            .to_owned(),
                    },
                });
            }
            scene.commands.push(DrawCommand::Rect {
                entity: *id,
                rect,
                fill: fill.into(),
            });
        }
        if let Some(text) = &entity.authored.text {
            if !context.font_hashes.contains(&text.font_sha256) {
                return Err(RenderError::Text {
                    entity: *id,
                    source: TextError::FontAbsentFromContext {
                        hash: text.font_sha256.clone(),
                    },
                });
            }
            let run = shape(&ShapeRequest {
                text: &text.content,
                font_sha256: &text.font_sha256,
                font_size: text.size,
                direction: match context.writing_direction {
                    WritingDirection::LeftToRight => TextDirection::LeftToRight,
                    WritingDirection::RightToLeft => TextDirection::RightToLeft,
                },
                language: &context.locale,
            })
            .map_err(|source| RenderError::Text {
                entity: *id,
                source,
            })?;
            scene.fidelity.push(RenderFidelity {
                entity: *id,
                status: Fidelity::Approximated {
                    reason: "profile 0 shapes pinned glyph runs exactly but rasterizes glyph IDs with a deterministic bitmap proxy instead of normative outlines".to_owned(),
                },
            });
            scene.commands.push(DrawCommand::Text {
                entity: *id,
                rect,
                run: Box::new(run),
                color: Color {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            });
        }
    }
    Ok(scene)
}

/// Deterministic profile-0 rasterizer. It intentionally supports only solid
/// rectangles and a fixed bitmap text proxy; unsupported semantics stay in the
/// scene fidelity report rather than disappearing.
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
        let (entity, rect, color, valid_text_run) = match command {
            DrawCommand::Rect { entity, rect, fill } => (*entity, *rect, *fill, true),
            DrawCommand::Text {
                entity,
                rect,
                color,
                run,
            } => (*entity, *rect, *color, shaped_run_is_valid(run)),
        };
        if !rect_is_valid(rect)
            || !rect_is_valid(scale_rect(rect, f64::from(target.scale_factor)))
            || !color_is_finite(color)
            || !valid_text_run
        {
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
            DrawCommand::Text {
                rect, run, color, ..
            } => draw_shaped_text_proxy(
                &mut image,
                scale_rect(*rect, f64::from(target.scale_factor)),
                run,
                f64::from(target.scale_factor),
                *color,
            ),
        }
    }
    Ok(image)
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
        .all(f32::is_finite)
}

fn shaped_run_is_valid(run: &ShapedRun) -> bool {
    let codepoints = run.text.chars().count();
    run.font_size.is_finite()
        && run.font_size > 0.0
        && run.units_per_em > 0
        && run.cluster_unit == CLUSTER_UNIT
        && codepoints <= MAX_SHAPING_CODEPOINTS
        && run
            .glyphs
            .iter()
            .all(|glyph| usize::try_from(glyph.cluster).is_ok_and(|cluster| cluster < codepoints))
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

fn draw_shaped_text_proxy(
    image: &mut RasterImage,
    rect: Rect,
    run: &ShapedRun,
    target_scale: f64,
    color: Color,
) {
    let source = [
        channel(color.red),
        channel(color.green),
        channel(color.blue),
        channel(color.alpha),
    ];
    let font_unit_scale = run.font_size * target_scale / f64::from(run.units_per_em);
    let codepoints = run.text.chars().collect::<Vec<_>>();
    let mut pen_x = rect.x;
    let mut pen_y = rect.y;
    for glyph in &run.glyphs {
        let glyph_x = floor_coordinate(
            pen_x + f64::from(glyph.x_offset) * font_unit_scale,
            image.width,
        );
        let glyph_y = floor_coordinate(
            pen_y - f64::from(glyph.y_offset) * font_unit_scale,
            image.height,
        );
        let is_space = usize::try_from(glyph.cluster)
            .ok()
            .and_then(|cluster| codepoints.get(cluster))
            .copied()
            .is_some_and(char::is_whitespace);
        let glyph_bits = glyph.glyph_id.to_le_bytes();
        for row in 0..7_u32 {
            for column in 0..5_u32 {
                let bit = (row * 5 + column) % 8;
                let byte = glyph_bits[usize::try_from((row + column) % 4).unwrap_or(0)];
                if !is_space && (byte >> bit) & 1 == 1 {
                    let x = glyph_x + column;
                    let y = glyph_y + row;
                    if x < image.width
                        && y < image.height
                        && f64::from(x) < rect.x + rect.width
                        && f64::from(y) < rect.y + rect.height
                    {
                        blend_pixel(image, x, y, source);
                    }
                }
            }
        }
        pen_x += f64::from(glyph.x_advance) * font_unit_scale;
        pen_y += f64::from(glyph.y_advance) * font_unit_scale;
    }
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
    use nuif_core::{Entity, SizeIntent, TextContent};
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
            size: 18.0,
            line_height: 20.0,
        });
        document.roots.push(text.id);
        document.entities.insert(text.id, text);
        let layout = nuif_layout::evaluate(&document, &EvaluationContext::viewport(100.0, 20.0));
        (document, layout)
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
                    red: 1.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
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
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
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
                    red: 1.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 1.0,
                },
            }],
            fidelity: Vec::new(),
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
