//! Native Masonry application shell.

mod widgets;

#[cfg(feature = "editor-automation")]
pub mod automation;

use self::widgets::{AuthorAction, AuthorContainer, CanvasAction, CanvasShortcut, DocumentCanvas};
use crate::{AccessibilityAction, EditorCommand, EditorDriver, EditorEvent};
use masonry::core::{ErasedAction, NewWidget, StyleProperty, Widget, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::layout::{AsUnit, Length, UnitPoint};
use masonry::parley::style::FontWeight;
use masonry::peniko::Color as UiColor;
use masonry::properties::{
    Background, BorderColor, BorderWidth, ContentColor, CornerRadius, Dimensions, Gap, Padding,
};
use masonry::theme::default_property_set;
use masonry::widgets::{
    Button, ButtonPress, Flex, Label, Portal, SizedBox, TextAction, TextInput, ZStack,
};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;
use nuif_adapter::{AdapterReport, ExportedSource, ImportedSource};
use nuif_codec::{
    CanonicalText, Decoder, Encoder, MAX_INPUT_BYTES, read_bounded as read_bounded_stream,
};
use nuif_core::{
    Align, Color, Document, Entity, EntityId, EntityKind, Fidelity, FlowDirection, LayoutFamily,
    Point, ShapeKind, SizeIntent, TextContent, validate,
};
use nuif_protocol::{Anchor, Axis as ProtocolAxis, Operation};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VIEWPORT_WIDTH: u32 = 768;
const VIEWPORT_HEIGHT: u32 = 640;
const LEFT_PANEL_WIDTH: Length = Length::const_px(272.0);
const RIGHT_PANEL_WIDTH: Length = Length::const_px(264.0);
const TOP_BAR_HEIGHT: Length = Length::const_px(48.0);
const STATUS_HEIGHT: Length = Length::const_px(28.0);
const PANEL: UiColor = UiColor::from_rgb8(0x1F, 0x1F, 0x1F);
const PANEL_RAISED: UiColor = UiColor::from_rgb8(0x28, 0x28, 0x28);
const BORDER: UiColor = UiColor::from_rgb8(0x39, 0x39, 0x39);
const TEXT: UiColor = UiColor::from_rgb8(0xEE, 0xEE, 0xEE);
const MUTED: UiColor = UiColor::from_rgb8(0xA7, 0xA7, 0xA7);
const ACCENT: UiColor = UiColor::from_rgb8(0x55, 0x8D, 0xFF);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tool {
    Move,
    Hand,
    Frame,
    Rectangle,
    Ellipse,
    Pen,
    Text,
}

impl Tool {
    const ALL: [(Self, &'static str, &'static str); 7] = [
        (Self::Move, "V", "Move"),
        (Self::Hand, "H", "Hand"),
        (Self::Frame, "F", "Frame"),
        (Self::Rectangle, "R", "Rectangle"),
        (Self::Ellipse, "O", "Ellipse"),
        (Self::Pen, "P", "Pen"),
        (Self::Text, "T", "Text"),
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiAction {
    New,
    ImportNative,
    ImportExternal(ExternalFormat),
    Save,
    SaveAs,
    Undo,
    Redo,
    Select(EntityId),
    AddPage,
    DeleteSelection,
    DuplicateSelection,
    ApplyInspector,
    ExportSnapshot,
    ExportExternal(ExternalFormat),
    ChooseTool(Tool),
    ChooseLeftPanel(LeftPanel),
    SetLayoutFamily(LayoutFamily),
    SetDirection(FlowDirection),
    SetAlign(Align),
    SetViewport(u32),
    ZoomIn,
    ZoomOut,
    ZoomFit,
    ZoomActual,
    ToggleLeftPanel,
    ToggleRightPanel,
    ToggleUi,
    TogglePalette,
    ToggleFileMenu,
    ToggleGrid,
    ToggleRulers,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExternalFormat {
    Svg,
    HtmlCss,
    Dtcg,
}

impl ExternalFormat {
    const ALL: [Self; 3] = [Self::Svg, Self::HtmlCss, Self::Dtcg];

    const fn name(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::HtmlCss => "HTML/CSS",
            Self::Dtcg => "DTCG tokens",
        }
    }

    const fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Svg => ("SVG profile", &["svg"]),
            Self::HtmlCss => ("HTML/CSS profile", &["html", "htm"]),
            Self::Dtcg => ("DTCG token profile", &["json"]),
        }
    }

    const fn default_file_name(self) -> &'static str {
        match self {
            Self::Svg => "nuif-export.svg",
            Self::HtmlCss => "nuif-export.html",
            Self::Dtcg => "nuif-export.tokens.json",
        }
    }

    const fn source_limit(self) -> usize {
        match self {
            Self::Svg => nuif_svg::MAX_SOURCE_BYTES,
            Self::HtmlCss => nuif_html::MAX_SOURCE_BYTES,
            Self::Dtcg => nuif_dtcg::MAX_SOURCE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LeftPanel {
    Pages,
    Layers,
    Components,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum InspectorField {
    Name(EntityId),
    Width(EntityId),
    Height(EntityId),
    X(EntityId),
    Y(EntityId),
    Gap(EntityId),
    PaddingTop(EntityId),
    PaddingRight(EntityId),
    PaddingBottom(EntityId),
    PaddingLeft(EntityId),
    Fill(EntityId),
    TextContent(EntityId),
    FontSize(EntityId),
    LineHeight(EntityId),
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent ephemeral editor-view toggles are clearer as named flags"
)]
struct Driver {
    window_id: WindowId,
    root_widget_id: Option<WidgetId>,
    editor: EditorDriver,
    document_path: Option<PathBuf>,
    dirty: bool,
    status: String,
    tool: Tool,
    left_panel: LeftPanel,
    viewport_width: u32,
    zoom: f64,
    show_left_panel: bool,
    show_right_panel: bool,
    hide_ui: bool,
    show_palette: bool,
    show_file_menu: bool,
    show_grid: bool,
    show_rulers: bool,
    actions: HashMap<WidgetId, UiAction>,
    entity_widgets: HashMap<EntityId, WidgetId>,
    control_widgets: HashMap<(EntityId, &'static str), WidgetId>,
    text_fields: HashMap<WidgetId, InspectorField>,
    drafts: HashMap<InspectorField, String>,
}

impl Driver {
    fn new(window_id: WindowId, document: Document, document_path: Option<PathBuf>) -> Self {
        Self {
            window_id,
            root_widget_id: None,
            editor: EditorDriver::new(document),
            document_path,
            dirty: false,
            status: "Ready · profile 0 · px · 768 × 640".to_owned(),
            tool: Tool::Move,
            left_panel: LeftPanel::Layers,
            viewport_width: VIEWPORT_WIDTH,
            zoom: 1.0,
            show_left_panel: true,
            show_right_panel: true,
            hide_ui: false,
            show_palette: false,
            show_file_menu: false,
            show_grid: true,
            show_rulers: true,
            actions: HashMap::new(),
            entity_widgets: HashMap::new(),
            control_widgets: HashMap::new(),
            text_fields: HashMap::new(),
            drafts: HashMap::new(),
        }
    }

    fn build_view(&mut self) -> NewWidget<dyn Widget> {
        self.actions.clear();
        self.entity_widgets.clear();
        self.control_widgets.clear();
        self.text_fields.clear();

        let snapshot = match self.editor.execute(EditorCommand::Snapshot {
            width: self.viewport_width,
            height: VIEWPORT_HEIGHT,
        }) {
            Ok(EditorEvent::Snapshot { snapshot }) => Some(snapshot),
            Ok(_) => unreachable!("snapshot command returns snapshot event"),
            Err(error) => {
                self.status = format!("Render error: {error}");
                None
            }
        };

        let selection = self.editor.selection().first().copied();
        let (rgba, boxes) = snapshot.as_ref().map_or_else(
            || {
                (
                    vec![0xFF; self.viewport_width as usize * VIEWPORT_HEIGHT as usize * 4],
                    Vec::new(),
                )
            },
            |snapshot| {
                (
                    snapshot.raster.rgba.clone(),
                    snapshot
                        .layout
                        .boxes
                        .iter()
                        .map(|(entity, rect)| (*entity, *rect))
                        .collect(),
                )
            },
        );
        let canvas =
            DocumentCanvas::new(self.viewport_width, VIEWPORT_HEIGHT, rgba, boxes, selection)
                .with_view_options(self.zoom, self.show_grid, self.show_rulers)
                .prepare()
                .with_props(Dimensions::STRETCH);
        let mut canvas_stack = ZStack::new().with(canvas, UnitPoint::CENTER);
        if !self.hide_ui {
            canvas_stack = canvas_stack.with(self.build_toolbar(), UnitPoint::BOTTOM);
        }
        if self.show_palette {
            canvas_stack = canvas_stack.with(self.build_command_palette(), UnitPoint::CENTER);
        }
        let canvas_region = canvas_stack.prepare().with_props(Dimensions::STRETCH);

        let mut body = Flex::row();
        if !self.hide_ui && self.show_left_panel {
            body = body.with_fixed(self.build_left_panel(selection));
        }
        body = body.with(canvas_region, 1.0);
        if !self.hide_ui && self.show_right_panel {
            body = body.with_fixed(self.build_right_panel(selection));
        }
        let status = self.build_status();
        let mut root = Flex::column();
        if !self.hide_ui {
            root = root.with_fixed(self.build_top_bar());
        }
        root = root.with(body.prepare(), 1.0).with_fixed(status);
        let shell = NewWidget::new(root)
            .with_props((Gap::ZERO, Dimensions::STRETCH))
            .erased();
        if self.show_file_menu && !self.hide_ui {
            ZStack::new()
                .with(shell, UnitPoint::CENTER)
                .with(self.build_file_menu_anchor(), UnitPoint::TOP_LEFT)
                .prepare()
                .with_props(Dimensions::STRETCH)
                .erased()
        } else {
            shell
        }
    }

    fn build_top_bar(&mut self) -> NewWidget<dyn Widget> {
        let title = self
            .document_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.nuif")
            .to_owned();
        let row = Flex::row()
            .with_fixed(label("NUIF", 14.0, TEXT, true))
            .with_fixed_spacer(10.px())
            .with_fixed(self.button("File", UiAction::ToggleFileMenu, self.show_file_menu))
            .with_fixed(self.button("Undo", UiAction::Undo, false))
            .with_fixed(self.button("Redo", UiAction::Redo, false))
            .with_fixed_spacer(10.px())
            .with(label(&title, 12.0, TEXT, true), 1.0)
            .with_fixed(self.button(
                "360",
                UiAction::SetViewport(360),
                self.viewport_width == 360,
            ))
            .with_fixed(self.button(
                "768",
                UiAction::SetViewport(768),
                self.viewport_width == 768,
            ))
            .with_fixed(self.button(
                "1440",
                UiAction::SetViewport(1440),
                self.viewport_width == 1440,
            ))
            .with_fixed_spacer(8.px())
            .with_fixed(self.button("−", UiAction::ZoomOut, false))
            .with_fixed(label(
                &format!("{}%", (self.zoom * 100.0).round()),
                11.0,
                TEXT,
                false,
            ))
            .with_fixed(self.button("+", UiAction::ZoomIn, false))
            .with_fixed(label("px", 11.0, MUTED, true))
            .with_fixed(self.button("Grid", UiAction::ToggleGrid, self.show_grid))
            .with_fixed(self.button("Rulers", UiAction::ToggleRulers, self.show_rulers))
            .with_fixed(self.button("Layers", UiAction::ToggleLeftPanel, self.show_left_panel))
            .with_fixed(self.button("Design", UiAction::ToggleRightPanel, self.show_right_panel))
            .with_fixed(self.button("Commands", UiAction::TogglePalette, self.show_palette))
            .with_fixed(self.button("Export PNG", UiAction::ExportSnapshot, true));
        NewWidget::new(SizedBox::new(row.prepare()).height(TOP_BAR_HEIGHT))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                Padding::from_vh(7.px(), 10.px()),
            ))
            .erased()
    }

    fn build_file_menu_anchor(&mut self) -> NewWidget<dyn Widget> {
        let mut menu = Flex::column()
            .with_fixed(label("FILE", 10.0, MUTED, true))
            .with_fixed_spacer(8.px());
        for (caption, action) in [
            ("New document", UiAction::New),
            ("Import NUIF…", UiAction::ImportNative),
            ("Save", UiAction::Save),
            ("Save as…", UiAction::SaveAs),
        ] {
            menu = menu
                .with_fixed(self.button(caption, action, false))
                .with_fixed_spacer(4.px());
        }
        menu = menu
            .with_fixed_spacer(5.px())
            .with_fixed(label("IMPORT PROFILE", 10.0, MUTED, true))
            .with_fixed_spacer(5.px());
        for format in ExternalFormat::ALL {
            menu = menu
                .with_fixed(self.button(
                    &format!("Import {}…", format.name()),
                    UiAction::ImportExternal(format),
                    false,
                ))
                .with_fixed_spacer(4.px());
        }
        menu = menu
            .with_fixed_spacer(5.px())
            .with_fixed(label("EXPORT", 10.0, MUTED, true))
            .with_fixed_spacer(5.px())
            .with_fixed(self.button("Export PNG…", UiAction::ExportSnapshot, false))
            .with_fixed_spacer(4.px());
        for format in ExternalFormat::ALL {
            menu = menu
                .with_fixed(self.button(
                    &format!("Export {}…", format.name()),
                    UiAction::ExportExternal(format),
                    false,
                ))
                .with_fixed_spacer(4.px());
        }
        menu = menu.with_fixed(self.button("Close menu", UiAction::ToggleFileMenu, false));
        let menu = NewWidget::new(SizedBox::new(menu.prepare()).width(Length::const_px(220.0)))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(ACCENT),
                BorderWidth::all(1.px()),
                CornerRadius::all(10.px()),
                Padding::from_vh(12.px(), 12.px()),
            ));
        let anchored = Flex::column().with_fixed_spacer(52.px()).with_fixed(
            Flex::row()
                .with_fixed_spacer(10.px())
                .with_fixed(menu)
                .prepare(),
        );
        NewWidget::new(anchored).erased()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the navigation modes stay together so identity wiring is auditable"
    )]
    fn build_left_panel(&mut self, selection: Option<EntityId>) -> NewWidget<dyn Widget> {
        let mut tabs = Flex::row();
        for (caption, panel) in [
            ("Pages", LeftPanel::Pages),
            ("Layers", LeftPanel::Layers),
            ("Components", LeftPanel::Components),
        ] {
            tabs = tabs.with_fixed(self.button(
                caption,
                UiAction::ChooseLeftPanel(panel),
                self.left_panel == panel,
            ));
        }
        let mut content = Flex::column()
            .with_fixed(tabs.prepare())
            .with_fixed_spacer(14.px());
        match self.left_panel {
            LeftPanel::Pages => {
                content = content
                    .with_fixed(label("SURFACES", 10.0, MUTED, true))
                    .with_fixed_spacer(7.px());
                let roots = self.editor.document().roots.clone();
                for (index, entity) in roots.into_iter().enumerate() {
                    let name = self.editor.document().entities[&entity]
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Page {}", index + 1));
                    let button =
                        self.button(&name, UiAction::Select(entity), selection == Some(entity));
                    let row = AuthorContainer::tree_item(button, entity, format!("Page {name}"))
                        .prepare();
                    self.entity_widgets.insert(entity, row.id());
                    content = content.with_fixed_spacer(4.px()).with_fixed(row);
                }
                content = content.with_fixed_spacer(10.px()).with_fixed(self.button(
                    "+ Add page",
                    UiAction::AddPage,
                    false,
                ));
            }
            LeftPanel::Layers => {
                content = content
                    .with_fixed(label("LAYER TREE", 10.0, MUTED, true))
                    .with_fixed_spacer(7.px());
                let rows = layer_rows(self.editor.document());
                if rows.is_empty() {
                    content = content.with_fixed(label("No layers", 12.0, MUTED, false));
                } else {
                    for (entity, name, depth) in rows {
                        let caption = format!(
                            "{}{}  {}",
                            "  ".repeat(depth),
                            kind_glyph(&self.editor.document().entities[&entity].kind),
                            name
                        );
                        let button = self.button(
                            &caption,
                            UiAction::Select(entity),
                            selection == Some(entity),
                        );
                        let row =
                            AuthorContainer::tree_item(button, entity, format!("Layer {name}"))
                                .prepare();
                        self.entity_widgets.insert(entity, row.id());
                        content = content.with_fixed_spacer(4.px()).with_fixed(row);
                    }
                }
            }
            LeftPanel::Components => {
                content = content
                    .with_fixed(label("LOCAL COMPONENTS", 10.0, MUTED, true))
                    .with_fixed_spacer(7.px());
                let components = self
                    .editor
                    .document()
                    .entities
                    .values()
                    .filter(|entity| matches!(entity.kind, EntityKind::Component))
                    .map(|entity| {
                        (
                            entity.id,
                            entity
                                .name
                                .clone()
                                .unwrap_or_else(|| "Component".to_owned()),
                        )
                    })
                    .collect::<Vec<_>>();
                if components.is_empty() {
                    content = content.with_fixed(label("No local components", 12.0, MUTED, false));
                }
                for (entity, name) in components {
                    let button = self.button(
                        &format!("◇ {name}"),
                        UiAction::Select(entity),
                        selection == Some(entity),
                    );
                    let row =
                        AuthorContainer::tree_item(button, entity, format!("Component {name}"))
                            .prepare();
                    self.entity_widgets.insert(entity, row.id());
                    content = content.with_fixed_spacer(4.px()).with_fixed(row);
                }
            }
        }

        let portal = Portal::new(content.prepare()).constrain_horizontal(true);
        NewWidget::new(SizedBox::new(portal.prepare()).width(LEFT_PANEL_WIDTH))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                Padding::from_vh(14.px(), 12.px()),
            ))
            .erased()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "inspector sections stay together so their specified order is auditable"
    )]
    fn build_right_panel(&mut self, selection: Option<EntityId>) -> NewWidget<dyn Widget> {
        let mut content = Flex::column()
            .with_fixed(label("Design", 12.0, TEXT, true))
            .with_fixed_spacer(14.px());
        if let Some(entity_id) = selection {
            let entity = self.editor.document().entities[&entity_id].clone();
            content = content
                .with_fixed(label("SELECTION", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label(
                    &format!("{}  {}", kind_glyph(&entity.kind), kind_name(&entity.kind)),
                    11.0,
                    MUTED,
                    false,
                ))
                .with_fixed_spacer(12.px())
                .with_fixed(label("Name", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Name(entity_id),
                    entity.name.as_deref().unwrap_or(""),
                    "Layer name",
                ))
                .with_fixed_spacer(10.px())
                .with_fixed(self.button("Duplicate", UiAction::DuplicateSelection, false))
                .with_fixed(self.button("Delete", UiAction::DeleteSelection, false))
                .with_fixed_spacer(18.px())
                .with_fixed(label("POSITION", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label("X · px", 11.0, TEXT, false))
                .with_fixed(self.inspector_input(
                    InspectorField::X(entity_id),
                    &entity.authored.position.x.to_string(),
                    "X",
                ))
                .with_fixed_spacer(5.px())
                .with_fixed(label("Y · px", 11.0, TEXT, false))
                .with_fixed(self.inspector_input(
                    InspectorField::Y(entity_id),
                    &entity.authored.position.y.to_string(),
                    "Y",
                ))
                .with_fixed_spacer(14.px())
                .with_fixed(label("SIZING", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label("Width · px, fill, auto or %", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Width(entity_id),
                    &size_value(&entity.authored.width),
                    "Width",
                ))
                .with_fixed_spacer(8.px())
                .with_fixed(label("Height · px", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Height(entity_id),
                    &size_value(&entity.authored.height),
                    "Height",
                ))
                .with_fixed_spacer(18.px());

            if matches!(
                entity.kind,
                EntityKind::Surface | EntityKind::Container | EntityKind::Component
            ) {
                content = content
                    .with_fixed(label("LAYOUT FAMILY", 10.0, MUTED, true))
                    .with_fixed_spacer(7.px());
                let mut families = Flex::row();
                for (caption, family) in [
                    ("Free", LayoutFamily::Freeform),
                    ("Stack", LayoutFamily::Stack),
                    ("Flex", LayoutFamily::Flex),
                ] {
                    families = families.with_fixed(self.button(
                        caption,
                        UiAction::SetLayoutFamily(family),
                        entity.authored.layout.family == family,
                    ));
                }
                content = content
                    .with_fixed(families.prepare())
                    .with_fixed_spacer(7.px());
                let mut directions = Flex::row();
                for (caption, direction) in [
                    ("Row", FlowDirection::Row),
                    ("Column", FlowDirection::Column),
                ] {
                    directions = directions.with_fixed(self.button(
                        caption,
                        UiAction::SetDirection(direction),
                        entity.authored.layout.direction == direction,
                    ));
                }
                content = content
                    .with_fixed(directions.prepare())
                    .with_fixed_spacer(7.px())
                    .with_fixed(label("Gap · px", 11.0, TEXT, false))
                    .with_fixed(self.inspector_input(
                        InspectorField::Gap(entity_id),
                        &entity.authored.layout.gap.to_string(),
                        "Gap",
                    ))
                    .with_fixed_spacer(7.px())
                    .with_fixed(label("Padding T / R / B / L · px", 11.0, TEXT, false))
                    .with_fixed(self.inspector_input(
                        InspectorField::PaddingTop(entity_id),
                        &entity.authored.layout.padding.top.to_string(),
                        "Top",
                    ))
                    .with_fixed(self.inspector_input(
                        InspectorField::PaddingRight(entity_id),
                        &entity.authored.layout.padding.right.to_string(),
                        "Right",
                    ))
                    .with_fixed(self.inspector_input(
                        InspectorField::PaddingBottom(entity_id),
                        &entity.authored.layout.padding.bottom.to_string(),
                        "Bottom",
                    ))
                    .with_fixed(self.inspector_input(
                        InspectorField::PaddingLeft(entity_id),
                        &entity.authored.layout.padding.left.to_string(),
                        "Left",
                    ))
                    .with_fixed_spacer(7.px());
                let mut alignments = Flex::row();
                for (caption, align) in [
                    ("Start", Align::Start),
                    ("Center", Align::Center),
                    ("End", Align::End),
                    ("Stretch", Align::Stretch),
                ] {
                    alignments = alignments.with_fixed(self.button(
                        caption,
                        UiAction::SetAlign(align),
                        entity.authored.layout.align == align,
                    ));
                }
                content = content
                    .with_fixed(alignments.prepare())
                    .with_fixed_spacer(18.px());
            }

            content = content
                .with_fixed(label("FILL", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label("Hex RGBA · use none to clear", 11.0, TEXT, false))
                .with_fixed(self.inspector_input(
                    InspectorField::Fill(entity_id),
                    &fill_value(entity.authored.fill),
                    "#RRGGBB",
                ))
                .with_fixed_spacer(18.px());

            if let Some(text) = &entity.authored.text {
                content = content
                    .with_fixed(label("TYPOGRAPHY", 10.0, MUTED, true))
                    .with_fixed_spacer(7.px())
                    .with_fixed(label("Content", 11.0, TEXT, false))
                    .with_fixed(self.inspector_input(
                        InspectorField::TextContent(entity_id),
                        &text.content,
                        "Text",
                    ))
                    .with_fixed_spacer(7.px())
                    .with_fixed(label("Font size · px", 11.0, TEXT, false))
                    .with_fixed(self.inspector_input(
                        InspectorField::FontSize(entity_id),
                        &text.size.to_string(),
                        "Size",
                    ))
                    .with_fixed_spacer(7.px())
                    .with_fixed(label("Line height · px", 11.0, TEXT, false))
                    .with_fixed(self.inspector_input(
                        InspectorField::LineHeight(entity_id),
                        &text.line_height.to_string(),
                        "Line height",
                    ))
                    .with_fixed_spacer(18.px());
            }

            let diagnostics = validate(self.editor.document());
            content = content
                .with_fixed(self.button("Apply changes", UiAction::ApplyInspector, true))
                .with_fixed_spacer(18.px())
                .with_fixed(label("DIAGNOSTICS", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label(
                    if diagnostics.is_empty() {
                        "No validation issues"
                    } else {
                        "Document has validation issues"
                    },
                    11.0,
                    if diagnostics.is_empty() { MUTED } else { TEXT },
                    false,
                ));
        } else {
            content = content
                .with_fixed(label("DOCUMENT", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label(
                    "Select a layer to edit its properties.",
                    11.0,
                    MUTED,
                    false,
                ))
                .with_fixed_spacer(18.px())
                .with_fixed(label("LOCAL VARIABLES", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label(
                    &format!("{} tokens", self.editor.document().tokens.len()),
                    11.0,
                    TEXT,
                    false,
                ));
        }
        let portal = Portal::new(content.prepare()).constrain_horizontal(true);
        NewWidget::new(SizedBox::new(portal.prepare()).width(RIGHT_PANEL_WIDTH))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                Padding::from_vh(14.px(), 14.px()),
            ))
            .erased()
    }

    fn build_toolbar(&mut self) -> NewWidget<dyn Widget> {
        let mut row = Flex::row();
        for (tool, shortcut, name) in Tool::ALL {
            let selected = self.tool == tool;
            let caption = if selected {
                format!("{shortcut} {name}")
            } else {
                shortcut.to_owned()
            };
            row = row.with_fixed(self.button(&caption, UiAction::ChooseTool(tool), selected));
        }
        NewWidget::new(SizedBox::new(row.prepare()).height(Length::const_px(48.0)))
            .with_props((
                Background::Color(PANEL_RAISED),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                CornerRadius::all(10.px()),
                Padding::from_vh(6.px(), 7.px()),
            ))
            .erased()
    }

    fn build_command_palette(&mut self) -> NewWidget<dyn Widget> {
        let mut commands = Flex::column()
            .with_fixed(label("COMMANDS", 11.0, MUTED, true))
            .with_fixed_spacer(8.px());
        for (caption, action) in [
            ("New document", UiAction::New),
            ("Import NUIF document…", UiAction::ImportNative),
            ("Save", UiAction::Save),
            ("Save as…", UiAction::SaveAs),
            ("Undo", UiAction::Undo),
            ("Redo", UiAction::Redo),
            ("Duplicate selection", UiAction::DuplicateSelection),
            ("Delete selection", UiAction::DeleteSelection),
            ("Export PNG snapshot…", UiAction::ExportSnapshot),
            (
                "Import SVG profile…",
                UiAction::ImportExternal(ExternalFormat::Svg),
            ),
            (
                "Import HTML/CSS profile…",
                UiAction::ImportExternal(ExternalFormat::HtmlCss),
            ),
            (
                "Import DTCG token profile…",
                UiAction::ImportExternal(ExternalFormat::Dtcg),
            ),
            (
                "Export SVG profile…",
                UiAction::ExportExternal(ExternalFormat::Svg),
            ),
            (
                "Export HTML/CSS profile…",
                UiAction::ExportExternal(ExternalFormat::HtmlCss),
            ),
            (
                "Export DTCG token profile…",
                UiAction::ExportExternal(ExternalFormat::Dtcg),
            ),
            ("Fit canvas", UiAction::ZoomFit),
            ("Hide interface", UiAction::ToggleUi),
            ("Close commands", UiAction::TogglePalette),
        ] {
            commands = commands
                .with_fixed(self.button(caption, action, false))
                .with_fixed_spacer(4.px());
        }
        NewWidget::new(SizedBox::new(commands.prepare()).width(Length::const_px(280.0)))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(ACCENT),
                BorderWidth::all(1.px()),
                CornerRadius::all(10.px()),
                Padding::from_vh(14.px(), 14.px()),
            ))
            .erased()
    }

    fn build_status(&self) -> NewWidget<dyn Widget> {
        let history = format!(
            "px · Grid {} · Rulers {}{}{}",
            if self.show_grid { "on" } else { "off" },
            if self.show_rulers { "on" } else { "off" },
            if self.editor.can_undo() {
                " · Undo available"
            } else {
                ""
            },
            if self.editor.can_redo() {
                " · Redo available"
            } else {
                ""
            }
        );
        let row = Flex::row()
            .with(label(&self.status, 10.0, MUTED, false), 1.0)
            .with_fixed(label(&history, 10.0, MUTED, false));
        NewWidget::new(SizedBox::new(row.prepare()).height(STATUS_HEIGHT))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                Padding::from_vh(5.px(), 10.px()),
            ))
            .erased()
    }

    fn button(&mut self, caption: &str, action: UiAction, selected: bool) -> NewWidget<Button> {
        let widget = Button::with_text(caption).prepare().with_props((
            Background::Color(if selected { ACCENT } else { PANEL_RAISED }),
            BorderColor::new(if selected { ACCENT } else { BORDER }),
            BorderWidth::all(1.px()),
            CornerRadius::all(6.px()),
            Padding::from_vh(5.px(), 8.px()),
        ));
        self.actions.insert(widget.id(), action);
        widget
    }

    fn inspector_input(
        &mut self,
        field: InspectorField,
        value: &str,
        placeholder: &str,
    ) -> NewWidget<AuthorContainer> {
        let value = self
            .drafts
            .get(&field)
            .map_or_else(|| value.to_owned(), Clone::clone);
        let input = TextInput::new(&value).with_placeholder(placeholder);
        self.text_fields.insert(input.area_pod().id(), field);
        let entity = match field {
            InspectorField::Name(entity)
            | InspectorField::Width(entity)
            | InspectorField::Height(entity)
            | InspectorField::X(entity)
            | InspectorField::Y(entity)
            | InspectorField::Gap(entity)
            | InspectorField::PaddingTop(entity)
            | InspectorField::PaddingRight(entity)
            | InspectorField::PaddingBottom(entity)
            | InspectorField::PaddingLeft(entity)
            | InspectorField::Fill(entity)
            | InspectorField::TextContent(entity)
            | InspectorField::FontSize(entity)
            | InspectorField::LineHeight(entity) => entity,
        };
        let (label, semantic_label, role) = match field {
            InspectorField::Name(_) => {
                ("Name control", "name", masonry::accesskit::Role::TextInput)
            }
            InspectorField::Width(_) => (
                "Width control",
                "width",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::Height(_) => (
                "Height control",
                "height",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::X(_) => ("X control", "x", masonry::accesskit::Role::SpinButton),
            InspectorField::Y(_) => ("Y control", "y", masonry::accesskit::Role::SpinButton),
            InspectorField::Gap(_) => ("Gap control", "gap", masonry::accesskit::Role::SpinButton),
            InspectorField::PaddingTop(_) => (
                "Top padding control",
                "padding_top",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::PaddingRight(_) => (
                "Right padding control",
                "padding_right",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::PaddingBottom(_) => (
                "Bottom padding control",
                "padding_bottom",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::PaddingLeft(_) => (
                "Left padding control",
                "padding_left",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::Fill(_) => {
                ("Fill control", "fill", masonry::accesskit::Role::TextInput)
            }
            InspectorField::TextContent(_) => (
                "Text content control",
                "text",
                masonry::accesskit::Role::TextInput,
            ),
            InspectorField::FontSize(_) => (
                "Font size control",
                "font_size",
                masonry::accesskit::Role::SpinButton,
            ),
            InspectorField::LineHeight(_) => (
                "Line height control",
                "line_height",
                masonry::accesskit::Role::SpinButton,
            ),
        };
        let control = AuthorContainer::value_control(
            input.prepare(),
            entity,
            label,
            semantic_label,
            role,
            value,
        )
        .prepare();
        self.control_widgets
            .insert((entity, semantic_label), control.id());
        control
    }

    fn refresh(&mut self, ctx: &mut DriverCtx<'_>) {
        let view = self.build_view();
        let root_widget_id = self
            .root_widget_id
            .expect("the editor root widget is registered before actions run");
        ctx.render_root(self.window_id)
            .edit_widget(root_widget_id, |mut root| {
                let mut root = root.downcast::<SizedBox>();
                SizedBox::set_child(&mut root, view);
            });
        let title = self.window_title();
        ctx.window(self.window_id).handle().set_title(&title);
    }

    fn window_title(&self) -> String {
        let file = self
            .document_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.nuif");
        format!("{}{} — NUIF", if self.dirty { "● " } else { "" }, file)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "command routing stays exhaustive and centralized"
    )]
    fn handle_ui_action(&mut self, action: UiAction) -> bool {
        match action {
            UiAction::New => {
                self.editor = EditorDriver::new(new_native_document(EntityId::new(1)));
                self.document_path = None;
                self.dirty = false;
                self.drafts.clear();
                "Created a new profile-zero document".clone_into(&mut self.status);
            }
            UiAction::ImportNative => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("NUIF document", &["nuif"])
                    .pick_file()
                else {
                    return false;
                };
                if let Err(error) = self.open_path(&path) {
                    self.status = format!("Import failed: {error}");
                }
            }
            UiAction::ImportExternal(format) => {
                if let Err(error) = self.import_external(format) {
                    self.status = format!("{} import failed: {error}", format.name());
                }
            }
            UiAction::Save => {
                if let Err(error) = self.save(false) {
                    self.status = format!("Save failed: {error}");
                }
            }
            UiAction::SaveAs => {
                if let Err(error) = self.save(true) {
                    self.status = format!("Save failed: {error}");
                }
            }
            UiAction::Undo => match self.editor.execute(EditorCommand::Undo) {
                Ok(_) => {
                    self.dirty = true;
                    "Undid the last document operation".clone_into(&mut self.status);
                }
                Err(error) => self.status = error.to_string(),
            },
            UiAction::Redo => match self.editor.execute(EditorCommand::Redo) {
                Ok(_) => {
                    self.dirty = true;
                    "Redid the last document operation".clone_into(&mut self.status);
                }
                Err(error) => self.status = error.to_string(),
            },
            UiAction::Select(entity) => {
                if let Err(error) = self.editor.execute(EditorCommand::Select { entity }) {
                    self.status = error.to_string();
                } else {
                    self.drafts.clear();
                    self.status = format!("Selected {entity}");
                }
            }
            UiAction::AddPage => {
                if let Err(error) = self.add_page() {
                    self.status = format!("Add page failed: {error}");
                }
            }
            UiAction::DeleteSelection => {
                if let Err(error) = self.delete_selection() {
                    self.status = format!("Delete failed: {error}");
                }
            }
            UiAction::DuplicateSelection => {
                if let Err(error) = self.duplicate_selection() {
                    self.status = format!("Duplicate failed: {error}");
                }
            }
            UiAction::ApplyInspector => self.apply_all_drafts(),
            UiAction::ExportSnapshot => {
                if let Err(error) = self.export_snapshot() {
                    self.status = format!("Snapshot failed: {error}");
                }
            }
            UiAction::ExportExternal(format) => {
                if let Err(error) = self.export_external(format) {
                    self.status = format!("{} export failed: {error}", format.name());
                }
            }
            UiAction::ChooseTool(tool) => {
                self.tool = tool;
                self.status = format!("{} tool selected", tool_name(tool));
            }
            UiAction::ChooseLeftPanel(panel) => self.left_panel = panel,
            UiAction::SetLayoutFamily(family) => {
                self.update_selected_layout(|layout| layout.family = family);
            }
            UiAction::SetDirection(direction) => {
                self.update_selected_layout(|layout| layout.direction = direction);
            }
            UiAction::SetAlign(align) => self.update_selected_layout(|layout| layout.align = align),
            UiAction::SetViewport(width) => {
                self.viewport_width = width;
                self.zoom = 1.0;
                self.status = format!("Evaluation viewport set to {width} × {VIEWPORT_HEIGHT} px");
            }
            UiAction::ZoomIn => {
                self.zoom = (self.zoom * 1.25).min(4.0);
                self.status = format!("Canvas zoom {}%", (self.zoom * 100.0).round());
            }
            UiAction::ZoomOut => {
                self.zoom = (self.zoom / 1.25).max(0.25);
                self.status = format!("Canvas zoom {}%", (self.zoom * 100.0).round());
            }
            UiAction::ZoomFit | UiAction::ZoomActual => self.zoom = 1.0,
            UiAction::ToggleLeftPanel => self.show_left_panel = !self.show_left_panel,
            UiAction::ToggleRightPanel => self.show_right_panel = !self.show_right_panel,
            UiAction::ToggleUi => {
                self.hide_ui = !self.hide_ui;
                self.show_palette = false;
                self.show_file_menu = false;
            }
            UiAction::TogglePalette => {
                self.show_palette = !self.show_palette;
                self.show_file_menu = false;
            }
            UiAction::ToggleFileMenu => {
                self.show_file_menu = !self.show_file_menu;
                self.show_palette = false;
            }
            UiAction::ToggleGrid => {
                self.show_grid = !self.show_grid;
                self.status = format!(
                    "Canvas grid {} · spacing follows the px ruler",
                    if self.show_grid { "shown" } else { "hidden" }
                );
            }
            UiAction::ToggleRulers => {
                self.show_rulers = !self.show_rulers;
                self.status = format!(
                    "Pixel rulers {}",
                    if self.show_rulers { "shown" } else { "hidden" }
                );
            }
        }
        if !matches!(
            action,
            UiAction::ToggleFileMenu
                | UiAction::TogglePalette
                | UiAction::ToggleGrid
                | UiAction::ToggleRulers
        ) {
            self.show_file_menu = false;
        }
        true
    }

    fn handle_canvas_action(&mut self, action: CanvasAction) {
        let CanvasAction::Activate {
            entity,
            document_position,
        } = action
        else {
            let CanvasAction::Shortcut(shortcut) = action else {
                unreachable!()
            };
            self.handle_shortcut(shortcut);
            return;
        };
        match self.tool {
            Tool::Move => {
                let result = match entity {
                    Some(entity) => self.editor.execute(EditorCommand::Select { entity }),
                    None => self.editor.execute(EditorCommand::ClearSelection),
                };
                match result {
                    Ok(_) => {
                        self.drafts.clear();
                        self.status = entity.map_or_else(
                            || "Cleared selection".to_owned(),
                            |entity| format!("Selected {entity}"),
                        );
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            Tool::Hand => {
                "Hand tool is active; scroll to navigate the canvas".clone_into(&mut self.status);
            }
            tool => {
                let Some((x, y)) = document_position else {
                    "Create inside the page bounds".clone_into(&mut self.status);
                    return;
                };
                if let Err(error) = self.insert_entity(tool, entity, x, y) {
                    self.status = format!("Insert failed: {error}");
                }
            }
        }
    }

    fn handle_shortcut(&mut self, shortcut: CanvasShortcut) {
        let action = match shortcut {
            CanvasShortcut::Tool(key) => match key.to_ascii_uppercase() {
                'V' => Some(UiAction::ChooseTool(Tool::Move)),
                'H' => Some(UiAction::ChooseTool(Tool::Hand)),
                'F' => Some(UiAction::ChooseTool(Tool::Frame)),
                'R' => Some(UiAction::ChooseTool(Tool::Rectangle)),
                'O' => Some(UiAction::ChooseTool(Tool::Ellipse)),
                'P' => Some(UiAction::ChooseTool(Tool::Pen)),
                'T' => Some(UiAction::ChooseTool(Tool::Text)),
                _ => None,
            },
            CanvasShortcut::Undo => Some(UiAction::Undo),
            CanvasShortcut::Redo => Some(UiAction::Redo),
            CanvasShortcut::Save => Some(UiAction::Save),
            CanvasShortcut::Duplicate => Some(UiAction::DuplicateSelection),
            CanvasShortcut::Delete => Some(UiAction::DeleteSelection),
            CanvasShortcut::Palette => Some(UiAction::TogglePalette),
            CanvasShortcut::Export => Some(UiAction::ExportSnapshot),
            CanvasShortcut::ZoomIn => Some(UiAction::ZoomIn),
            CanvasShortcut::ZoomOut => Some(UiAction::ZoomOut),
            CanvasShortcut::ZoomFit => Some(UiAction::ZoomFit),
            CanvasShortcut::ZoomActual => Some(UiAction::ZoomActual),
            CanvasShortcut::ToggleUi => Some(UiAction::ToggleUi),
            CanvasShortcut::ToggleGrid => Some(UiAction::ToggleGrid),
            CanvasShortcut::ToggleRulers => Some(UiAction::ToggleRulers),
        };
        if let Some(action) = action {
            self.handle_ui_action(action);
        }
    }

    fn handle_author_action(&mut self, action: AuthorAction) {
        match action {
            AuthorAction::Select { author_id } => {
                if let Err(error) = self
                    .editor
                    .dispatch_accessibility_action(AccessibilityAction::Select { author_id })
                {
                    self.status = error.to_string();
                } else {
                    self.drafts.clear();
                    self.status = format!("Selected {author_id} through accessibility");
                }
            }
            AuthorAction::SetValue {
                author_id,
                label,
                value,
            } => {
                if let Err(error) =
                    self.editor
                        .dispatch_accessibility_action(AccessibilityAction::SetValue {
                            author_id,
                            label: label.clone(),
                            value,
                        })
                {
                    self.status = format!("Accessibility edit failed: {error}");
                } else {
                    self.dirty = true;
                    self.drafts.clear();
                    self.status = format!("Updated {label} through accessibility");
                }
            }
        }
    }

    fn insert_entity(
        &mut self,
        tool: Tool,
        hit: Option<EntityId>,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        let (kind, name, width, height, fill) = match tool {
            Tool::Frame => (
                EntityKind::Container,
                "Frame",
                320.0,
                240.0,
                Some(color(0.96, 0.96, 0.97, 1.0)),
            ),
            Tool::Rectangle => (
                EntityKind::Shape(ShapeKind::Rectangle),
                "Rectangle",
                160.0,
                100.0,
                Some(color(0.39, 0.35, 0.93, 1.0)),
            ),
            Tool::Ellipse => (
                EntityKind::Shape(ShapeKind::Ellipse),
                "Ellipse",
                120.0,
                120.0,
                Some(color(0.18, 0.67, 0.53, 1.0)),
            ),
            Tool::Pen => (
                EntityKind::Shape(ShapeKind::Path),
                "Path",
                160.0,
                100.0,
                Some(color(0.93, 0.42, 0.48, 1.0)),
            ),
            Tool::Text => (EntityKind::Text, "Text", 180.0, 40.0, None),
            Tool::Move | Tool::Hand => return Ok(()),
        };
        let id = next_entity_id(self.editor.document())?;
        let mut entity = Entity::new(id, kind);
        entity.name = Some(name.to_owned());
        entity.authored.position = Point { x, y };
        entity.authored.width = SizeIntent::Fixed(width);
        entity.authored.height = SizeIntent::Fixed(height);
        entity.authored.fill = fill;
        if tool == Tool::Text {
            entity.authored.text = Some(TextContent {
                content: "Text".to_owned(),
                font: nuif_text::PINNED_FONT_NAME.to_owned(),
                font_sha256: nuif_text::PINNED_FONT_SHA256.to_owned(),
                size: 24.0,
                line_height: 28.0,
            });
            entity.authored.fill = Some(color(0.1, 0.1, 0.12, 1.0));
        }

        let parent = insertion_parent(self.editor.document(), hit);
        let siblings = parent.map_or(&self.editor.document().roots, |parent| {
            &self.editor.document().entities[&parent].children
        });
        let anchor = siblings
            .last()
            .copied()
            .map_or(Anchor::Start, Anchor::After);
        self.editor
            .dispatch_accessibility_action(AccessibilityAction::Insert {
                parent,
                anchor,
                entity: Box::new(entity),
            })
            .map_err(|error| error.to_string())?;
        self.editor
            .execute(EditorCommand::Select { entity: id })
            .map_err(|error| error.to_string())?;
        self.tool = Tool::Move;
        self.drafts.clear();
        self.dirty = true;
        self.status = format!("Inserted {name}");
        Ok(())
    }

    fn add_page(&mut self) -> Result<(), String> {
        let id = next_entity_id(self.editor.document())?;
        let mut surface = Entity::new(id, EntityKind::Surface);
        surface.name = Some(format!("Page {}", self.editor.document().roots.len() + 1));
        surface.authored.width = SizeIntent::Fill;
        surface.authored.height = SizeIntent::Fill;
        surface.authored.fill = Some(color(0.96, 0.96, 0.97, 1.0));
        let anchor = self
            .editor
            .document()
            .roots
            .last()
            .copied()
            .map_or(Anchor::Start, Anchor::After);
        self.editor
            .execute(EditorCommand::Apply {
                operations: vec![Operation::Insert {
                    parent: None,
                    anchor,
                    entity: Box::new(surface),
                }],
            })
            .map_err(|error| error.to_string())?;
        self.editor
            .execute(EditorCommand::Select { entity: id })
            .map_err(|error| error.to_string())?;
        self.left_panel = LeftPanel::Pages;
        self.dirty = true;
        "Added a page".clone_into(&mut self.status);
        Ok(())
    }

    fn delete_selection(&mut self) -> Result<(), String> {
        let Some(entity) = self.editor.selection().first().copied() else {
            "Nothing is selected".clone_into(&mut self.status);
            return Ok(());
        };
        self.editor
            .execute(EditorCommand::Remove { entity })
            .map_err(|error| error.to_string())?;
        self.drafts.clear();
        self.dirty = true;
        self.status = format!("Deleted {entity}");
        Ok(())
    }

    fn duplicate_selection(&mut self) -> Result<(), String> {
        let Some(root) = self.editor.selection().first().copied() else {
            "Nothing is selected".clone_into(&mut self.status);
            return Ok(());
        };
        let document = self.editor.document();
        let mut source_ids = Vec::new();
        collect_subtree(document, root, &mut source_ids);
        let first_id = next_entity_id(document)?.0;
        let id_map = source_ids
            .iter()
            .enumerate()
            .map(|(offset, source)| (*source, EntityId::new(first_id + offset as u128)))
            .collect::<BTreeMap<_, _>>();
        let root_parent = document.parent_of(root);
        let mut operations = Vec::with_capacity(source_ids.len());
        for source in &source_ids {
            let mut entity = document.entities[source].clone();
            entity.id = id_map[source];
            entity.children.clear();
            if *source == root {
                entity.authored.position.x += 16.0;
                entity.authored.position.y += 16.0;
                if let Some(name) = &mut entity.name {
                    name.push_str(" copy");
                }
            }
            if let EntityKind::Instance { component } = &mut entity.kind
                && let Some(mapped) = id_map.get(component)
            {
                *component = *mapped;
            }
            let source_parent = document.parent_of(*source);
            let parent = if *source == root {
                root_parent
            } else {
                source_parent.and_then(|parent| id_map.get(&parent).copied())
            };
            let source_siblings = source_parent.map_or(&document.roots, |parent| {
                &document.entities[&parent].children
            });
            let source_index = source_siblings
                .iter()
                .position(|id| id == source)
                .unwrap_or(0);
            let anchor = if *source == root {
                Anchor::After(root)
            } else {
                source_siblings[..source_index]
                    .iter()
                    .rev()
                    .find_map(|sibling| id_map.get(sibling).copied())
                    .map_or(Anchor::Start, Anchor::After)
            };
            operations.push(Operation::Insert {
                parent,
                anchor,
                entity: Box::new(entity),
            });
        }
        let duplicate = id_map[&root];
        self.editor
            .execute(EditorCommand::Apply { operations })
            .map_err(|error| error.to_string())?;
        self.editor
            .execute(EditorCommand::Select { entity: duplicate })
            .map_err(|error| error.to_string())?;
        self.drafts.clear();
        self.dirty = true;
        self.status = format!("Duplicated {root}");
        Ok(())
    }

    fn update_selected_layout(&mut self, update: impl FnOnce(&mut nuif_core::LayoutStyle)) {
        let Some(entity) = self.editor.selection().first().copied() else {
            "Select a container to edit layout".clone_into(&mut self.status);
            return;
        };
        let mut layout = self.editor.document().entities[&entity]
            .authored
            .layout
            .clone();
        update(&mut layout);
        match self.editor.execute(EditorCommand::SetLayout {
            entity,
            value: layout,
        }) {
            Ok(_) => {
                self.dirty = true;
                self.drafts.clear();
                "Updated layout".clone_into(&mut self.status);
            }
            Err(error) => self.status = format!("Layout edit failed: {error}"),
        }
    }

    fn handle_text_action(&mut self, widget_id: WidgetId, action: TextAction) -> bool {
        let Some(field) = self.text_fields.get(&widget_id).copied() else {
            return false;
        };
        match action {
            TextAction::Changed(value) => {
                self.drafts.insert(field, value);
                false
            }
            TextAction::Entered(value) => {
                self.drafts.insert(field, value);
                self.apply_field(&field);
                true
            }
            TextAction::Cancelled => {
                self.drafts.remove(&field);
                true
            }
        }
    }

    fn apply_all_drafts(&mut self) {
        let fields = self.drafts.keys().copied().collect::<Vec<_>>();
        if fields.is_empty() {
            "No changed properties to apply".clone_into(&mut self.status);
            return;
        }
        self.apply_fields(&fields);
    }

    fn apply_field(&mut self, field: &InspectorField) -> bool {
        self.apply_fields(&[*field])
    }

    #[expect(
        clippy::too_many_lines,
        reason = "atomic inspector lowering keeps parsing and grouping in one audit path"
    )]
    fn apply_fields(&mut self, fields: &[InspectorField]) -> bool {
        let Some(entity_id) = fields.first().map(inspector_entity) else {
            return true;
        };
        let Some(original) = self.editor.document().entities.get(&entity_id).cloned() else {
            self.status = format!("Property edit failed: entity {entity_id} no longer exists");
            return false;
        };
        let mut edited = original.clone();
        let mut rename = None;
        let mut width_changed = false;
        let mut height_changed = false;
        let mut position_changed = false;
        let mut layout_changed = false;
        let mut fill_changed = false;
        let mut text_changed = false;
        for field in fields {
            let Some(value) = self.drafts.get(field) else {
                continue;
            };
            let parsed = match *field {
                InspectorField::Name(_) => {
                    rename = Some(value.clone());
                    Ok(())
                }
                InspectorField::Width(_) => parse_size_intent(value).map(|value| {
                    edited.authored.width = value;
                    width_changed = true;
                }),
                InspectorField::Height(_) => parse_size_intent(value).map(|value| {
                    edited.authored.height = value;
                    height_changed = true;
                }),
                InspectorField::X(_) => parse_number(value).map(|value| {
                    edited.authored.position.x = value;
                    position_changed = true;
                }),
                InspectorField::Y(_) => parse_number(value).map(|value| {
                    edited.authored.position.y = value;
                    position_changed = true;
                }),
                InspectorField::Gap(_) => parse_number(value).map(|value| {
                    edited.authored.layout.gap = value;
                    layout_changed = true;
                }),
                InspectorField::PaddingTop(_) => parse_number(value).map(|value| {
                    edited.authored.layout.padding.top = value;
                    layout_changed = true;
                }),
                InspectorField::PaddingRight(_) => parse_number(value).map(|value| {
                    edited.authored.layout.padding.right = value;
                    layout_changed = true;
                }),
                InspectorField::PaddingBottom(_) => parse_number(value).map(|value| {
                    edited.authored.layout.padding.bottom = value;
                    layout_changed = true;
                }),
                InspectorField::PaddingLeft(_) => parse_number(value).map(|value| {
                    edited.authored.layout.padding.left = value;
                    layout_changed = true;
                }),
                InspectorField::Fill(_) => parse_fill(value).map(|value| {
                    edited.authored.fill = value;
                    fill_changed = true;
                }),
                InspectorField::TextContent(_) => edited.authored.text.as_mut().map_or_else(
                    || Err("selected entity has no text content".to_owned()),
                    |text| {
                        value.clone_into(&mut text.content);
                        text_changed = true;
                        Ok(())
                    },
                ),
                InspectorField::FontSize(_) => parse_number(value).and_then(|value| {
                    let text = edited
                        .authored
                        .text
                        .as_mut()
                        .ok_or_else(|| "selected entity has no text content".to_owned())?;
                    text.size = value;
                    text_changed = true;
                    Ok(())
                }),
                InspectorField::LineHeight(_) => parse_number(value).and_then(|value| {
                    let text = edited
                        .authored
                        .text
                        .as_mut()
                        .ok_or_else(|| "selected entity has no text content".to_owned())?;
                    text.line_height = value;
                    text_changed = true;
                    Ok(())
                }),
            };
            if let Err(error) = parsed {
                self.status = format!("Property edit failed: {error}");
                return false;
            }
        }
        let mut operations = Vec::new();
        if let Some(name) = rename {
            operations.push(Operation::Rename {
                entity: entity_id,
                name: Some(name),
            });
        }
        if width_changed {
            operations.push(Operation::SetSize {
                entity: entity_id,
                axis: ProtocolAxis::Horizontal,
                value: edited.authored.width.clone(),
            });
        }
        if height_changed {
            operations.push(Operation::SetSize {
                entity: entity_id,
                axis: ProtocolAxis::Vertical,
                value: edited.authored.height.clone(),
            });
        }
        if position_changed {
            operations.push(Operation::SetPosition {
                entity: entity_id,
                value: edited.authored.position,
            });
        }
        if layout_changed {
            operations.push(Operation::SetLayout {
                entity: entity_id,
                value: edited.authored.layout.clone(),
            });
        }
        if fill_changed {
            operations.push(Operation::SetFill {
                entity: entity_id,
                value: edited.authored.fill,
            });
        }
        if text_changed {
            operations.push(Operation::SetText {
                entity: entity_id,
                value: edited.authored.text.clone(),
            });
        }
        if operations.is_empty() {
            return true;
        }
        match self.editor.execute(EditorCommand::Apply { operations }) {
            Ok(_) => {
                for field in fields {
                    self.drafts.remove(field);
                }
                self.dirty = true;
                self.status = if fields.len() == 1 {
                    "Updated property".to_owned()
                } else {
                    format!("Updated {} properties atomically", fields.len())
                };
                true
            }
            Err(error) => {
                self.status = format!("Property edit failed: {error}");
                false
            }
        }
    }

    fn open_path(&mut self, path: &Path) -> Result<(), String> {
        let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
        let bytes =
            read_bounded_stream(&mut file, MAX_INPUT_BYTES).map_err(|error| error.to_string())?;
        let document = CanonicalText
            .decode(&bytes)
            .map_err(|error| error.to_string())?;
        self.editor = EditorDriver::new(document);
        self.document_path = Some(path.to_path_buf());
        self.dirty = false;
        self.drafts.clear();
        self.status = format!("Opened {}", path.display());
        Ok(())
    }

    fn import_external(&mut self, format: ExternalFormat) -> Result<(), String> {
        let (filter_name, extensions) = format.filter();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, extensions)
            .pick_file()
        else {
            return Ok(());
        };
        let source = read_external_source(&path, format.source_limit())?;
        let imported = import_external_source(format, &source)?;
        let summary = adapter_report_summary(format, &imported.retentive.report);
        let result = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title(format!("Import {} profile", format.name()))
            .set_description(format!(
                "{summary}\n\nOpen this import as a new unsaved NUIF document?"
            ))
            .set_buttons(rfd::MessageButtons::OkCancelCustom(
                "Import".to_owned(),
                "Cancel".to_owned(),
            ))
            .show();
        let accepted = match result {
            rfd::MessageDialogResult::Ok => true,
            rfd::MessageDialogResult::Custom(value) => value == "Import",
            _ => false,
        };
        if !accepted {
            self.status = format!("Cancelled {} import", format.name());
            return Ok(());
        }

        self.editor = EditorDriver::new(imported.document);
        self.document_path = None;
        self.dirty = true;
        self.drafts.clear();
        self.status = format!("Imported {} · {summary}", format.name());
        Ok(())
    }

    fn save(&mut self, force_dialog: bool) -> Result<(), String> {
        let path = if force_dialog || self.document_path.is_none() {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("NUIF document", &["nuif"])
                .set_file_name("Untitled.nuif")
                .save_file()
            else {
                return Ok(());
            };
            path
        } else {
            self.document_path.clone().unwrap()
        };
        let bytes = CanonicalText
            .encode(self.editor.document())
            .map_err(|error| error.to_string())?;
        fs::write(&path, bytes).map_err(|error| error.to_string())?;
        self.document_path = Some(path.clone());
        self.dirty = false;
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    fn export_snapshot(&mut self) -> Result<(), String> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("nuif-snapshot.png")
            .save_file()
        else {
            return Ok(());
        };
        let event = self
            .editor
            .execute(EditorCommand::Snapshot {
                width: VIEWPORT_WIDTH,
                height: VIEWPORT_HEIGHT,
            })
            .map_err(|error| error.to_string())?;
        let EditorEvent::Snapshot { snapshot } = event else {
            unreachable!("snapshot command returns snapshot event");
        };
        fs::write(&path, &snapshot.raster.png).map_err(|error| error.to_string())?;
        self.status = format!("Exported {}", path.display());
        Ok(())
    }

    fn export_external(&mut self, format: ExternalFormat) -> Result<(), String> {
        let exported = export_external_document(format, self.editor.document())?;
        let (filter_name, extensions) = format.filter();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, extensions)
            .set_file_name(format.default_file_name())
            .save_file()
        else {
            return Ok(());
        };
        let report_path = adapter_report_path(&path)?;
        let report =
            serde_json::to_vec_pretty(&exported.report).map_err(|error| error.to_string())?;
        fs::write(&report_path, report).map_err(|error| error.to_string())?;
        fs::write(&path, exported.source.as_bytes()).map_err(|error| error.to_string())?;
        self.status = format!(
            "Exported {} and fidelity report {}",
            path.display(),
            report_path.display()
        );
        Ok(())
    }
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_>,
        widget_id: WidgetId,
        action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown editor window");
        let refresh = if action.is::<ButtonPress>() {
            self.actions
                .get(&widget_id)
                .copied()
                .is_some_and(|action| self.handle_ui_action(action))
        } else if action.is::<CanvasAction>() {
            self.handle_canvas_action(*action.downcast::<CanvasAction>().unwrap());
            true
        } else if action.is::<AuthorAction>() {
            self.handle_author_action(*action.downcast::<AuthorAction>().unwrap());
            true
        } else if action.is::<TextAction>() {
            self.handle_text_action(widget_id, *action.downcast::<TextAction>().unwrap())
        } else {
            false
        };
        if refresh {
            self.refresh(ctx);
        }
    }
}

fn read_external_source(path: &Path, limit: usize) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let bytes = read_bounded_stream(&mut file, limit).map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| format!("source is not valid UTF-8: {error}"))
}

fn import_external_source(format: ExternalFormat, source: &str) -> Result<ImportedSource, String> {
    match format {
        ExternalFormat::Svg => nuif_svg::import_source(source).map_err(|error| error.to_string()),
        ExternalFormat::HtmlCss => {
            nuif_html::import_source(source).map_err(|error| error.to_string())
        }
        ExternalFormat::Dtcg => nuif_dtcg::import_source(source).map_err(|error| error.to_string()),
    }
}

fn export_external_document(
    format: ExternalFormat,
    document: &Document,
) -> Result<ExportedSource, String> {
    match format {
        ExternalFormat::Svg => {
            nuif_svg::export_document(document).map_err(|error| error.to_string())
        }
        ExternalFormat::HtmlCss => {
            nuif_html::export_document(document).map_err(|error| error.to_string())
        }
        ExternalFormat::Dtcg => {
            nuif_dtcg::export_document(document).map_err(|error| error.to_string())
        }
    }
}

fn adapter_report_summary(format: ExternalFormat, report: &AdapterReport) -> String {
    let mut lossless = 0;
    let mut representable = 0;
    let mut approximated = 0;
    let mut preserved = 0;
    let mut unsupported = 0;
    for entry in &report.fidelity {
        match entry.status {
            Fidelity::Lossless => lossless += 1,
            Fidelity::Representable => representable += 1,
            Fidelity::Approximated { .. } => approximated += 1,
            Fidelity::PreservedUnrenderable { .. } => preserved += 1,
            Fidelity::Unsupported { .. } => unsupported += 1,
        }
    }
    format!(
        "{} · lossless {lossless} · representable {representable} · approximated {approximated} · preserved {preserved} · unsupported {unsupported} · {} correspondences",
        format.name(),
        report.correspondences.len()
    )
}

fn adapter_report_path(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "export file name is not valid UTF-8".to_owned())?;
    Ok(path.with_file_name(format!("{name}.report.json")))
}

struct NativeOptions {
    document: Document,
    path: Option<PathBuf>,
}

fn parse_native_options() -> Result<Option<NativeOptions>, String> {
    let mut args = env::args().skip(1);
    let mut path = None;
    let mut new_document = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!("usage: nuif-editor [--document <nuif> | --new-document <id> | <nuif>]");
                return Ok(None);
            }
            "--version" | "-V" => {
                println!("NUIF Editor {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--document" => {
                path = Some(
                    args.next()
                        .ok_or_else(|| "--document requires a path".to_owned())?
                        .into(),
                );
            }
            "--new-document" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--new-document requires an identifier".to_owned())?;
                new_document = Some(
                    value
                        .parse::<EntityId>()
                        .map_err(|error| format!("invalid document identifier: {error}"))?,
                );
            }
            value if !value.starts_with('-') && path.is_none() => path = Some(value.into()),
            unknown => return Err(format!("unknown native-editor argument {unknown:?}")),
        }
    }
    if path.is_some() && new_document.is_some() {
        return Err("--document and --new-document are mutually exclusive".to_owned());
    }
    if let Some(path) = path {
        let mut file = fs::File::open(&path).map_err(|error| error.to_string())?;
        let bytes =
            read_bounded_stream(&mut file, MAX_INPUT_BYTES).map_err(|error| error.to_string())?;
        let document = CanonicalText
            .decode(&bytes)
            .map_err(|error| error.to_string())?;
        Ok(Some(NativeOptions {
            document,
            path: Some(path),
        }))
    } else {
        Ok(Some(NativeOptions {
            document: new_native_document(new_document.unwrap_or(EntityId::new(1))),
            path: None,
        }))
    }
}

/// Starts the native editor window.
///
/// # Errors
///
/// Returns an error when launch arguments, document input, or the platform
/// event loop are invalid.
pub fn run() -> Result<(), String> {
    let Some(options) = parse_native_options()? else {
        return Ok(());
    };
    let window_id = WindowId::next();
    let mut driver = Driver::new(window_id, options.document, options.path);
    let initial_view = driver.build_view();
    let root = SizedBox::new(initial_view).prepare();
    driver.root_widget_id = Some(root.id());
    let window_size = LogicalSize::new(1280.0, 800.0);
    let window_attributes = Window::default_attributes()
        .with_title(driver.window_title())
        .with_resizable(true)
        .with_inner_size(window_size)
        .with_min_inner_size(LogicalSize::new(900.0, 600.0));

    masonry_winit::app::run(
        vec![
            NewWindow::new_with_id(window_id, window_attributes, root.erased())
                .with_base_color(PANEL),
        ],
        driver,
        default_property_set(),
    )
    .map_err(|error| error.to_string())
}

fn label(text: &str, size: f32, color: UiColor, bold: bool) -> NewWidget<Label> {
    let mut label = Label::new(text).with_style(StyleProperty::FontSize(size));
    if bold {
        label = label.with_style(StyleProperty::FontWeight(FontWeight::SEMI_BOLD));
    }
    label.prepare().with_props(ContentColor::new(color))
}

fn new_native_document(id: EntityId) -> Document {
    let mut document = Document::empty(id);
    let surface_id = EntityId::new(id.0.saturating_add(1));
    let mut surface = Entity::new(surface_id, EntityKind::Surface);
    surface.name = Some("Page 1".to_owned());
    surface.authored.width = SizeIntent::Fill;
    surface.authored.height = SizeIntent::Fill;
    surface.authored.fill = Some(color(0.96, 0.96, 0.97, 1.0));
    document.roots.push(surface_id);
    document.entities.insert(surface_id, surface);
    document
}

fn layer_rows(document: &Document) -> Vec<(EntityId, String, usize)> {
    fn visit(
        document: &Document,
        entity: EntityId,
        depth: usize,
        rows: &mut Vec<(EntityId, String, usize)>,
    ) {
        let Some(value) = document.entities.get(&entity) else {
            return;
        };
        rows.push((
            entity,
            value.name.clone().unwrap_or_else(|| kind_name(&value.kind)),
            depth,
        ));
        for child in &value.children {
            visit(document, *child, depth + 1, rows);
        }
    }
    let mut rows = Vec::new();
    for root in &document.roots {
        visit(document, *root, 0, &mut rows);
    }
    rows
}

fn kind_name(kind: &EntityKind) -> String {
    match kind {
        EntityKind::Surface => "Surface",
        EntityKind::Container => "Frame",
        EntityKind::Shape(ShapeKind::Rectangle) => "Rectangle",
        EntityKind::Shape(ShapeKind::Ellipse) => "Ellipse",
        EntityKind::Shape(ShapeKind::Path) => "Path",
        EntityKind::Text => "Text",
        EntityKind::Image => "Image",
        EntityKind::Component => "Component",
        EntityKind::Instance { .. } => "Instance",
        EntityKind::Unknown(_) => "Unknown",
    }
    .to_owned()
}

fn kind_glyph(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Surface => "S",
        EntityKind::Container => "F",
        EntityKind::Shape(ShapeKind::Rectangle) => "R",
        EntityKind::Shape(ShapeKind::Ellipse) => "O",
        EntityKind::Shape(ShapeKind::Path) => "P",
        EntityKind::Text => "T",
        EntityKind::Image => "I",
        EntityKind::Component => "C",
        EntityKind::Instance { .. } => "i",
        EntityKind::Unknown(_) => "?",
    }
}

fn tool_name(tool: Tool) -> &'static str {
    Tool::ALL
        .iter()
        .find_map(|(candidate, _, name)| (*candidate == tool).then_some(*name))
        .unwrap()
}

fn size_value(value: &SizeIntent) -> String {
    match value {
        SizeIntent::Fixed(value) => value.to_string(),
        SizeIntent::Auto => "auto".to_owned(),
        SizeIntent::Fill => "fill".to_owned(),
        SizeIntent::Intrinsic => "intrinsic".to_owned(),
        SizeIntent::Percentage(value) => format!("{value}%"),
        SizeIntent::MinContent => "min-content".to_owned(),
        SizeIntent::MaxContent => "max-content".to_owned(),
        SizeIntent::FitContent(value) => format!("fit-content({value})"),
    }
}

fn parse_size_intent(value: &str) -> Result<SizeIntent, String> {
    let value = value.trim();
    match value {
        "auto" => Ok(SizeIntent::Auto),
        "fill" => Ok(SizeIntent::Fill),
        "intrinsic" => Ok(SizeIntent::Intrinsic),
        "min-content" => Ok(SizeIntent::MinContent),
        "max-content" => Ok(SizeIntent::MaxContent),
        _ if value.ends_with('%') => value
            .strip_suffix('%')
            .map_or_else(
                || Err(format!("invalid percentage {value:?}")),
                parse_number,
            )
            .map(SizeIntent::Percentage)
            .map_err(|_| format!("invalid percentage {value:?}")),
        _ if value.starts_with("fit-content(") && value.ends_with(')') => value
            .strip_prefix("fit-content(")
            .and_then(|value| value.strip_suffix(')'))
            .map_or_else(
                || Err(format!("invalid fit-content value {value:?}")),
                parse_number,
            )
            .map(SizeIntent::FitContent)
            .map_err(|_| format!("invalid fit-content value {value:?}")),
        _ => parse_number(value).map(SizeIntent::Fixed),
    }
}

fn parse_number(value: &str) -> Result<f64, String> {
    let number = value
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid number {value:?}"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err("numbers must be finite".to_owned())
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "channels are clamped to the exact u8 range before conversion"
)]
fn fill_value(fill: Option<Color>) -> String {
    fill.map_or_else(
        || "none".to_owned(),
        |fill| {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                (fill.red.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.green.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
                (fill.alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
            )
        },
    )
}

fn parse_fill(value: &str) -> Result<Option<Color>, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("fill must be #RRGGBB, #RRGGBBAA or none".to_owned());
    }
    let channel = |start| {
        u8::from_str_radix(&hex[start..start + 2], 16)
            .map(f32::from)
            .map(|value| value / 255.0)
            .map_err(|_| "invalid fill colour".to_owned())
    };
    Ok(Some(color(
        channel(0)?,
        channel(2)?,
        channel(4)?,
        if hex.len() == 8 { channel(6)? } else { 1.0 },
    )))
}

fn inspector_entity(field: &InspectorField) -> EntityId {
    match *field {
        InspectorField::Name(entity)
        | InspectorField::Width(entity)
        | InspectorField::Height(entity)
        | InspectorField::X(entity)
        | InspectorField::Y(entity)
        | InspectorField::Gap(entity)
        | InspectorField::PaddingTop(entity)
        | InspectorField::PaddingRight(entity)
        | InspectorField::PaddingBottom(entity)
        | InspectorField::PaddingLeft(entity)
        | InspectorField::Fill(entity)
        | InspectorField::TextContent(entity)
        | InspectorField::FontSize(entity)
        | InspectorField::LineHeight(entity) => entity,
    }
}

fn collect_subtree(document: &Document, entity: EntityId, output: &mut Vec<EntityId>) {
    output.push(entity);
    if let Some(value) = document.entities.get(&entity) {
        for child in &value.children {
            collect_subtree(document, *child, output);
        }
    }
}

fn color(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
    Color {
        space: nuif_core::ColorSpace::Srgb,
        red,
        green,
        blue,
        alpha,
    }
}

fn next_entity_id(document: &Document) -> Result<EntityId, String> {
    document
        .entities
        .keys()
        .map(|entity| entity.0)
        .chain(std::iter::once(document.id.0))
        .max()
        .and_then(|value| value.checked_add(1))
        .map(EntityId::new)
        .ok_or_else(|| "entity identifier space is exhausted".to_owned())
}

fn insertion_parent(document: &Document, hit: Option<EntityId>) -> Option<EntityId> {
    let hit = hit?;
    let entity = document.entities.get(&hit)?;
    if matches!(
        entity.kind,
        EntityKind::Surface | EntityKind::Container | EntityKind::Component
    ) {
        Some(hit)
    } else {
        document.parent_of(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::accesskit::{Action, ActionData, ActionRequest, TreeId};
    use masonry_testing::{TestHarness, TestHarnessParams};

    fn harness_from_driver(driver: &mut Driver) -> TestHarness<SizedBox> {
        let root = SizedBox::new(driver.build_view()).prepare();
        driver.root_widget_id = Some(root.id());
        let params = TestHarnessParams::default().with_size((1280, 800));
        TestHarness::create_with(default_property_set(), root, params)
    }

    fn harness() -> (TestHarness<SizedBox>, Driver) {
        let window_id = WindowId::next();
        let mut driver = Driver::new(window_id, new_native_document(EntityId::new(1)), None);
        let harness = harness_from_driver(&mut driver);
        (harness, driver)
    }

    #[test]
    fn shell_renders_and_exposes_entity_identity() {
        let (mut harness, driver) = harness();
        let image = harness.render();
        assert_eq!((image.width(), image.height()), (1280, 800));
        let surface = driver.editor.document().roots[0].to_string();
        let surface_widget = driver.entity_widgets[&driver.editor.document().roots[0]];
        assert!(
            harness
                .access_node(surface_widget)
                .is_some_and(|node| node.author_id() == Some(surface.as_str()))
        );
    }

    #[test]
    fn shell_defaults_to_pixel_grid_and_rulers() {
        let (_, mut driver) = harness();
        assert!(driver.show_grid);
        assert!(driver.show_rulers);
        assert!(driver.status.contains("px"));

        assert!(driver.handle_ui_action(UiAction::ToggleGrid));
        assert!(driver.handle_ui_action(UiAction::ToggleRulers));
        assert!(!driver.show_grid);
        assert!(!driver.show_rulers);
    }

    #[test]
    fn file_menu_exposes_native_and_profile_adapter_routes() {
        let (_, mut driver) = harness();
        assert!(driver.handle_ui_action(UiAction::ToggleFileMenu));
        let _ = driver.build_view();

        assert!(driver.show_file_menu);
        assert!(
            driver
                .actions
                .values()
                .any(|action| matches!(action, UiAction::ImportNative))
        );
        assert!(
            driver
                .actions
                .values()
                .any(|action| matches!(action, UiAction::ExportSnapshot))
        );
        assert!(
            driver
                .actions
                .values()
                .any(|action| matches!(action, UiAction::SaveAs))
        );
        for format in ExternalFormat::ALL {
            assert!(
                driver
                    .actions
                    .values()
                    .any(|action| *action == UiAction::ImportExternal(format))
            );
            assert!(
                driver
                    .actions
                    .values()
                    .any(|action| *action == UiAction::ExportExternal(format))
            );
        }
    }

    #[test]
    fn editor_adapter_routes_round_trip_declared_profiles() {
        let fixtures = [
            (ExternalFormat::Svg, nuif_svg::profile_fixture()),
            (ExternalFormat::HtmlCss, nuif_html::profile_fixture()),
            (ExternalFormat::Dtcg, nuif_dtcg::profile_fixture()),
        ];
        for (format, document) in fixtures {
            let exported = export_external_document(format, &document).unwrap();
            let imported = import_external_source(format, &exported.source).unwrap();
            assert_eq!(imported.document, document);
            assert!(exported.report.is_lossless());
            assert!(adapter_report_summary(format, &exported.report).contains("unsupported 0"));
        }
    }

    #[test]
    fn adapter_report_is_a_sibling_of_the_export() {
        assert_eq!(
            adapter_report_path(Path::new("fixtures/card.svg")).unwrap(),
            PathBuf::from("fixtures/card.svg.report.json")
        );
    }

    #[test]
    fn canvas_insert_and_inspector_edit_use_semantic_operations() {
        let (_, mut driver) = harness();
        let surface = driver.editor.document().roots[0];
        driver
            .insert_entity(Tool::Rectangle, Some(surface), 40.0, 60.0)
            .unwrap();
        let selected = driver.editor.selection()[0];
        driver
            .drafts
            .insert(InspectorField::Width(selected), "240".to_owned());
        assert!(driver.apply_field(&InspectorField::Width(selected)));
        assert_eq!(
            driver.editor.document().entities[&selected].authored.width,
            SizeIntent::Fixed(240.0)
        );
        assert_eq!(driver.editor.operation_log().len(), 2);
    }

    #[test]
    fn accesskit_nodes_drive_selection_and_inspector_edits() {
        let (mut harness, mut driver) = harness();
        let surface = driver.editor.document().roots[0];
        let surface_widget = driver.entity_widgets[&surface];
        let _ = harness.redraw();
        harness.process_access_event(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: surface_widget.to_raw().into(),
            data: None,
        });
        let (action, source) = harness.pop_action::<AuthorAction>().unwrap();
        assert_eq!(source, surface_widget);
        driver.handle_author_action(action);
        assert_eq!(driver.editor.selection(), &[surface]);

        let mut harness = harness_from_driver(&mut driver);
        let width_widget = driver.control_widgets[&(surface, "width")];
        let _ = harness.redraw();
        harness.process_access_event(ActionRequest {
            action: Action::SetValue,
            target_tree: TreeId::ROOT,
            target_node: width_widget.to_raw().into(),
            data: Some(ActionData::Value("320".into())),
        });
        let (action, source) = harness.pop_action::<AuthorAction>().unwrap();
        assert_eq!(source, width_widget);
        driver.handle_author_action(action);
        assert_eq!(
            driver.editor.document().entities[&surface].authored.width,
            SizeIntent::Fixed(320.0)
        );
    }

    #[test]
    fn inspector_applies_related_drafts_as_one_transaction() {
        let (_, mut driver) = harness();
        let surface = driver.editor.document().roots[0];
        driver
            .editor
            .execute(EditorCommand::Select { entity: surface })
            .unwrap();
        driver
            .drafts
            .insert(InspectorField::X(surface), "24".to_owned());
        driver
            .drafts
            .insert(InspectorField::Y(surface), "36".to_owned());
        driver
            .drafts
            .insert(InspectorField::Gap(surface), "12".to_owned());
        driver
            .drafts
            .insert(InspectorField::Fill(surface), "#336699CC".to_owned());
        driver.apply_all_drafts();

        let entity = &driver.editor.document().entities[&surface];
        assert_eq!(entity.authored.position, Point { x: 24.0, y: 36.0 });
        assert!((entity.authored.layout.gap - 12.0).abs() < f64::EPSILON);
        assert_eq!(fill_value(entity.authored.fill), "#336699CC");
        assert_eq!(driver.editor.operation_log().len(), 1);
        assert_eq!(
            driver.editor.operation_log()[0].transactions[0]
                .operations
                .len(),
            3
        );
    }

    #[test]
    fn duplicate_selection_clones_a_complete_subtree() {
        let (_, mut driver) = harness();
        let surface = driver.editor.document().roots[0];
        driver
            .insert_entity(Tool::Frame, Some(surface), 20.0, 20.0)
            .unwrap();
        let frame = driver.editor.selection()[0];
        driver
            .insert_entity(Tool::Rectangle, Some(frame), 8.0, 8.0)
            .unwrap();
        driver
            .editor
            .execute(EditorCommand::Select { entity: frame })
            .unwrap();
        driver.duplicate_selection().unwrap();

        let duplicate = driver.editor.selection()[0];
        assert_ne!(duplicate, frame);
        assert_eq!(
            driver.editor.document().entities[&duplicate].children.len(),
            1
        );
        assert_eq!(driver.editor.document().entities.len(), 5);
        assert_eq!(driver.editor.operation_log().len(), 3);
    }

    #[test]
    fn inspector_rejects_nonfinite_size_forms() {
        for value in ["NaN", "inf", "NaN%", "fit-content(inf)"] {
            assert!(parse_size_intent(value).is_err(), "accepted {value}");
        }
        assert_eq!(
            parse_size_intent("25%").unwrap(),
            SizeIntent::Percentage(25.0)
        );
        assert_eq!(
            parse_size_intent("fit-content(320)").unwrap(),
            SizeIntent::FitContent(320.0)
        );
    }
}
