use nuif_codec::canonical_hash;
use nuif_core::{
    Align, Document, Edges, Entity, EntityId, EntityKind, Fidelity, FlowDirection, LayoutFamily,
    LayoutStyle, PropertyValue, SizeIntent, TextContent, Token,
};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use tree_sitter::{Node, Parser};

use crate::profile::profile_issues;
use crate::syntax::unescape_html;
use crate::{
    AdapterError, AdapterReport, CorrespondenceRecord, CorrespondenceTarget, FidelityEntry,
    ImportedSource, MAX_SOURCE_BYTES, PROFILE_NAME, RetentiveSource, SourceSpan, entity_pointer,
    token_pointer,
};

#[derive(Clone, Debug)]
struct Attribute {
    value: String,
    span: SourceSpan,
}

#[derive(Clone, Debug)]
struct RawEntity {
    id: EntityId,
    kind: String,
    parent: Option<EntityId>,
    attributes: BTreeMap<String, Attribute>,
    text: Option<Attribute>,
}

#[derive(Default)]
struct HtmlState {
    document: Option<Attribute>,
    profile: Option<Attribute>,
    style: Option<Attribute>,
    entities: Vec<RawEntity>,
}

/// Imports the bounded HTML/CSS profile and captures byte correspondences.
///
/// # Errors
///
/// Returns a typed syntax, marker, value, fidelity, resource-limit or
/// canonical error when the source is not a valid profile document.
pub fn import_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_html::LANGUAGE.into())
        .map_err(|error| AdapterError::HtmlSyntax(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| AdapterError::HtmlSyntax("parser returned no tree".to_owned()))?;
    if tree.root_node().has_error() {
        return Err(AdapterError::HtmlSyntax(first_error(tree.root_node())));
    }

    let mut state = HtmlState::default();
    walk_html(tree.root_node(), source, None, &mut state)?;
    let profile = required_marker(state.profile, "data-nuif-profile")?;
    if profile.value != PROFILE_NAME {
        return Err(AdapterError::ProfileMarker(format!(
            "expected profile {PROFILE_NAME}, found {}",
            profile.value
        )));
    }
    let document_marker = required_marker(state.document, "data-nuif-document")?;
    let document_id = parse_id(&document_marker.value, "/id")?;
    let style = required_marker(state.style, "style[data-nuif-styles]")?;
    validate_css(&style)?;

    let mut correspondences = vec![CorrespondenceRecord {
        target: CorrespondenceTarget::Document { id: document_id },
        pointer: "/id".to_owned(),
        span: document_marker.span,
    }];
    let tokens = parse_tokens(&style, &mut correspondences)?;
    let mut document = Document::empty(document_id);
    document.tokens = tokens;
    build_entities(&state.entities, &style, &mut document, &mut correspondences)?;
    let issues = profile_issues(&document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            issues: issues.len(),
            report: Box::new(report(&document, issues, correspondences, false)?),
        });
    }
    let fidelity = correspondences
        .iter()
        .map(|record| FidelityEntry {
            target: record.target.clone(),
            pointer: record.pointer.clone(),
            status: Fidelity::Lossless,
        })
        .collect();
    let report = report(&document, fidelity, correspondences, true)?;
    Ok(ImportedSource {
        document: document.clone(),
        retentive: RetentiveSource {
            source: source.to_owned(),
            document,
            report,
        },
    })
}

fn report(
    document: &Document,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<CorrespondenceRecord>,
    preserved: bool,
) -> Result<AdapterReport, AdapterError> {
    Ok(AdapterReport {
        schema_version: 1,
        source_format: PROFILE_NAME.to_owned(),
        canonical_hash: Some(
            canonical_hash(document).map_err(|error| AdapterError::Canonical(error.to_string()))?,
        ),
        fidelity,
        correspondences,
        unmapped_source_preserved: preserved,
    })
}

fn first_error(root: Node<'_>) -> String {
    let mut cursor = root.walk();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.is_error() || node.is_missing() {
            return format!(
                "{} at bytes {}..{}",
                node.kind(),
                node.start_byte(),
                node.end_byte()
            );
        }
        pending.extend(node.named_children(&mut cursor));
    }
    "tree contains an unspecified recovery node".to_owned()
}

fn walk_html(
    node: Node<'_>,
    source: &str,
    parent_entity: Option<EntityId>,
    state: &mut HtmlState,
) -> Result<(), AdapterError> {
    if node.kind() == "style_element" {
        parse_style_element(node, source, state)?;
        return Ok(());
    }
    let mut next_parent = parent_entity;
    if node.kind() == "element"
        && let Some(start_tag) = direct_child(node, "start_tag")
    {
        let attributes = attributes(start_tag, source)?;
        if let Some(value) = attributes.get("data-nuif-profile") {
            set_once(&mut state.profile, value.clone(), "data-nuif-profile")?;
        }
        if let Some(value) = attributes.get("data-nuif-document") {
            set_once(&mut state.document, value.clone(), "data-nuif-document")?;
        }
        if let Some(id_attribute) = attributes.get("data-nuif-id") {
            let id = parse_id(&id_attribute.value, "/entities/*/id")?;
            let kind = attributes
                .get("data-nuif-kind")
                .ok_or_else(|| {
                    AdapterError::ProfileMarker(format!("entity {id} lacks data-nuif-kind"))
                })?
                .value
                .clone();
            let text = (kind == "text")
                .then(|| element_text(node, start_tag, source))
                .transpose()?;
            state.entities.push(RawEntity {
                id,
                kind,
                parent: parent_entity,
                attributes,
                text,
            });
            next_parent = Some(id);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_html(child, source, next_parent, state)?;
    }
    Ok(())
}

fn parse_style_element(
    node: Node<'_>,
    source: &str,
    state: &mut HtmlState,
) -> Result<(), AdapterError> {
    let start = direct_child(node, "start_tag")
        .ok_or_else(|| AdapterError::HtmlSyntax("style element lacks start tag".to_owned()))?;
    let attributes = attributes(start, source)?;
    if !attributes.contains_key("data-nuif-styles") {
        return Ok(());
    }
    let raw = direct_child(node, "raw_text")
        .ok_or_else(|| AdapterError::ProfileMarker("mapped style element is empty".to_owned()))?;
    let value = Attribute {
        value: source[raw.byte_range()].to_owned(),
        span: SourceSpan {
            start: raw.start_byte(),
            end: raw.end_byte(),
        },
    };
    set_once(&mut state.style, value, "style[data-nuif-styles]")
}

fn direct_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn attributes(
    start_tag: Node<'_>,
    source: &str,
) -> Result<BTreeMap<String, Attribute>, AdapterError> {
    let mut values = BTreeMap::new();
    let mut cursor = start_tag.walk();
    for node in start_tag
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "attribute")
    {
        let raw = &source[node.byte_range()];
        let (name, value) = raw
            .split_once('=')
            .map_or((raw.trim(), None), |(name, value)| {
                (name.trim(), Some(value.trim()))
            });
        let attribute = if let Some(value) = value {
            if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
                return Err(AdapterError::HtmlSyntax(format!(
                    "attribute {name} must use double quotes"
                )));
            }
            let relative_start = raw.find('"').expect("opening quote exists") + 1;
            let relative_end = raw.rfind('"').expect("closing quote exists");
            Attribute {
                value: unescape_html(&raw[relative_start..relative_end]),
                span: SourceSpan {
                    start: node.start_byte() + relative_start,
                    end: node.start_byte() + relative_end,
                },
            }
        } else {
            Attribute {
                value: String::new(),
                span: SourceSpan {
                    start: node.end_byte(),
                    end: node.end_byte(),
                },
            }
        };
        if values.insert(name.to_owned(), attribute).is_some() {
            return Err(AdapterError::HtmlSyntax(format!(
                "attribute {name} is duplicated"
            )));
        }
    }
    Ok(values)
}

fn element_text(
    element: Node<'_>,
    start_tag: Node<'_>,
    source: &str,
) -> Result<Attribute, AdapterError> {
    let end_tag = direct_child(element, "end_tag")
        .ok_or_else(|| AdapterError::HtmlSyntax("text entity lacks end tag".to_owned()))?;
    let span = SourceSpan {
        start: start_tag.end_byte(),
        end: end_tag.start_byte(),
    };
    let raw = &source[span.start..span.end];
    if raw.contains('<') {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities/*/authored/text/content".to_owned(),
            reason: "text entity content cannot contain nested markup".to_owned(),
        });
    }
    Ok(Attribute {
        value: unescape_html(raw),
        span,
    })
}

fn validate_css(css: &Attribute) -> Result<(), AdapterError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_css::LANGUAGE.into())
        .map_err(|error| AdapterError::CssSyntax(error.to_string()))?;
    let tree = parser
        .parse(&css.value, None)
        .ok_or_else(|| AdapterError::CssSyntax("parser returned no tree".to_owned()))?;
    if tree.root_node().has_error() {
        Err(AdapterError::CssSyntax(first_error(tree.root_node())))
    } else {
        Ok(())
    }
}

fn parse_tokens(
    style: &Attribute,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<BTreeMap<EntityId, Token>, AdapterError> {
    let mut tokens = BTreeMap::new();
    let mut offset = 0;
    for line in style.value.split_inclusive('\n') {
        let leading = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("--nuif-token-") {
            if rest.len() < 33 {
                return invalid("/tokens", "truncated token declaration");
            }
            let id = parse_id(&rest[..32], "/tokens/*/id")?;
            let after_id = &rest[32..];
            let colon = after_id
                .find(':')
                .ok_or_else(|| invalid_error("/tokens", "token declaration lacks colon"))?;
            let after_colon = &after_id[colon + 1..];
            let px = after_colon
                .find("px;")
                .ok_or_else(|| invalid_error("/tokens", "token declaration lacks px unit"))?;
            let raw_value = &after_colon[..px];
            let value_start_in_raw = raw_value.len() - raw_value.trim_start().len();
            let value_text = raw_value.trim();
            let value = parse_number(value_text, &token_pointer(id, "/value"))?;
            let marker = "/* nuif-name:";
            let name_start = after_colon[px + 3..]
                .find(marker)
                .map(|index| px + 3 + index + marker.len())
                .ok_or_else(|| invalid_error("/tokens", "token name marker is missing"))?;
            let name_end = after_colon[name_start..]
                .find(" */")
                .map(|index| name_start + index)
                .ok_or_else(|| invalid_error("/tokens", "token name marker is unterminated"))?;
            let line_base =
                style.span.start + offset + leading + "--nuif-token-".len() + 32 + colon + 1;
            let value_span = SourceSpan {
                start: line_base + value_start_in_raw,
                end: line_base + value_start_in_raw + value_text.len(),
            };
            let name_span = SourceSpan {
                start: line_base + name_start,
                end: line_base + name_end,
            };
            correspondences.push(CorrespondenceRecord {
                target: CorrespondenceTarget::Token { id },
                pointer: token_pointer(id, "/value"),
                span: value_span,
            });
            correspondences.push(CorrespondenceRecord {
                target: CorrespondenceTarget::Token { id },
                pointer: token_pointer(id, "/name"),
                span: name_span,
            });
            if tokens
                .insert(
                    id,
                    Token {
                        id,
                        name: after_colon[name_start..name_end].to_owned(),
                        value: PropertyValue::Real(value),
                    },
                )
                .is_some()
            {
                return invalid("/tokens", "token identifier is duplicated");
            }
        }
        offset += line.len();
    }
    Ok(tokens)
}

fn build_entities(
    raw_entities: &[RawEntity],
    style: &Attribute,
    document: &mut Document,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<(), AdapterError> {
    let mut seen = BTreeSet::new();
    for raw in raw_entities {
        if !seen.insert(raw.id) {
            return invalid("/entities", "entity identifier is duplicated");
        }
        let entity = match raw.kind.as_str() {
            "container" => parse_container(raw, style, correspondences)?,
            "text" => parse_text(raw, correspondences)?,
            kind => {
                return invalid(
                    &entity_pointer(raw.id, "/kind"),
                    &format!("unsupported data-nuif-kind {kind}"),
                );
            }
        };
        document.entities.insert(raw.id, entity);
        if let Some(parent) = raw.parent {
            let parent_entity = document.entities.get_mut(&parent).ok_or_else(|| {
                invalid_error(
                    &entity_pointer(raw.id, ""),
                    "parent entity appears after child",
                )
            })?;
            parent_entity.children.push(raw.id);
        } else {
            document.roots.push(raw.id);
        }
    }
    Ok(())
}

fn parse_container(
    raw: &RawEntity,
    style: &Attribute,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<Entity, AdapterError> {
    let mut entity = Entity::new(raw.id, EntityKind::Container);
    map_common_attributes(raw, &mut entity, correspondences)?;
    let block = entity_css_block(style, raw.id)?;
    entity.authored.width = SizeIntent::Fixed(css_px(
        &block,
        "width",
        raw.id,
        "/authored/width",
        correspondences,
    )?);
    entity.authored.height = SizeIntent::Fixed(css_px(
        &block,
        "height",
        raw.id,
        "/authored/height",
        correspondences,
    )?);
    entity.authored.layout = parse_container_layout(raw.id, &block, correspondences)?;
    let token = required_attribute(raw, "data-nuif-token-spacing")?;
    let token_id = parse_id(
        &token.value,
        &entity_pointer(raw.id, "/authored/values/token.spacing"),
    )?;
    entity
        .authored
        .values
        .insert("token.spacing".to_owned(), PropertyValue::Token(token_id));
    mapped(
        correspondences,
        raw.id,
        "/authored/values/token.spacing",
        token.span,
    );
    Ok(entity)
}

fn parse_container_layout(
    id: EntityId,
    block: &CssBlock<'_>,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<LayoutStyle, AdapterError> {
    let display = css_declaration(block, "display")?;
    if display.value != "flex" {
        return invalid(
            &entity_pointer(id, "/authored/layout/family"),
            "display must be flex",
        );
    }
    mapped(correspondences, id, "/authored/layout/family", display.span);
    let direction = css_declaration(block, "flex-direction")?;
    let direction_value = match direction.value.as_str() {
        "row" => FlowDirection::Row,
        "column" => FlowDirection::Column,
        _ => {
            return invalid(
                &entity_pointer(id, "/authored/layout/direction"),
                "invalid flex direction",
            );
        }
    };
    mapped(
        correspondences,
        id,
        "/authored/layout/direction",
        direction.span,
    );
    let gap = css_px(block, "gap", id, "/authored/layout/gap", correspondences)?;
    let padding = Edges {
        top: css_px(
            block,
            "padding-top",
            id,
            "/authored/layout/padding/top",
            correspondences,
        )?,
        right: css_px(
            block,
            "padding-right",
            id,
            "/authored/layout/padding/right",
            correspondences,
        )?,
        bottom: css_px(
            block,
            "padding-bottom",
            id,
            "/authored/layout/padding/bottom",
            correspondences,
        )?,
        left: css_px(
            block,
            "padding-left",
            id,
            "/authored/layout/padding/left",
            correspondences,
        )?,
    };
    let align = css_declaration(block, "align-items")?;
    let align_value = match align.value.as_str() {
        "flex-start" => Align::Start,
        "center" => Align::Center,
        "flex-end" => Align::End,
        "stretch" => Align::Stretch,
        _ => {
            return invalid(
                &entity_pointer(id, "/authored/layout/align"),
                "invalid align-items value",
            );
        }
    };
    mapped(correspondences, id, "/authored/layout/align", align.span);
    Ok(LayoutStyle {
        family: LayoutFamily::Stack,
        direction: direction_value,
        gap,
        padding,
        align: align_value,
        ..LayoutStyle::default()
    })
}

fn parse_text(
    raw: &RawEntity,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<Entity, AdapterError> {
    let mut entity = Entity::new(raw.id, EntityKind::Text);
    map_common_attributes(raw, &mut entity, correspondences)?;
    entity.authored.width = SizeIntent::Fill;
    entity.authored.height = SizeIntent::Intrinsic;
    let content = raw.text.as_ref().ok_or_else(|| {
        invalid_error(
            &entity_pointer(raw.id, "/authored/text/content"),
            "text content is missing",
        )
    })?;
    let font = required_attribute(raw, "data-nuif-font")?;
    let hash = required_attribute(raw, "data-nuif-font-sha256")?;
    let size = required_attribute(raw, "data-nuif-font-size")?;
    let line_height = required_attribute(raw, "data-nuif-line-height")?;
    entity.authored.text = Some(TextContent {
        content: content.value.clone(),
        font: font.value.clone(),
        font_sha256: hash.value.clone(),
        size: parse_number(&size.value, &entity_pointer(raw.id, "/authored/text/size"))?,
        line_height: parse_number(
            &line_height.value,
            &entity_pointer(raw.id, "/authored/text/line_height"),
        )?,
    });
    for (suffix, attribute) in [
        ("/authored/text/content", content),
        ("/authored/text/font", font),
        ("/authored/text/font_sha256", hash),
        ("/authored/text/size", size),
        ("/authored/text/line_height", line_height),
    ] {
        mapped(correspondences, raw.id, suffix, attribute.span);
    }
    Ok(entity)
}

fn map_common_attributes(
    raw: &RawEntity,
    entity: &mut Entity,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<(), AdapterError> {
    let id = required_attribute(raw, "data-nuif-id")?;
    mapped(correspondences, raw.id, "/id", id.span);
    let kind = required_attribute(raw, "data-nuif-kind")?;
    mapped(correspondences, raw.id, "/kind", kind.span);
    if let Some(name) = raw.attributes.get("data-nuif-name") {
        entity.name = Some(name.value.clone());
        mapped(correspondences, raw.id, "/name", name.span);
    }
    Ok(())
}

struct CssBlock<'a> {
    value: &'a str,
    start: usize,
}

fn entity_css_block(style: &Attribute, id: EntityId) -> Result<CssBlock<'_>, AdapterError> {
    let selector = format!("[data-nuif-id=\"{id}\"]");
    let selector_start = unique_find(
        &style.value,
        &selector,
        &entity_pointer(id, "/authored/layout"),
    )?;
    let after_selector = selector_start + selector.len();
    let open = style.value[after_selector..]
        .find('{')
        .map(|index| after_selector + index)
        .ok_or_else(|| {
            invalid_error(
                &entity_pointer(id, "/authored/layout"),
                "CSS rule lacks opening brace",
            )
        })?;
    let close = style.value[open + 1..]
        .find('}')
        .map(|index| open + 1 + index)
        .ok_or_else(|| {
            invalid_error(
                &entity_pointer(id, "/authored/layout"),
                "CSS rule lacks closing brace",
            )
        })?;
    Ok(CssBlock {
        value: &style.value[open + 1..close],
        start: style.span.start + open + 1,
    })
}

fn css_declaration(block: &CssBlock<'_>, property: &str) -> Result<Attribute, AdapterError> {
    let mut found = None;
    let mut offset = 0;
    for line in block.value.split_inclusive('\n') {
        let leading = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(property)
            && let Some(after_colon) = rest.strip_prefix(':')
        {
            let before_semicolon = after_colon
                .split_once(';')
                .map_or(after_colon, |(value, _)| value);
            let value = before_semicolon.trim();
            let value_leading = before_semicolon.len() - before_semicolon.trim_start().len();
            let start = block.start + offset + leading + property.len() + 1 + value_leading;
            let attribute = Attribute {
                value: value.to_owned(),
                span: SourceSpan {
                    start,
                    end: start + value.len(),
                },
            };
            if found.replace(attribute).is_some() {
                return invalid(property, "CSS declaration is duplicated");
            }
        }
        offset += line.len();
    }
    found.ok_or_else(|| invalid_error(property, "CSS declaration is missing"))
}

fn css_px(
    block: &CssBlock<'_>,
    property: &str,
    id: EntityId,
    suffix: &str,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<f64, AdapterError> {
    let declaration = css_declaration(block, property)?;
    let number = declaration
        .value
        .strip_suffix("px")
        .ok_or_else(|| invalid_error(&entity_pointer(id, suffix), "CSS length must use px"))?;
    let number_length = number.trim_end().len();
    let span = SourceSpan {
        start: declaration.span.start,
        end: declaration.span.start + number_length,
    };
    mapped(correspondences, id, suffix, span);
    parse_number(number.trim(), &entity_pointer(id, suffix))
}

fn mapped(
    correspondences: &mut Vec<CorrespondenceRecord>,
    id: EntityId,
    suffix: &str,
    span: SourceSpan,
) {
    correspondences.push(CorrespondenceRecord {
        target: CorrespondenceTarget::Entity { id },
        pointer: entity_pointer(id, suffix),
        span,
    });
}

fn required_attribute<'a>(raw: &'a RawEntity, name: &str) -> Result<&'a Attribute, AdapterError> {
    raw.attributes
        .get(name)
        .ok_or_else(|| AdapterError::ProfileMarker(format!("entity {} lacks {name}", raw.id)))
}

fn required_marker(value: Option<Attribute>, name: &str) -> Result<Attribute, AdapterError> {
    value.ok_or_else(|| AdapterError::ProfileMarker(format!("{name} is missing")))
}

fn set_once(
    target: &mut Option<Attribute>,
    value: Attribute,
    name: &str,
) -> Result<(), AdapterError> {
    if target.replace(value).is_some() {
        Err(AdapterError::ProfileMarker(format!("{name} is duplicated")))
    } else {
        Ok(())
    }
}

fn unique_find(source: &str, needle: &str, pointer: &str) -> Result<usize, AdapterError> {
    let first = source
        .find(needle)
        .ok_or_else(|| invalid_error(pointer, "mapped source marker is missing"))?;
    if source[first + needle.len()..].contains(needle) {
        invalid(pointer, "mapped source marker is duplicated")
    } else {
        Ok(first)
    }
}

fn parse_id(value: &str, pointer: &str) -> Result<EntityId, AdapterError> {
    EntityId::from_str(value).map_err(|error| invalid_error(pointer, &error.to_string()))
}

fn parse_number(value: &str, pointer: &str) -> Result<f64, AdapterError> {
    value
        .parse::<f64>()
        .map_err(|error| invalid_error(pointer, &error.to_string()))
        .and_then(|number| {
            if number.is_finite() {
                Ok(number)
            } else {
                invalid(pointer, "number must be finite")
            }
        })
}

fn invalid<T>(pointer: &str, reason: &str) -> Result<T, AdapterError> {
    Err(invalid_error(pointer, reason))
}

fn invalid_error(pointer: &str, reason: &str) -> AdapterError {
    AdapterError::InvalidValue {
        pointer: pointer.to_owned(),
        reason: reason.to_owned(),
    }
}
