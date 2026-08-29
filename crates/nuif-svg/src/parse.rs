use crate::export::{fill, number};
use crate::{
    AdapterError, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry, ImportedSource,
    MAX_SOURCE_BYTES, MAX_XML_NODES, PROFILE_NAME, RetentiveSource, SVG_NAMESPACE, SourceSpan,
    entity_pointer,
};
use nuif_adapter::AdapterReport;
use nuif_codec::canonical_hash;
use nuif_core::{
    Color, ColorSpace, Document, Entity, EntityId, EntityKind, Fidelity, Point, Semantics,
    ShapeKind, SizeIntent, TextContent,
};
use roxmltree::{Node, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

/// Imports a bounded, marked `nuif-svg-0` source document.
///
/// # Errors
///
/// Returns typed resource, XML, profile, value or canonicalization errors.
pub fn import_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_XML_NODES,
        entity_resolver: None,
    };
    let xml = roxmltree::Document::parse_with_options(source, options)
        .map_err(|error| AdapterError::XmlSyntax(error.to_string()))?;
    let root = xml.root_element();
    if root.tag_name().name() != "svg" || root.tag_name().namespace() != Some(SVG_NAMESPACE) {
        return Err(AdapterError::ProfileMarker(
            "root must be an SVG-namespace svg element".to_owned(),
        ));
    }
    required_literal(root, "data-nuif-profile", PROFILE_NAME, "/profile")?;
    let document_id = parse_id(
        required_attribute(root, "data-nuif-document", "/id")?.value(),
        "/id",
    )?;
    let mut state = ParseState {
        document: Document::empty(document_id),
        correspondences: Vec::new(),
        fidelity: Vec::new(),
        parsed_ids: BTreeSet::new(),
    };
    state.attribute_record(
        root,
        "data-nuif-document",
        CorrespondenceTarget::Document { id: document_id },
        "/id".to_owned(),
    )?;
    let surface = state.parse_entity(root, None)?;
    if surface.kind != EntityKind::Surface {
        return Err(AdapterError::ProfileMarker(
            "root data-nuif-kind must be surface".to_owned(),
        ));
    }
    state.document.roots.push(surface.id);
    state.document.entities.insert(surface.id, surface);

    let marked = root
        .descendants()
        .filter(|node| node.is_element() && node.has_attribute("data-nuif-id"))
        .count();
    if marked != state.parsed_ids.len() {
        return Err(AdapterError::ProfileMarker(
            "mapped elements must be direct children of mapped containers".to_owned(),
        ));
    }
    state.fidelity.insert(
        0,
        FidelityEntry {
            target: CorrespondenceTarget::Document { id: document_id },
            pointer: String::new(),
            status: Fidelity::Lossless,
        },
    );
    state.correspondences.sort_by(|left, right| {
        (&left.target, &left.pointer, left.span).cmp(&(&right.target, &right.pointer, right.span))
    });
    state
        .fidelity
        .sort_by(|left, right| (&left.target, &left.pointer).cmp(&(&right.target, &right.pointer)));
    let report = AdapterReport {
        schema_version: 1,
        source_format: PROFILE_NAME.to_owned(),
        canonical_hash: Some(
            canonical_hash(&state.document)
                .map_err(|error| AdapterError::Canonical(error.to_string()))?,
        ),
        fidelity: state.fidelity,
        correspondences: state.correspondences,
        unmapped_source_preserved: true,
    };
    let document = state.document;
    Ok(ImportedSource {
        document: document.clone(),
        retentive: RetentiveSource {
            source: source.to_owned(),
            document,
            report,
        },
    })
}

struct ParseState {
    document: Document,
    correspondences: Vec<CorrespondenceRecord>,
    fidelity: Vec<FidelityEntry>,
    parsed_ids: BTreeSet<EntityId>,
}

impl ParseState {
    fn parse_entity(
        &mut self,
        node: Node<'_, '_>,
        parent: Option<EntityId>,
    ) -> Result<Entity, AdapterError> {
        if node.tag_name().namespace() != Some(SVG_NAMESPACE) {
            return Err(AdapterError::ProfileMarker(
                "mapped elements must use the SVG namespace".to_owned(),
            ));
        }
        let id_attribute = required_attribute(node, "data-nuif-id", "/entities")?;
        let id = parse_id(id_attribute.value(), "/entities")?;
        if !self.parsed_ids.insert(id) {
            return Err(AdapterError::ProfileMarker(format!(
                "entity identity {id} is duplicated"
            )));
        }
        let kind_name =
            required_attribute(node, "data-nuif-kind", &entity_pointer(id, "/kind"))?.value();
        let kind = parse_kind(node.tag_name().name(), kind_name, id)?;
        let mut entity = Entity::new(id, kind);
        entity.name = node.attribute("data-nuif-name").map(str::to_owned);
        entity.semantics = Semantics {
            role: node.attribute("role").map(str::to_owned),
            accessible_name: node.attribute("aria-label").map(str::to_owned),
            states: BTreeMap::default(),
        };
        self.common_records(node, &entity)?;

        match entity.kind {
            EntityKind::Surface => self.parse_surface(node, &mut entity)?,
            EntityKind::Container => {}
            EntityKind::Shape(ShapeKind::Rectangle) => self.parse_rectangle(node, &mut entity)?,
            EntityKind::Shape(ShapeKind::Ellipse) => self.parse_ellipse(node, &mut entity)?,
            EntityKind::Text => self.parse_text(node, &mut entity)?,
            _ => unreachable!("parse_kind returns only profile kinds"),
        }
        if matches!(entity.kind, EntityKind::Surface | EntityKind::Container) {
            for child in node.children().filter(Node::is_element) {
                if child.has_attribute("data-nuif-id") {
                    let parsed = self.parse_entity(child, Some(id))?;
                    entity.children.push(parsed.id);
                    self.document.entities.insert(parsed.id, parsed);
                }
            }
        } else if node.children().any(|child| child.is_element()) {
            return invalid(
                &entity_pointer(id, "/children"),
                "mapped graphics cannot contain elements",
            );
        }
        if parent.is_none() && entity.kind != EntityKind::Surface {
            return Err(AdapterError::ProfileMarker(
                "the mapped root must be a surface".to_owned(),
            ));
        }
        self.fidelity.push(FidelityEntry {
            target: CorrespondenceTarget::Entity { id },
            pointer: entity_pointer(id, ""),
            status: Fidelity::Lossless,
        });
        Ok(entity)
    }

    fn common_records(&mut self, node: Node<'_, '_>, entity: &Entity) -> Result<(), AdapterError> {
        let target = CorrespondenceTarget::Entity { id: entity.id };
        self.attribute_record(
            node,
            "data-nuif-id",
            target.clone(),
            entity_pointer(entity.id, "/id"),
        )?;
        self.attribute_record(
            node,
            "data-nuif-kind",
            target.clone(),
            entity_pointer(entity.id, "/kind"),
        )?;
        self.optional_attribute_record(
            node,
            "data-nuif-name",
            target.clone(),
            entity_pointer(entity.id, "/name"),
        );
        self.optional_attribute_record(
            node,
            "role",
            target.clone(),
            entity_pointer(entity.id, "/semantics/role"),
        );
        self.optional_attribute_record(
            node,
            "aria-label",
            target,
            entity_pointer(entity.id, "/semantics/accessible_name"),
        );
        Ok(())
    }

    fn parse_surface(
        &mut self,
        node: Node<'_, '_>,
        entity: &mut Entity,
    ) -> Result<(), AdapterError> {
        let width = self.number_attribute(node, "width", entity.id, "/authored/width")?;
        let height = self.number_attribute(node, "height", entity.id, "/authored/height")?;
        entity.authored.width = SizeIntent::Fixed(width);
        entity.authored.height = SizeIntent::Fixed(height);
        let expected = format!("0 0 {} {}", number(width), number(height));
        required_literal(
            node,
            "viewBox",
            &expected,
            &entity_pointer(entity.id, "/authored"),
        )?;
        self.attribute_record(
            node,
            "viewBox",
            CorrespondenceTarget::Entity { id: entity.id },
            entity_pointer(entity.id, "/authored"),
        )
    }

    fn parse_rectangle(
        &mut self,
        node: Node<'_, '_>,
        entity: &mut Entity,
    ) -> Result<(), AdapterError> {
        entity.authored.position = Point {
            x: self.number_attribute(node, "x", entity.id, "/authored/position/x")?,
            y: self.number_attribute(node, "y", entity.id, "/authored/position/y")?,
        };
        entity.authored.width = SizeIntent::Fixed(self.number_attribute(
            node,
            "width",
            entity.id,
            "/authored/width",
        )?);
        entity.authored.height = SizeIntent::Fixed(self.number_attribute(
            node,
            "height",
            entity.id,
            "/authored/height",
        )?);
        entity.authored.fill = self.fill_attribute(node, entity.id)?;
        Ok(())
    }

    fn parse_ellipse(
        &mut self,
        node: Node<'_, '_>,
        entity: &mut Entity,
    ) -> Result<(), AdapterError> {
        let x = self.number_attribute(node, "data-nuif-x", entity.id, "/authored/position/x")?;
        let y = self.number_attribute(node, "data-nuif-y", entity.id, "/authored/position/y")?;
        let width = self.number_attribute(node, "data-nuif-width", entity.id, "/authored/width")?;
        let height =
            self.number_attribute(node, "data-nuif-height", entity.id, "/authored/height")?;
        entity.authored.position = Point { x, y };
        entity.authored.width = SizeIntent::Fixed(width);
        entity.authored.height = SizeIntent::Fixed(height);
        for (attribute, expected) in [
            ("cx", x + width / 2.0),
            ("cy", y + height / 2.0),
            ("rx", width / 2.0),
            ("ry", height / 2.0),
        ] {
            let observed = parse_number(
                required_attribute(node, attribute, &entity_pointer(entity.id, "/authored"))?
                    .value(),
                &entity_pointer(entity.id, "/authored"),
            )?;
            if number(observed) != number(expected) {
                return invalid(
                    &entity_pointer(entity.id, "/authored"),
                    "derived ellipse geometry does not match retained NUIF geometry",
                );
            }
            self.attribute_record(
                node,
                attribute,
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/authored"),
            )?;
        }
        entity.authored.fill = self.fill_attribute(node, entity.id)?;
        Ok(())
    }

    fn parse_text(&mut self, node: Node<'_, '_>, entity: &mut Entity) -> Result<(), AdapterError> {
        entity.authored.position = Point {
            x: self.number_attribute(node, "x", entity.id, "/authored/position/x")?,
            y: self.number_attribute(node, "y", entity.id, "/authored/position/y")?,
        };
        entity.authored.width = SizeIntent::Fixed(self.number_attribute(
            node,
            "data-nuif-width",
            entity.id,
            "/authored/width",
        )?);
        entity.authored.height = SizeIntent::Fixed(self.number_attribute(
            node,
            "data-nuif-height",
            entity.id,
            "/authored/height",
        )?);
        required_literal(
            node,
            "dominant-baseline",
            "text-before-edge",
            &entity_pointer(entity.id, "/authored/text"),
        )?;
        let font = required_attribute(
            node,
            "font-family",
            &entity_pointer(entity.id, "/authored/text/font"),
        )?
        .value()
        .to_owned();
        let font_sha256 = required_attribute(
            node,
            "data-nuif-font-sha256",
            &entity_pointer(entity.id, "/authored/text/font_sha256"),
        )?
        .value()
        .to_owned();
        let size = self.number_attribute(node, "font-size", entity.id, "/authored/text/size")?;
        let line_height = self.number_attribute(
            node,
            "data-nuif-line-height",
            entity.id,
            "/authored/text/line_height",
        )?;
        self.attribute_record(
            node,
            "font-family",
            CorrespondenceTarget::Entity { id: entity.id },
            entity_pointer(entity.id, "/authored/text/font"),
        )?;
        self.attribute_record(
            node,
            "data-nuif-font-sha256",
            CorrespondenceTarget::Entity { id: entity.id },
            entity_pointer(entity.id, "/authored/text/font_sha256"),
        )?;
        let text_nodes = node.children().filter(Node::is_text).collect::<Vec<_>>();
        if text_nodes.len() != 1 {
            return invalid(
                &entity_pointer(entity.id, "/authored/text/content"),
                "text elements require exactly one text node",
            );
        }
        let text_node = text_nodes[0];
        let content = text_node.text().unwrap_or_default().to_owned();
        self.correspondences.push(CorrespondenceRecord {
            target: CorrespondenceTarget::Entity { id: entity.id },
            pointer: entity_pointer(entity.id, "/authored/text/content"),
            span: span(text_node.range()),
        });
        entity.authored.text = Some(TextContent {
            content,
            font,
            font_sha256,
            size,
            line_height,
        });
        entity.authored.fill = self.fill_attribute(node, entity.id)?;
        Ok(())
    }

    fn number_attribute(
        &mut self,
        node: Node<'_, '_>,
        name: &str,
        id: EntityId,
        suffix: &str,
    ) -> Result<f64, AdapterError> {
        let pointer = entity_pointer(id, suffix);
        let attribute = required_attribute(node, name, &pointer)?;
        let value = parse_number(attribute.value(), &pointer)?;
        self.correspondences.push(CorrespondenceRecord {
            target: CorrespondenceTarget::Entity { id },
            pointer,
            span: span(attribute.range_value()),
        });
        Ok(value)
    }

    fn fill_attribute(
        &mut self,
        node: Node<'_, '_>,
        id: EntityId,
    ) -> Result<Option<Color>, AdapterError> {
        let pointer = entity_pointer(id, "/authored/fill");
        let attribute = required_attribute(node, "fill", &pointer)?;
        let value = parse_fill(attribute.value(), &pointer)?;
        self.correspondences.push(CorrespondenceRecord {
            target: CorrespondenceTarget::Entity { id },
            pointer,
            span: span(attribute.range_value()),
        });
        Ok(value)
    }

    fn attribute_record(
        &mut self,
        node: Node<'_, '_>,
        name: &str,
        target: CorrespondenceTarget,
        pointer: String,
    ) -> Result<(), AdapterError> {
        let attribute = required_attribute(node, name, &pointer)?;
        self.correspondences.push(CorrespondenceRecord {
            target,
            pointer,
            span: span(attribute.range_value()),
        });
        Ok(())
    }

    fn optional_attribute_record(
        &mut self,
        node: Node<'_, '_>,
        name: &str,
        target: CorrespondenceTarget,
        pointer: String,
    ) {
        if let Some(attribute) = node.attribute_node(name) {
            self.correspondences.push(CorrespondenceRecord {
                target,
                pointer,
                span: span(attribute.range_value()),
            });
        }
    }
}

fn parse_kind(tag: &str, kind: &str, id: EntityId) -> Result<EntityKind, AdapterError> {
    match (tag, kind) {
        ("svg", "surface") => Ok(EntityKind::Surface),
        ("g", "container") => Ok(EntityKind::Container),
        ("rect", "rectangle") => Ok(EntityKind::Shape(ShapeKind::Rectangle)),
        ("ellipse", "ellipse") => Ok(EntityKind::Shape(ShapeKind::Ellipse)),
        ("text", "text") => Ok(EntityKind::Text),
        _ => invalid(
            &entity_pointer(id, "/kind"),
            "element name and data-nuif-kind do not name a supported pair",
        ),
    }
}

fn required_attribute<'a, 'input>(
    node: Node<'a, 'input>,
    name: &str,
    pointer: &str,
) -> Result<roxmltree::Attribute<'a, 'input>, AdapterError> {
    node.attribute_node(name)
        .ok_or_else(|| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: format!("required attribute {name} is missing"),
        })
}

fn required_literal(
    node: Node<'_, '_>,
    name: &str,
    expected: &str,
    pointer: &str,
) -> Result<(), AdapterError> {
    let observed = required_attribute(node, name, pointer)?.value();
    if observed == expected {
        Ok(())
    } else {
        invalid(pointer, &format!("{name} must equal {expected}"))
    }
}

fn parse_id(value: &str, pointer: &str) -> Result<EntityId, AdapterError> {
    EntityId::from_str(value).map_err(|error| AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: error.to_string(),
    })
}

fn parse_number(value: &str, pointer: &str) -> Result<f64, AdapterError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|error| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: error.to_string(),
        })?;
    if parsed.is_finite() && number(parsed) == value {
        Ok(parsed)
    } else {
        invalid(pointer, "number must use the canonical finite spelling")
    }
}

fn parse_fill(value: &str, pointer: &str) -> Result<Option<Color>, AdapterError> {
    if value == "none" {
        return Ok(None);
    }
    if value.len() != 7 || !value.starts_with('#') {
        return invalid(
            pointer,
            "fill must be none or a lowercase six-digit sRGB color",
        );
    }
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&value[range], 16).map_err(|_| AdapterError::InvalidValue {
            pointer: pointer.to_owned(),
            reason: "fill must be none or a lowercase six-digit sRGB color".to_owned(),
        })
    };
    let color = Color {
        space: ColorSpace::Srgb,
        red: f32::from(channel(1..3)?) / 255.0,
        green: f32::from(channel(3..5)?) / 255.0,
        blue: f32::from(channel(5..7)?) / 255.0,
        alpha: 1.0,
    };
    if fill(Some(color)) == value {
        Ok(Some(color))
    } else {
        invalid(pointer, "fill must use lowercase hexadecimal digits")
    }
}

fn span(range: std::ops::Range<usize>) -> SourceSpan {
    SourceSpan {
        start: range.start,
        end: range.end,
    }
}

fn invalid<T>(pointer: &str, reason: &str) -> Result<T, AdapterError> {
    Err(AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: reason.to_owned(),
    })
}

pub(crate) fn encoded_scalar(
    source: &str,
    record: &CorrespondenceRecord,
) -> Result<String, AdapterError> {
    source
        .get(record.span.start..record.span.end)
        .map(str::to_owned)
        .ok_or_else(|| AdapterError::StaleSpan {
            pointer: record.pointer.clone(),
        })
}
