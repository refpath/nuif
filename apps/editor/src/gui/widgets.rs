use masonry::accesskit::{Action, ActionData, Node, Role};
use masonry::core::keyboard::{Key, NamedKey};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx,
    PointerButton, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Widget,
    WidgetId, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Rect, Size, Stroke, Vec2};
use masonry::layout::{LayoutSize, LenReq, Length, SizeDef};
use masonry::peniko::{Color, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use nuif_core::EntityId;
use nuif_layout::Rect as LayoutRect;
use serde::{Deserialize, Serialize};
use tracing::{Span, trace_span};

const CANVAS_BACKGROUND: Color = Color::from_rgb8(0x2C, 0x2C, 0x2C);
const CANVAS_GRID: Color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0x10);
const RULER_BACKGROUND: Color = Color::from_rgba8(0x1F, 0x1F, 0x1F, 0xF5);
const RULER_TICK: Color = Color::from_rgba8(0xEE, 0xEE, 0xEE, 0x78);
const RULER_BORDER: Color = Color::from_rgba8(0x00, 0x00, 0x00, 0x70);
const PAGE_BORDER: Color = Color::from_rgba8(0x00, 0x00, 0x00, 0x55);
const SELECTION: Color = Color::from_rgb8(0x55, 0x8D, 0xFF);
const RULER_SIZE: f64 = 24.0;
const RESIZE_HANDLE_SIZE: f64 = 8.0;
const RESIZE_HIT_SIZE: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasAction {
    Activate {
        entity: Option<EntityId>,
        document_position: Option<(f64, f64)>,
    },
    Move {
        entity: EntityId,
        document_delta: (f64, f64),
        document_position: (f64, f64),
        snap_to_pixel: bool,
    },
    Resize {
        entity: EntityId,
        start_size: (f64, f64),
        document_size: (f64, f64),
        handle: ResizeHandle,
        snap_to_pixel: bool,
    },
    Shortcut(CanvasShortcut),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResizeHandle {
    NorthWest,
    North,
    NorthEast,
    East,
    #[default]
    SouthEast,
    South,
    SouthWest,
    West,
}

impl ResizeHandle {
    const ALL: [Self; 8] = [
        Self::NorthWest,
        Self::NorthEast,
        Self::SouthEast,
        Self::SouthWest,
        Self::North,
        Self::East,
        Self::South,
        Self::West,
    ];
    const TRAILING: [Self; 3] = [Self::SouthEast, Self::East, Self::South];

    pub(super) const fn horizontal_direction(self) -> i8 {
        match self {
            Self::NorthWest | Self::SouthWest | Self::West => -1,
            Self::North | Self::South => 0,
            Self::NorthEast | Self::East | Self::SouthEast => 1,
        }
    }

    pub(super) const fn vertical_direction(self) -> i8 {
        match self {
            Self::NorthWest | Self::North | Self::NorthEast => -1,
            Self::East | Self::West => 0,
            Self::SouthEast | Self::South | Self::SouthWest => 1,
        }
    }

    const fn is_corner(self) -> bool {
        self.horizontal_direction() != 0 && self.vertical_direction() != 0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum CanvasResizePolicy {
    #[default]
    None,
    Trailing,
    Freeform,
}

#[derive(Clone, Copy, Debug)]
enum CanvasDragKind {
    Move,
    Resize {
        handle: ResizeHandle,
        start_rect: LayoutRect,
    },
}

#[derive(Clone, Copy, Debug)]
struct CanvasDrag {
    entity: Option<EntityId>,
    kind: CanvasDragKind,
    start_screen: Point,
    start_document: Option<Point>,
    current_screen: Point,
    current_document: Option<Point>,
    preserve_aspect_ratio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasShortcut {
    Tool(char),
    Undo,
    Redo,
    Save,
    Duplicate,
    Delete,
    Palette,
    Export,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    ZoomActual,
    ToggleUi,
    ToggleGrid,
    ToggleRulers,
}

pub struct DocumentCanvas {
    image: ImageBrush,
    image_size: Size,
    boxes: Vec<(EntityId, LayoutRect)>,
    selection: Option<EntityId>,
    zoom: f64,
    show_grid: bool,
    show_rulers: bool,
    resize_policy: CanvasResizePolicy,
    size: Size,
    drag: Option<CanvasDrag>,
}

impl DocumentCanvas {
    pub fn new(
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        boxes: Vec<(EntityId, LayoutRect)>,
        selection: Option<EntityId>,
    ) -> Self {
        Self {
            image: ImageBrush::new(ImageData {
                data: rgba.into(),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::Alpha,
                width,
                height,
            }),
            image_size: Size::new(f64::from(width), f64::from(height)),
            boxes,
            selection,
            zoom: 1.0,
            show_grid: true,
            show_rulers: true,
            resize_policy: CanvasResizePolicy::None,
            size: Size::ZERO,
            drag: None,
        }
    }

    pub fn with_view_options(mut self, zoom: f64, show_grid: bool, show_rulers: bool) -> Self {
        self.zoom = zoom;
        self.show_grid = show_grid;
        self.show_rulers = show_rulers;
        self
    }

    pub(super) fn with_resize_policy(mut self, policy: CanvasResizePolicy) -> Self {
        self.resize_policy = policy;
        self
    }

    fn page_transform(&self) -> (f64, Vec2) {
        let available_width = (self.size.width - 96.0).max(1.0);
        let available_height = (self.size.height - 96.0).max(1.0);
        let scale = ((available_width / self.image_size.width)
            .min(available_height / self.image_size.height)
            .clamp(0.05, 2.0)
            * self.zoom)
            .clamp(0.05, 8.0);
        let offset = Vec2::new(
            (self.size.width - self.image_size.width * scale) * 0.5,
            (self.size.height - self.image_size.height * scale) * 0.5,
        );
        (scale, offset)
    }

    fn page_rect(&self) -> Rect {
        let (scale, offset) = self.page_transform();
        Rect::from_origin_size(
            offset.to_point(),
            (
                self.image_size.width * scale,
                self.image_size.height * scale,
            ),
        )
    }

    fn entity_at(&self, point: Point) -> Option<EntityId> {
        let (scale, offset) = self.page_transform();
        let document_point = Point::new((point.x - offset.x) / scale, (point.y - offset.y) / scale);
        self.boxes
            .iter()
            .filter(|(_, rect)| {
                document_point.x >= rect.x
                    && document_point.x <= rect.x + rect.width
                    && document_point.y >= rect.y
                    && document_point.y <= rect.y + rect.height
            })
            .min_by(|(_, left), (_, right)| {
                (left.width * left.height)
                    .partial_cmp(&(right.width * right.height))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(entity, _)| *entity)
    }

    fn document_position(&self, point: Point) -> Option<(f64, f64)> {
        let point = self.document_coordinates(point);
        (point.x >= 0.0
            && point.x <= self.image_size.width
            && point.y >= 0.0
            && point.y <= self.image_size.height)
            .then_some((point.x, point.y))
    }

    fn document_coordinates(&self, point: Point) -> Point {
        let (scale, offset) = self.page_transform();
        Point::new((point.x - offset.x) / scale, (point.y - offset.y) / scale)
    }

    fn resize_handles(&self) -> &'static [ResizeHandle] {
        match self.resize_policy {
            CanvasResizePolicy::None => &[],
            CanvasResizePolicy::Trailing => &ResizeHandle::TRAILING,
            CanvasResizePolicy::Freeform => &ResizeHandle::ALL,
        }
    }

    fn resize_handle_point(&self, rect: &LayoutRect, handle: ResizeHandle) -> Point {
        let (scale, offset) = self.page_transform();
        let horizontal = f64::from(handle.horizontal_direction());
        let vertical = f64::from(handle.vertical_direction());
        let x = rect.x + rect.width * (horizontal + 1.0) * 0.5;
        let y = rect.y + rect.height * (vertical + 1.0) * 0.5;
        Point::new(offset.x + x * scale, offset.y + y * scale)
    }

    fn resize_target(&self, point: Point) -> Option<(EntityId, LayoutRect, ResizeHandle)> {
        let entity = self.selection?;
        let (_, rect) = self.boxes.iter().find(|(id, _)| *id == entity)?;
        let half = RESIZE_HIT_SIZE * 0.5;
        self.resize_handles().iter().copied().find_map(|handle| {
            let handle_point = self.resize_handle_point(rect, handle);
            (point.x >= handle_point.x - half
                && point.x <= handle_point.x + half
                && point.y >= handle_point.y - half
                && point.y <= handle_point.y + half)
                .then_some((entity, *rect, handle))
        })
    }

    #[cfg(feature = "editor-automation")]
    pub(super) fn local_entity_center(&self, entity: EntityId) -> Option<Point> {
        let (_, rect) = self.boxes.iter().find(|(id, _)| *id == entity)?;
        let (scale, offset) = self.page_transform();
        Some(Point::new(
            offset.x + (rect.x + rect.width * 0.5) * scale,
            offset.y + (rect.y + rect.height * 0.5) * scale,
        ))
    }

    #[cfg(feature = "editor-automation")]
    pub(super) fn local_document_delta(&self, delta: (f64, f64)) -> Vec2 {
        let (scale, _) = self.page_transform();
        Vec2::new(delta.0 * scale, delta.1 * scale)
    }

    #[cfg(feature = "editor-automation")]
    pub(super) fn local_resize_handle(
        &self,
        entity: EntityId,
        handle: ResizeHandle,
    ) -> Option<Point> {
        (self.selection == Some(entity)).then_some(())?;
        let (_, rect) = self.boxes.iter().find(|(id, _)| *id == entity)?;
        self.resize_handles().contains(&handle).then_some(())?;
        Some(self.resize_handle_point(rect, handle))
    }

    fn finish_drag(
        &mut self,
        point: Point,
        snap_to_pixel: bool,
        preserve_aspect_ratio: bool,
    ) -> Option<CanvasAction> {
        let mut drag = self.drag.take()?;
        drag.current_screen = point;
        drag.current_document = Some(self.document_coordinates(point));
        drag.preserve_aspect_ratio = preserve_aspect_ratio;
        let moved = (drag.current_screen - drag.start_screen).hypot2() >= 9.0;
        Some(
            match (
                moved,
                drag.entity,
                drag.start_document,
                drag.current_document,
                drag.kind,
            ) {
                (true, Some(entity), Some(start), Some(current), CanvasDragKind::Move) => {
                    CanvasAction::Move {
                        entity,
                        document_delta: (current.x - start.x, current.y - start.y),
                        document_position: (current.x, current.y),
                        snap_to_pixel,
                    }
                }
                (
                    true,
                    Some(entity),
                    Some(start),
                    Some(current),
                    CanvasDragKind::Resize { handle, start_rect },
                ) => {
                    let resized = resized_rect(
                        start_rect,
                        handle,
                        current - start,
                        drag.preserve_aspect_ratio,
                    );
                    CanvasAction::Resize {
                        entity,
                        start_size: (start_rect.width, start_rect.height),
                        document_size: (resized.width, resized.height),
                        handle,
                        snap_to_pixel,
                    }
                }
                _ => CanvasAction::Activate {
                    entity: drag.entity,
                    document_position: drag.start_document.map(|point| (point.x, point.y)),
                },
            },
        )
    }
}

fn resized_rect(
    start: LayoutRect,
    handle: ResizeHandle,
    delta: Vec2,
    preserve_aspect_ratio: bool,
) -> LayoutRect {
    let horizontal = f64::from(handle.horizontal_direction());
    let vertical = f64::from(handle.vertical_direction());
    let mut width = if horizontal == 0.0 {
        start.width
    } else {
        (start.width + delta.x * horizontal).max(1.0)
    };
    let mut height = if vertical == 0.0 {
        start.height
    } else {
        (start.height + delta.y * vertical).max(1.0)
    };
    if preserve_aspect_ratio && handle.is_corner() && start.width > 0.0 && start.height > 0.0 {
        let width_scale = width / start.width;
        let height_scale = height / start.height;
        let scale = if (width_scale - 1.0).abs() >= (height_scale - 1.0).abs() {
            width_scale
        } else {
            height_scale
        }
        .max((1.0 / start.width).max(1.0 / start.height));
        width = start.width * scale;
        height = start.height * scale;
    }
    LayoutRect {
        x: if horizontal < 0.0 {
            start.x + start.width - width
        } else {
            start.x
        },
        y: if vertical < 0.0 {
            start.y + start.height - height
        } else {
            start.y
        },
        width,
        height,
    }
}

fn visible_grid_step(scale: f64) -> f64 {
    let mut step = 8.0;
    while step * scale < 24.0 && step < 4096.0 {
        step *= 2.0;
    }
    step
}

fn paint_grid(painter: &mut Painter<'_>, content: Rect, scale: f64, offset: Vec2) {
    let spacing = visible_grid_step(scale) * scale;
    let mut x = offset.x.rem_euclid(spacing);
    while x < content.width() {
        painter
            .fill(Rect::new(x, 0.0, x + 1.0, content.height()), CANVAS_GRID)
            .draw();
        x += spacing;
    }
    let mut y = offset.y.rem_euclid(spacing);
    while y < content.height() {
        painter
            .fill(Rect::new(0.0, y, content.width(), y + 1.0), CANVAS_GRID)
            .draw();
        y += spacing;
    }
}

fn paint_rulers(painter: &mut Painter<'_>, content: Rect, scale: f64, offset: Vec2) {
    painter
        .fill(
            Rect::new(0.0, 0.0, content.width(), RULER_SIZE),
            RULER_BACKGROUND,
        )
        .draw();
    painter
        .fill(
            Rect::new(0.0, 0.0, RULER_SIZE, content.height()),
            RULER_BACKGROUND,
        )
        .draw();
    painter
        .fill(
            Rect::new(0.0, RULER_SIZE - 1.0, content.width(), RULER_SIZE),
            RULER_BORDER,
        )
        .draw();
    painter
        .fill(
            Rect::new(RULER_SIZE - 1.0, 0.0, RULER_SIZE, content.height()),
            RULER_BORDER,
        )
        .draw();

    let step = visible_grid_step(scale);
    let screen_step = step * scale;
    let major_step = step * 4.0;
    let mut document_x = ((-offset.x / scale) / step).floor() * step;
    let mut screen_x = offset.x + document_x * scale;
    while screen_x < content.width() {
        if screen_x >= RULER_SIZE {
            let major = (document_x / major_step).fract().abs() < f64::EPSILON;
            let height = if major { 11.0 } else { 6.0 };
            painter
                .fill(
                    Rect::new(screen_x, RULER_SIZE - height, screen_x + 1.0, RULER_SIZE),
                    RULER_TICK,
                )
                .draw();
        }
        document_x += step;
        screen_x += screen_step;
    }

    let mut document_y = ((-offset.y / scale) / step).floor() * step;
    let mut screen_y = offset.y + document_y * scale;
    while screen_y < content.height() {
        if screen_y >= RULER_SIZE {
            let major = (document_y / major_step).fract().abs() < f64::EPSILON;
            let width = if major { 11.0 } else { 6.0 };
            painter
                .fill(
                    Rect::new(RULER_SIZE - width, screen_y, RULER_SIZE, screen_y + 1.0),
                    RULER_TICK,
                )
                .draw();
        }
        document_y += step;
        screen_y += screen_step;
    }

    if (RULER_SIZE..content.width()).contains(&offset.x) {
        painter
            .fill(
                Rect::new(offset.x, 0.0, offset.x + 1.0, RULER_SIZE),
                SELECTION,
            )
            .draw();
    }
    if (RULER_SIZE..content.height()).contains(&offset.y) {
        painter
            .fill(
                Rect::new(0.0, offset.y, RULER_SIZE, offset.y + 1.0),
                SELECTION,
            )
            .draw();
    }
}

impl Widget for DocumentCanvas {
    type Action = CanvasAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(event)
                if matches!(event.button, None | Some(PointerButton::Primary)) =>
            {
                ctx.request_focus();
                ctx.capture_pointer();
                let point = ctx.local_position(event.state.position);
                let resize = self.resize_target(point);
                let start_document = if resize.is_some() {
                    Some(self.document_coordinates(point))
                } else {
                    self.document_position(point).map(|(x, y)| Point::new(x, y))
                };
                self.drag = Some(CanvasDrag {
                    entity: resize
                        .map_or_else(|| self.entity_at(point), |(entity, _, _)| Some(entity)),
                    kind: resize.map_or(CanvasDragKind::Move, |(_, start_rect, handle)| {
                        CanvasDragKind::Resize { handle, start_rect }
                    }),
                    start_screen: point,
                    start_document,
                    current_screen: point,
                    current_document: self.document_position(point).map(|(x, y)| Point::new(x, y)),
                    preserve_aspect_ratio: event.state.modifiers.shift(),
                });
                ctx.set_handled();
            }
            PointerEvent::Move(event) if ctx.is_active() => {
                let point = ctx.local_position(event.current.position);
                let document = self.document_coordinates(point);
                if let Some(drag) = &mut self.drag {
                    drag.current_screen = point;
                    drag.current_document = Some(document);
                    drag.preserve_aspect_ratio = event.current.modifiers.shift();
                    ctx.request_paint_only();
                    ctx.set_handled();
                }
            }
            PointerEvent::Up(event)
                if matches!(event.button, None | Some(PointerButton::Primary)) =>
            {
                let point = ctx.local_position(event.state.position);
                if let Some(action) = self.finish_drag(
                    point,
                    !event.state.modifiers.ctrl(),
                    event.state.modifiers.shift(),
                ) {
                    ctx.submit_action::<Self::Action>(action);
                    ctx.request_paint_only();
                    ctx.set_handled();
                }
            }
            PointerEvent::Cancel(_) => {
                self.drag = None;
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let TextEvent::Keyboard(event) = event else {
            return;
        };
        if !event.state.is_down() {
            return;
        }
        let command = if cfg!(target_os = "macos") {
            event.modifiers.meta()
        } else {
            event.modifiers.ctrl()
        };
        let shortcut = match &event.key {
            Key::Character(value) if command && value.eq_ignore_ascii_case("z") => {
                Some(if event.modifiers.shift() {
                    CanvasShortcut::Redo
                } else {
                    CanvasShortcut::Undo
                })
            }
            Key::Character(value) if command && value.eq_ignore_ascii_case("y") => {
                Some(CanvasShortcut::Redo)
            }
            Key::Character(value) if command && value.eq_ignore_ascii_case("s") => {
                Some(CanvasShortcut::Save)
            }
            Key::Character(value) if command && value.eq_ignore_ascii_case("d") => {
                Some(CanvasShortcut::Duplicate)
            }
            Key::Character(value) if command && value.eq_ignore_ascii_case("k") => {
                Some(CanvasShortcut::Palette)
            }
            Key::Character(value)
                if command && event.modifiers.shift() && value.eq_ignore_ascii_case("e") =>
            {
                Some(CanvasShortcut::Export)
            }
            Key::Character(value) if command && matches!(value.as_str(), "+" | "=") => {
                Some(CanvasShortcut::ZoomIn)
            }
            Key::Character(value) if command && value == "-" => Some(CanvasShortcut::ZoomOut),
            Key::Character(value) if event.modifiers.shift() && value == "1" => {
                Some(CanvasShortcut::ZoomFit)
            }
            Key::Character(value) if event.modifiers.shift() && value == "0" => {
                Some(CanvasShortcut::ZoomActual)
            }
            Key::Character(value) if event.modifiers.shift() && value.eq_ignore_ascii_case("r") => {
                Some(CanvasShortcut::ToggleRulers)
            }
            Key::Character(value) if command && value == "'" => Some(CanvasShortcut::ToggleGrid),
            Key::Character(value) if command && value == "\\" => Some(CanvasShortcut::ToggleUi),
            Key::Character(value)
                if !command && !event.modifiers.alt() && value.chars().count() == 1 =>
            {
                value.chars().next().map(CanvasShortcut::Tool)
            }
            Key::Named(NamedKey::Delete | NamedKey::Backspace) => Some(CanvasShortcut::Delete),
            _ => None,
        };
        if let Some(shortcut) = shortcut {
            ctx.submit_action::<Self::Action>(CanvasAction::Shortcut(shortcut));
            ctx.set_handled();
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if event.action == masonry::accesskit::Action::Click {
            ctx.submit_action::<Self::Action>(CanvasAction::Activate {
                entity: None,
                document_position: None,
            });
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match len_req {
            LenReq::FitContent(space) => space,
            LenReq::MinContent | LenReq::MaxContent => Length::const_px(320.0),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.size = size;
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let content = ctx.content_box();
        painter.fill(content, CANVAS_BACKGROUND).draw();
        let (scale, offset) = self.page_transform();
        if self.show_grid {
            paint_grid(painter, content, scale, offset);
        }

        let page = self.page_rect();
        painter.fill(page.inflate(1.0, 1.0), PAGE_BORDER).draw();
        let transform = Affine::translate(offset) * Affine::scale(scale);
        painter.draw_image(&self.image, transform);

        let mut selection = self.selection;
        let mut preview_delta = Vec2::ZERO;
        let mut resize_preview = None;
        if let Some(drag) = &self.drag {
            selection = drag.entity.or(selection);
            if let (Some(start), Some(current)) = (drag.start_document, drag.current_document) {
                preview_delta = current - start;
            }
            if let CanvasDragKind::Resize { handle, start_rect } = drag.kind {
                resize_preview = Some(resized_rect(
                    start_rect,
                    handle,
                    preview_delta,
                    drag.preserve_aspect_ratio,
                ));
            }
        }
        if let Some(selection) = selection
            && let Some((_, rect)) = self.boxes.iter().find(|(entity, _)| *entity == selection)
        {
            let preview = if let Some(resized) = resize_preview {
                resized
            } else {
                LayoutRect {
                    x: rect.x + preview_delta.x,
                    y: rect.y + preview_delta.y,
                    width: rect.width,
                    height: rect.height,
                }
            };
            let selected = Rect::new(
                offset.x + preview.x * scale,
                offset.y + preview.y * scale,
                offset.x + (preview.x + preview.width) * scale,
                offset.y + (preview.y + preview.height) * scale,
            );
            painter
                .stroke(selected, &Stroke::new(2.0), SELECTION)
                .draw();
            for handle in self.resize_handles() {
                let handle_point = Point::new(
                    selected.x0
                        + selected.width() * (f64::from(handle.horizontal_direction()) + 1.0) * 0.5,
                    selected.y0
                        + selected.height() * (f64::from(handle.vertical_direction()) + 1.0) * 0.5,
                );
                painter
                    .fill(
                        Rect::from_center_size(
                            handle_point,
                            Size::new(RESIZE_HANDLE_SIZE, RESIZE_HANDLE_SIZE),
                        ),
                        SELECTION,
                    )
                    .draw();
            }
        }
        if self.show_rulers {
            paint_rulers(painter, content, scale, offset);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("Document canvas");
        node.set_description(format!(
            "Rendered NUIF profile-zero document in pixels; background grid {}; rulers {}",
            if self.show_grid { "shown" } else { "hidden" },
            if self.show_rulers { "shown" } else { "hidden" }
        ));
        node.add_action(masonry::accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("DocumentCanvas", id = id.trace())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_resize_handle_preserves_its_opposite_edges() {
        let start = LayoutRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        for (handle, delta, expected) in [
            (
                ResizeHandle::NorthWest,
                Vec2::new(-20.0, -10.0),
                LayoutRect {
                    x: -10.0,
                    y: 10.0,
                    width: 120.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::North,
                Vec2::new(99.0, -10.0),
                LayoutRect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::NorthEast,
                Vec2::new(20.0, -10.0),
                LayoutRect {
                    x: 10.0,
                    y: 10.0,
                    width: 120.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::East,
                Vec2::new(20.0, 99.0),
                LayoutRect {
                    x: 10.0,
                    y: 20.0,
                    width: 120.0,
                    height: 50.0,
                },
            ),
            (
                ResizeHandle::SouthEast,
                Vec2::new(20.0, 10.0),
                LayoutRect {
                    x: 10.0,
                    y: 20.0,
                    width: 120.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::South,
                Vec2::new(99.0, 10.0),
                LayoutRect {
                    x: 10.0,
                    y: 20.0,
                    width: 100.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::SouthWest,
                Vec2::new(-20.0, 10.0),
                LayoutRect {
                    x: -10.0,
                    y: 20.0,
                    width: 120.0,
                    height: 60.0,
                },
            ),
            (
                ResizeHandle::West,
                Vec2::new(-20.0, 99.0),
                LayoutRect {
                    x: -10.0,
                    y: 20.0,
                    width: 120.0,
                    height: 50.0,
                },
            ),
        ] {
            assert_eq!(resized_rect(start, handle, delta, false), expected);
        }
    }

    #[test]
    fn corner_resize_can_preserve_aspect_ratio() {
        let start = LayoutRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(
            resized_rect(start, ResizeHandle::NorthWest, Vec2::new(-20.0, -1.0), true,),
            LayoutRect {
                x: -10.0,
                y: 10.0,
                width: 120.0,
                height: 60.0,
            }
        );
    }
}

pub struct AuthorContainer {
    child: WidgetPod<dyn Widget>,
    author_id: EntityId,
    label: String,
    role: Role,
    value: Option<String>,
    behavior: AuthorBehavior,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuthorAction {
    Select {
        author_id: EntityId,
    },
    SetValue {
        author_id: EntityId,
        label: String,
        value: String,
    },
}

enum AuthorBehavior {
    Select,
    SetValue { label: String },
}

impl AuthorContainer {
    pub fn tree_item(
        child: NewWidget<impl Widget + ?Sized>,
        entity: EntityId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            child: child.erased().to_pod(),
            author_id: entity,
            label: label.into(),
            role: Role::TreeItem,
            value: None,
            behavior: AuthorBehavior::Select,
        }
    }

    pub fn value_control(
        child: NewWidget<impl Widget + ?Sized>,
        entity: EntityId,
        label: impl Into<String>,
        semantic_label: impl Into<String>,
        role: Role,
        value: impl Into<String>,
    ) -> Self {
        Self {
            child: child.erased().to_pod(),
            author_id: entity,
            label: label.into(),
            role,
            value: Some(value.into()),
            behavior: AuthorBehavior::SetValue {
                label: semantic_label.into(),
            },
        }
    }
}

impl Widget for AuthorContainer {
    type Action = AuthorAction;

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match (&self.behavior, event.action, &event.data) {
            (AuthorBehavior::Select, Action::Click, _) => {
                ctx.submit_action::<Self::Action>(AuthorAction::Select {
                    author_id: self.author_id,
                });
            }
            (AuthorBehavior::SetValue { label }, Action::SetValue, Some(data)) => {
                let value = match data {
                    ActionData::Value(value) => value.to_string(),
                    ActionData::NumericValue(value) => value.to_string(),
                    _ => return,
                };
                ctx.submit_action::<Self::Action>(AuthorAction::SetValue {
                    author_id: self.author_id,
                    label: label.clone(),
                    value,
                });
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        ctx.compute_length(
            &mut self.child,
            len_req.into(),
            LayoutSize::maybe(axis.cross(), cross_length),
            axis,
            cross_length,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let child_size = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child_size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        ctx.derive_baselines(&self.child);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> Role {
        self.role
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_author_id(self.author_id.to_string());
        node.set_label(self.label.as_str());
        if let Some(value) = &self.value {
            node.set_value(value.as_str());
        }
        match self.behavior {
            AuthorBehavior::Select => node.add_action(Action::Click),
            AuthorBehavior::SetValue { .. } => node.add_action(Action::SetValue),
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("AuthorContainer", id = id.trace())
    }
}
