#![doc = "Renderer-independent scene boundary for NUIF."]

use nuif_core::EntityId;
use nuif_layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    Rect {
        entity: EntityId,
        rect: Rect,
        fill: Color,
    },
    Text {
        entity: EntityId,
        rect: Rect,
        text: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderScene {
    pub commands: Vec<DrawCommand>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

pub trait Renderer {
    type Output;
    type Error;

    fn render(
        &mut self,
        scene: &RenderScene,
        target: RenderTarget,
    ) -> Result<Self::Output, Self::Error>;
}
