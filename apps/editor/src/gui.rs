//! Native Masonry application shell.

mod widgets;

use self::widgets::{AuthorContainer, CanvasAction, DocumentCanvas};
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
use nuif_codec::{
    CanonicalText, Decoder, Encoder, MAX_INPUT_BYTES, read_bounded as read_bounded_stream,
};
use nuif_core::{
    Color, Document, Entity, EntityId, EntityKind, Point, ShapeKind, SizeIntent, TextContent,
};
use nuif_editor::{AccessibilityAction, EditorCommand, EditorDriver, EditorEvent};
use nuif_protocol::Anchor;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VIEWPORT_WIDTH: u32 = 768;
const VIEWPORT_HEIGHT: u32 = 640;
const NAVIGATION_WIDTH: Length = Length::const_px(48.0);
const LEFT_PANEL_WIDTH: Length = Length::const_px(248.0);
const RIGHT_PANEL_WIDTH: Length = Length::const_px(264.0);
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

#[derive(Clone, Copy, Debug)]
enum UiAction {
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Select(EntityId),
    ApplyInspector,
    ExportSnapshot,
    ChooseTool(Tool),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum InspectorField {
    Name(EntityId),
    Width(EntityId),
    Height(EntityId),
}

struct Driver {
    window_id: WindowId,
    root_widget_id: Option<WidgetId>,
    editor: EditorDriver,
    document_path: Option<PathBuf>,
    dirty: bool,
    status: String,
    tool: Tool,
    actions: HashMap<WidgetId, UiAction>,
    entity_widgets: HashMap<EntityId, WidgetId>,
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
            status: "Ready · profile 0 · 768 × 640".to_owned(),
            tool: Tool::Move,
            actions: HashMap::new(),
            entity_widgets: HashMap::new(),
            text_fields: HashMap::new(),
            drafts: HashMap::new(),
        }
    }

    fn build_view(&mut self) -> NewWidget<dyn Widget> {
        self.actions.clear();
        self.entity_widgets.clear();
        self.text_fields.clear();

        let snapshot = match self.editor.execute(EditorCommand::Snapshot {
            width: VIEWPORT_WIDTH,
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
                    vec![0xFF; VIEWPORT_WIDTH as usize * VIEWPORT_HEIGHT as usize * 4],
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
        let canvas = DocumentCanvas::new(VIEWPORT_WIDTH, VIEWPORT_HEIGHT, rgba, boxes, selection)
            .prepare()
            .with_props(Dimensions::STRETCH);
        let toolbar = self.build_toolbar();
        let canvas_region = ZStack::new()
            .with(canvas, UnitPoint::CENTER)
            .with(toolbar, UnitPoint::BOTTOM)
            .prepare()
            .with_props(Dimensions::STRETCH);

        let navigation = self.build_navigation();
        let left = self.build_left_panel(selection);
        let right = self.build_right_panel(selection);
        let body = Flex::row()
            .with_fixed(navigation)
            .with_fixed(left)
            .with(canvas_region, 1.0)
            .with_fixed(right);
        let status = self.build_status();
        let root = Flex::column().with(body.prepare(), 1.0).with_fixed(status);
        NewWidget::new(root).with_props(Gap::ZERO).erased()
    }

    fn build_navigation(&mut self) -> NewWidget<dyn Widget> {
        let mark = label("N", 18.0, TEXT, true);
        let mut column = Flex::column().with_fixed(mark);
        for (caption, action) in [
            ("≡", UiAction::Open),
            ("+", UiAction::New),
            ("S", UiAction::Save),
            ("⇧S", UiAction::SaveAs),
            ("↶", UiAction::Undo),
            ("↷", UiAction::Redo),
            ("↥", UiAction::ExportSnapshot),
        ] {
            column = column
                .with_fixed_spacer(8.px())
                .with_fixed(self.button(caption, action, false));
        }
        NewWidget::new(SizedBox::new(column.prepare()).width(NAVIGATION_WIDTH))
            .with_props((
                Background::Color(PANEL),
                BorderColor::new(BORDER),
                BorderWidth::all(1.px()),
                Padding::from_vh(12.px(), 7.px()),
            ))
            .erased()
    }

    fn build_left_panel(&mut self, selection: Option<EntityId>) -> NewWidget<dyn Widget> {
        let title = self
            .document_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.nuif")
            .to_owned();
        let mut content = Flex::column()
            .with_fixed(label(&title, 13.0, TEXT, true))
            .with_fixed_spacer(18.px())
            .with_fixed(label("PAGES", 10.0, MUTED, true))
            .with_fixed_spacer(7.px())
            .with_fixed(label("▣  Page 1", 12.0, TEXT, false))
            .with_fixed_spacer(18.px())
            .with_fixed(label("LAYERS", 10.0, MUTED, true));

        let rows = layer_rows(self.editor.document());
        if rows.is_empty() {
            content = content.with_fixed_spacer(8.px()).with_fixed(label(
                "No layers",
                12.0,
                MUTED,
                false,
            ));
        } else {
            for (entity, name, depth) in rows {
                let selected = selection == Some(entity);
                let caption = format!("{}{}", "  ".repeat(depth), name);
                let button = self.button(&caption, UiAction::Select(entity), selected);
                let row = AuthorContainer::new(button, entity, format!("Layer {name}")).prepare();
                self.entity_widgets.insert(entity, row.id());
                content = content.with_fixed_spacer(4.px()).with_fixed(row);
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

    fn build_right_panel(&mut self, selection: Option<EntityId>) -> NewWidget<dyn Widget> {
        let mut content = Flex::column()
            .with_fixed(label("Design    Prototype", 12.0, TEXT, true))
            .with_fixed_spacer(14.px());
        if let Some(entity_id) = selection {
            let entity = self.editor.document().entities[&entity_id].clone();
            content = content
                .with_fixed(label("SELECTION", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label(&kind_name(&entity.kind), 11.0, MUTED, false))
                .with_fixed_spacer(12.px())
                .with_fixed(label("Name", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Name(entity_id),
                    entity.name.as_deref().unwrap_or(""),
                    "Layer name",
                ))
                .with_fixed_spacer(14.px())
                .with_fixed(label("LAYOUT", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label("Width", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Width(entity_id),
                    &size_value(&entity.authored.width),
                    "Width",
                ))
                .with_fixed_spacer(8.px())
                .with_fixed(label("Height", 11.0, TEXT, false))
                .with_fixed_spacer(5.px())
                .with_fixed(self.inspector_input(
                    InspectorField::Height(entity_id),
                    &size_value(&entity.authored.height),
                    "Height",
                ))
                .with_fixed_spacer(10.px())
                .with_fixed(self.button("Apply", UiAction::ApplyInspector, true))
                .with_fixed_spacer(18.px())
                .with_fixed(label("APPEARANCE", 10.0, MUTED, true))
                .with_fixed_spacer(7.px())
                .with_fixed(label("Profile-zero solid fill", 11.0, MUTED, false));
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

    fn build_status(&self) -> NewWidget<dyn Widget> {
        let history = format!(
            "{}{}",
            if self.editor.can_undo() {
                "Undo available"
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
            | InspectorField::Height(entity) => entity,
        };
        let label = match field {
            InspectorField::Name(_) => "Name control",
            InspectorField::Width(_) => "Width control",
            InspectorField::Height(_) => "Height control",
        };
        AuthorContainer::new(input.prepare(), entity, label).prepare()
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

    fn handle_ui_action(&mut self, action: UiAction) -> bool {
        match action {
            UiAction::New => {
                self.editor = EditorDriver::new(new_native_document(EntityId::new(1)));
                self.document_path = None;
                self.dirty = false;
                self.drafts.clear();
                "Created a new profile-zero document".clone_into(&mut self.status);
            }
            UiAction::Open => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("NUIF document", &["nuif"])
                    .pick_file()
                else {
                    return false;
                };
                if let Err(error) = self.open_path(&path) {
                    self.status = format!("Open failed: {error}");
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
            UiAction::ApplyInspector => self.apply_all_drafts(),
            UiAction::ExportSnapshot => {
                if let Err(error) = self.export_snapshot() {
                    self.status = format!("Snapshot failed: {error}");
                }
            }
            UiAction::ChooseTool(tool) => {
                self.tool = tool;
                self.status = format!("{} tool selected", tool_name(tool));
            }
        }
        true
    }

    fn handle_canvas_action(&mut self, action: CanvasAction) {
        let CanvasAction::Activate {
            entity,
            document_position,
        } = action;
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
        for field in fields {
            if !self.apply_field(&field) {
                return;
            }
        }
    }

    fn apply_field(&mut self, field: &InspectorField) -> bool {
        let Some(value) = self.drafts.get(field).cloned() else {
            return true;
        };
        let (entity, field_label) = match field {
            InspectorField::Name(entity) => (*entity, "name"),
            InspectorField::Width(entity) => (*entity, "width"),
            InspectorField::Height(entity) => (*entity, "height"),
        };
        match self
            .editor
            .dispatch_accessibility_action(AccessibilityAction::SetValue {
                author_id: entity,
                label: field_label.to_owned(),
                value,
            }) {
            Ok(_) => {
                self.drafts.remove(field);
                self.dirty = true;
                self.status = format!("Updated {field_label}");
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
    use masonry_testing::{TestHarness, TestHarnessParams};

    fn harness() -> (TestHarness<SizedBox>, Driver) {
        let window_id = WindowId::next();
        let mut driver = Driver::new(window_id, new_native_document(EntityId::new(1)), None);
        let root = SizedBox::new(driver.build_view()).prepare();
        driver.root_widget_id = Some(root.id());
        let params = TestHarnessParams::default().with_size((1280, 800));
        (
            TestHarness::create_with(default_property_set(), root, params),
            driver,
        )
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
}
