use nuif_codec::canonical_hash;
use nuif_core::{
    Align, AuthoredProperties, Color, ColorSpace, Document, Edges, Entity, EntityId, EntityKind,
    Fidelity, FlowDirection, LayoutFamily, PropertyValue, ResponsiveOverride, Severity, SizeIntent,
    TextContent, Token, validate,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::str::FromStr;
use tree_sitter::{Node, Parser};

use crate::syntax::{escape_html, number, unescape_html};
use crate::{
    AdapterError, AdapterReport, CorrespondenceRecord, CorrespondenceTarget, ExportedSource,
    FidelityEntry, ImportedSource, MAX_SOURCE_BYTES, RetentiveSource, SourceEdit, SourceSpan,
    SynchronizedSource, entity_pointer, token_pointer,
};

pub const V0_PROFILE_NAME: &str = "nuif-html-css-v0";

type Key = (CorrespondenceTarget, String);

#[derive(Clone, Debug)]
struct Attribute {
    value: String,
    span: SourceSpan,
}

#[derive(Clone, Debug)]
struct RawEntity {
    id: EntityId,
    parent: Option<EntityId>,
    attributes: BTreeMap<String, Attribute>,
    text: Attribute,
}

#[derive(Default)]
struct HtmlState {
    document_attributes: Option<BTreeMap<String, Attribute>>,
    style: Option<Attribute>,
    entities: Vec<RawEntity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextStyle {
    font: String,
    font_sha256: String,
    size: f64,
    line_height: f64,
}

/// Exports the complete responsive-card model through the explicit v0 HTML/CSS mapping.
///
/// # Errors
///
/// Returns a typed unsupported-profile report or a parser/self-verification failure.
pub fn export_v0_document(document: &Document) -> Result<ExportedSource, AdapterError> {
    let issues = v0_profile_issues(document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            issues: issues.len(),
            report: Box::new(AdapterReport {
                schema_version: 1,
                source_format: V0_PROFILE_NAME.to_owned(),
                canonical_hash: canonical_hash(document).ok(),
                fidelity: issues,
                correspondences: Vec::new(),
                unmapped_source_preserved: false,
            }),
        });
    }
    let source = render(document)?;
    let imported = import_v0_source(&source)?;
    if imported.document != *document {
        return Err(AdapterError::SynchronizationMismatch);
    }
    Ok(ExportedSource {
        source,
        report: imported.retentive.report,
    })
}

/// Imports `nuif-html-css-v0` while retaining every mapped scalar byte span.
///
/// # Errors
///
/// Returns a typed resource, syntax, marker, value, profile, or hashing failure.
pub fn import_v0_source(source: &str) -> Result<ImportedSource, AdapterError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(AdapterError::SourceTooLarge);
    }
    let state = parse_html(source)?;
    let attributes = state
        .document_attributes
        .ok_or_else(|| AdapterError::ProfileMarker("v0 document marker is missing".to_owned()))?;
    let profile = required_attribute(&attributes, "data-nuif-profile", "document")?;
    if profile.value != V0_PROFILE_NAME {
        return Err(AdapterError::ProfileMarker(format!(
            "expected profile {V0_PROFILE_NAME}, found {}",
            profile.value
        )));
    }
    let document_attribute = required_attribute(&attributes, "data-nuif-document", "document")?;
    let document_id = parse_id(&document_attribute.value, "/id")?;
    let style = state.style.ok_or_else(|| {
        AdapterError::ProfileMarker("style[data-nuif-v0-styles] is missing".to_owned())
    })?;
    validate_css(&style.value)?;

    let mut correspondences = Vec::new();
    document_mapped(
        &mut correspondences,
        document_id,
        "/id",
        document_attribute.span,
    );
    let relations_attribute = required_attribute(&attributes, "data-nuif-relations", "document")?;
    let declarations_attribute =
        required_attribute(&attributes, "data-nuif-extension-declarations", "document")?;
    let extensions_attribute = required_attribute(&attributes, "data-nuif-extensions", "document")?;
    let mut document = Document::empty(document_id);
    document.relations = parse_json(&relations_attribute.value, "/relations")?;
    document.extension_declarations =
        parse_json(&declarations_attribute.value, "/extension_declarations")?;
    document.extensions = parse_json(&extensions_attribute.value, "/extensions")?;
    document_mapped(
        &mut correspondences,
        document_id,
        "/relations",
        relations_attribute.span,
    );
    document_mapped(
        &mut correspondences,
        document_id,
        "/extension_declarations",
        declarations_attribute.span,
    );
    document_mapped(
        &mut correspondences,
        document_id,
        "/extensions",
        extensions_attribute.span,
    );
    document.tokens = parse_tokens(&style, &mut correspondences)?;
    build_entities(&state.entities, &style, &mut document, &mut correspondences)?;

    let issues = v0_profile_issues(&document);
    if !issues.is_empty() {
        return Err(AdapterError::UnsupportedProfile {
            issues: issues.len(),
            report: Box::new(v0_report(&document, issues, correspondences, false)?),
        });
    }
    validate_marker_inventory(&document, &style.value)?;
    validate_derived_css(&document, &style, &mut correspondences)?;
    let mut fidelity = correspondences
        .iter()
        .map(|record| FidelityEntry {
            target: record.target.clone(),
            pointer: record.pointer.clone(),
            status: Fidelity::Lossless,
        })
        .collect::<Vec<_>>();
    fidelity.extend(target_fidelity(&document));
    let report = v0_report(&document, fidelity, correspondences, true)?;
    Ok(ImportedSource {
        document: document.clone(),
        retentive: RetentiveSource {
            source: source.to_owned(),
            document,
            report,
        },
    })
}

/// Applies v0 semantic edits only to their retained source spans.
///
/// # Errors
///
/// Returns typed stale, unsupported, syntax, or self-verification failures.
pub fn synchronize_v0(
    retentive: &RetentiveSource,
    edited: &Document,
) -> Result<SynchronizedSource, AdapterError> {
    let generated_before = export_v0_document(&retentive.document)?;
    let generated_after = match export_v0_document(edited) {
        Ok(exported) => exported,
        Err(AdapterError::UnsupportedProfile { report, .. }) => {
            return Err(AdapterError::UnmappedChanges {
                issues: report.fidelity.len(),
                report,
            });
        }
        Err(error) => return Err(error),
    };
    let structural = structural_issues(&retentive.document, edited);
    if !structural.is_empty() {
        return Err(AdapterError::UnmappedChanges {
            issues: structural.len(),
            report: Box::new(AdapterReport {
                schema_version: 1,
                source_format: V0_PROFILE_NAME.to_owned(),
                canonical_hash: generated_after.report.canonical_hash,
                fidelity: structural,
                correspondences: retentive.report.correspondences.clone(),
                unmapped_source_preserved: false,
            }),
        });
    }
    let current = correspondence_map(&retentive.report.correspondences)?;
    let before = correspondence_values(
        &generated_before.source,
        &generated_before.report.correspondences,
    )?;
    let after = correspondence_values(
        &generated_after.source,
        &generated_after.report.correspondences,
    )?;
    let current_keys = current.keys().cloned().collect::<BTreeSet<_>>();
    let before_keys = before.keys().cloned().collect::<BTreeSet<_>>();
    let after_keys = after.keys().cloned().collect::<BTreeSet<_>>();
    if current_keys != before_keys || current_keys != after_keys {
        let report = unmapped_key_report(edited, &current_keys, &after_keys);
        return Err(AdapterError::UnmappedChanges {
            issues: report.fidelity.len(),
            report: Box::new(report),
        });
    }

    let mut edits = Vec::new();
    for (key, record) in current {
        let expected = &before[&key];
        let observed = retentive
            .source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| AdapterError::StaleSpan {
                pointer: record.pointer.clone(),
            })?;
        if observed != expected {
            return Err(AdapterError::StaleSpan {
                pointer: record.pointer,
            });
        }
        let replacement = &after[&key];
        if observed != replacement {
            edits.push(SourceEdit {
                target: record.target,
                pointer: record.pointer,
                span: record.span,
                replacement: replacement.clone(),
            });
        }
    }
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.span.start));
    for pair in edits.windows(2) {
        if pair[1].span.end > pair[0].span.start {
            return Err(AdapterError::StaleSpan {
                pointer: pair[1].pointer.clone(),
            });
        }
    }
    let mut source = retentive.source.clone();
    for edit in &edits {
        source.replace_range(edit.span.start..edit.span.end, &edit.replacement);
    }
    edits.sort_by_key(|edit| edit.span.start);
    let imported = import_v0_source(&source)?;
    if imported.document != *edited {
        return Err(AdapterError::SynchronizationMismatch);
    }
    let mut report = imported.retentive.report;
    report.unmapped_source_preserved = true;
    Ok(SynchronizedSource {
        source,
        edits,
        report,
    })
}

fn render(document: &Document) -> Result<String, AdapterError> {
    let mut source = String::new();
    source.push_str("<!doctype html>\n");
    writeln!(
        source,
        "<html data-nuif-profile=\"{V0_PROFILE_NAME}\" data-nuif-document=\"{}\" data-nuif-relations=\"{}\" data-nuif-extension-declarations=\"{}\" data-nuif-extensions=\"{}\">",
        document.id,
        json_attribute(&document.relations)?,
        json_attribute(&document.extension_declarations)?,
        json_attribute(&document.extensions)?,
    )
    .expect("writing to a String cannot fail");
    source.push_str("<head>\n  <meta charset=\"utf-8\">\n  <style data-nuif-v0-styles=\"\">\n");
    source.push_str("    :root {\n");
    for token in document.tokens.values() {
        writeln!(
            source,
            "      --nuif-token-{}: {}; /* nuif-name:{}; nuif-type:{} */",
            token.id,
            token_css_value(&token.value)?,
            token.name,
            token_type(&token.value),
        )
        .expect("writing to a String cannot fail");
    }
    source.push_str("    }\n");
    for entity in document.entities.values() {
        render_entity_css(&mut source, entity);
    }
    for entity in document.entities.values() {
        for (index, rule) in entity.authored.responsive.iter().enumerate() {
            source.push_str(&render_responsive_css(entity.id, index, rule));
        }
    }
    source.push_str("  </style>\n</head>\n<body>\n");
    for root in &document.roots {
        render_entity_html(&mut source, document, *root, 1)?;
    }
    source.push_str("</body>\n</html>\n");
    Ok(source)
}

fn render_entity_css(source: &mut String, entity: &Entity) {
    let authored = &entity.authored;
    writeln!(source, "    /* nuif-entity-begin:{} */", entity.id)
        .expect("writing to a String cannot fail");
    writeln!(source, "    [data-nuif-id=\"{}\"] {{", entity.id)
        .expect("writing to a String cannot fail");
    writeln!(source, "      width: {};", size_css(&authored.width))
        .expect("writing to a String cannot fail");
    writeln!(source, "      height: {};", size_css(&authored.height))
        .expect("writing to a String cannot fail");
    writeln!(source, "      left: {}px;", number(authored.position.x))
        .expect("writing to a String cannot fail");
    writeln!(source, "      top: {}px;", number(authored.position.y))
        .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      --nuif-layout-family: {};",
        layout_family_css(authored.layout.family)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      display: {};",
        if authored.layout.family == LayoutFamily::Stack {
            "flex"
        } else {
            "block"
        }
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      flex-direction: {};",
        direction_css(authored.layout.direction)
    )
    .expect("writing to a String cannot fail");
    writeln!(source, "      gap: {}px;", number(authored.layout.gap))
        .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      padding-top: {}px;",
        number(authored.layout.padding.top)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      padding-right: {}px;",
        number(authored.layout.padding.right)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      padding-bottom: {}px;",
        number(authored.layout.padding.bottom)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      padding-left: {}px;",
        number(authored.layout.padding.left)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      align-items: {};",
        align_css(authored.layout.align)
    )
    .expect("writing to a String cannot fail");
    writeln!(
        source,
        "      background-color: {};",
        fill_css(authored.fill)
    )
    .expect("writing to a String cannot fail");
    source.push_str("    }\n");
    writeln!(source, "    /* nuif-entity-end:{} */", entity.id)
        .expect("writing to a String cannot fail");
}

fn render_entity_html(
    source: &mut String,
    document: &Document,
    id: EntityId,
    depth: usize,
) -> Result<(), AdapterError> {
    let entity = &document.entities[&id];
    let indent = "  ".repeat(depth);
    let kind = json_attribute(&entity.kind)?;
    let name = json_attribute(&entity.name)?;
    let responsive = json_attribute(&entity.authored.responsive)?;
    let values = json_attribute(&entity.authored.values)?;
    let semantics = json_attribute(&entity.semantics)?;
    let extensions = json_attribute(&entity.extensions)?;
    write!(
        source,
        "{indent}<div data-nuif-id=\"{}\" data-nuif-kind=\"{kind}\" data-nuif-name=\"{name}\" data-nuif-responsive=\"{responsive}\" data-nuif-values=\"{values}\" data-nuif-semantics=\"{semantics}\" data-nuif-extensions=\"{extensions}\"",
        entity.id,
    )
    .expect("writing to a String cannot fail");
    if let Some(text) = &entity.authored.text {
        let style = TextStyle {
            font: text.font.clone(),
            font_sha256: text.font_sha256.clone(),
            size: text.size,
            line_height: text.line_height,
        };
        write!(
            source,
            " data-nuif-text-style=\"{}\"",
            json_attribute(&style)?
        )
        .expect("writing to a String cannot fail");
    }
    source.push('>');
    if let Some(text) = &entity.authored.text {
        source.push_str(&escape_html(&text.content));
    }
    if entity.children.is_empty() {
        source.push_str("</div>\n");
    } else {
        source.push('\n');
        for child in &entity.children {
            render_entity_html(source, document, *child, depth + 1)?;
        }
        writeln!(source, "{indent}</div>").expect("writing to a String cannot fail");
    }
    Ok(())
}

fn parse_html(source: &str) -> Result<HtmlState, AdapterError> {
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
    Ok(state)
}

fn walk_html(
    node: Node<'_>,
    source: &str,
    parent: Option<EntityId>,
    state: &mut HtmlState,
) -> Result<(), AdapterError> {
    if node.kind() == "style_element" {
        parse_style_element(node, source, state)?;
        return Ok(());
    }
    let mut next_parent = parent;
    if node.kind() == "element"
        && let Some(start_tag) = direct_child(node, "start_tag")
    {
        let attributes = attributes(start_tag, source)?;
        if attributes
            .get("data-nuif-profile")
            .is_some_and(|value| value.value == V0_PROFILE_NAME)
            && state
                .document_attributes
                .replace(attributes.clone())
                .is_some()
        {
            return Err(AdapterError::ProfileMarker(
                "v0 document marker is duplicated".to_owned(),
            ));
        }
        if let Some(id_attribute) = attributes.get("data-nuif-id") {
            let id = parse_id(&id_attribute.value, "/entities/*/id")?;
            let text = element_text(node, start_tag, source)?;
            state.entities.push(RawEntity {
                id,
                parent,
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
    let start_tag = direct_child(node, "start_tag")
        .ok_or_else(|| AdapterError::HtmlSyntax("style element lacks start tag".to_owned()))?;
    let attributes = attributes(start_tag, source)?;
    if !attributes.contains_key("data-nuif-v0-styles") {
        return Ok(());
    }
    let raw = direct_child(node, "raw_text")
        .ok_or_else(|| AdapterError::ProfileMarker("mapped style is empty".to_owned()))?;
    let attribute = Attribute {
        value: source[raw.byte_range()].to_owned(),
        span: SourceSpan {
            start: raw.start_byte(),
            end: raw.end_byte(),
        },
    };
    if state.style.replace(attribute).is_some() {
        return Err(AdapterError::ProfileMarker(
            "style[data-nuif-v0-styles] is duplicated".to_owned(),
        ));
    }
    Ok(())
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
        let (name, raw_value) = raw.split_once('=').ok_or_else(|| {
            AdapterError::HtmlSyntax(format!("mapped attribute {raw} lacks a value"))
        })?;
        let raw_value = raw_value.trim();
        if !raw_value.starts_with('"') || !raw_value.ends_with('"') || raw_value.len() < 2 {
            return Err(AdapterError::HtmlSyntax(format!(
                "attribute {} must use double quotes",
                name.trim()
            )));
        }
        let relative_start = raw.find('"').expect("opening quote exists") + 1;
        let relative_end = raw.rfind('"').expect("closing quote exists");
        let attribute = Attribute {
            value: unescape_html(&raw[relative_start..relative_end]),
            span: SourceSpan {
                start: node.start_byte() + relative_start,
                end: node.start_byte() + relative_end,
            },
        };
        if values.insert(name.trim().to_owned(), attribute).is_some() {
            return Err(AdapterError::HtmlSyntax(format!(
                "attribute {} is duplicated",
                name.trim()
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
        .ok_or_else(|| AdapterError::HtmlSyntax("mapped entity lacks end tag".to_owned()))?;
    let span = SourceSpan {
        start: start_tag.end_byte(),
        end: end_tag.start_byte(),
    };
    let raw = &source[span.start..span.end];
    if raw.contains("data-nuif-id=") {
        return Ok(Attribute {
            value: String::new(),
            span: SourceSpan {
                start: span.start,
                end: span.start,
            },
        });
    }
    if raw.contains('<') {
        return Err(AdapterError::InvalidValue {
            pointer: "/entities/*/authored/text/content".to_owned(),
            reason: "mapped text cannot contain nested markup".to_owned(),
        });
    }
    Ok(Attribute {
        value: unescape_html(raw),
        span,
    })
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
        let id_attribute = required_attribute(&raw.attributes, "data-nuif-id", "entity")?;
        let kind_attribute = required_attribute(&raw.attributes, "data-nuif-kind", "entity")?;
        let name_attribute = required_attribute(&raw.attributes, "data-nuif-name", "entity")?;
        let responsive_attribute =
            required_attribute(&raw.attributes, "data-nuif-responsive", "entity")?;
        let values_attribute = required_attribute(&raw.attributes, "data-nuif-values", "entity")?;
        let semantics_attribute =
            required_attribute(&raw.attributes, "data-nuif-semantics", "entity")?;
        let extensions_attribute =
            required_attribute(&raw.attributes, "data-nuif-extensions", "entity")?;
        let kind: EntityKind = parse_json(&kind_attribute.value, &entity_pointer(raw.id, "/kind"))?;
        let mut entity = Entity::new(raw.id, kind);
        entity.name = parse_json(&name_attribute.value, &entity_pointer(raw.id, "/name"))?;
        entity.authored.responsive = parse_json(
            &responsive_attribute.value,
            &entity_pointer(raw.id, "/authored/responsive"),
        )?;
        entity.authored.values = parse_json(
            &values_attribute.value,
            &entity_pointer(raw.id, "/authored/values"),
        )?;
        entity.semantics = parse_json(
            &semantics_attribute.value,
            &entity_pointer(raw.id, "/semantics"),
        )?;
        entity.extensions = parse_json(
            &extensions_attribute.value,
            &entity_pointer(raw.id, "/extensions"),
        )?;
        entity.authored = parse_authored_css(raw, style, entity.authored, correspondences)?;
        if let Some(text_style) = raw.attributes.get("data-nuif-text-style") {
            let parsed: TextStyle =
                parse_json(&text_style.value, &entity_pointer(raw.id, "/authored/text"))?;
            entity.authored.text = Some(TextContent {
                content: raw.text.value.clone(),
                font: parsed.font,
                font_sha256: parsed.font_sha256,
                font_asset: None,
                size: parsed.size,
                line_height: parsed.line_height,
            });
            entity_mapped(
                correspondences,
                raw.id,
                "/authored/text/style",
                text_style.span,
            );
            entity_mapped(
                correspondences,
                raw.id,
                "/authored/text/content",
                raw.text.span,
            );
        } else if !raw.text.value.is_empty() {
            return invalid(
                &entity_pointer(raw.id, "/authored/text"),
                "non-text entity contains direct text",
            );
        }
        for (suffix, attribute) in [
            ("/id", id_attribute),
            ("/kind", kind_attribute),
            ("/name", name_attribute),
            ("/authored/responsive", responsive_attribute),
            ("/authored/values", values_attribute),
            ("/semantics", semantics_attribute),
            ("/extensions", extensions_attribute),
        ] {
            entity_mapped(correspondences, raw.id, suffix, attribute.span);
        }
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

fn parse_authored_css(
    raw: &RawEntity,
    style: &Attribute,
    mut authored: AuthoredProperties,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<AuthoredProperties, AdapterError> {
    let block = entity_css_block(style, raw.id)?;
    authored.width =
        parse_size_declaration(&block, "width", raw.id, "/authored/width", correspondences)?;
    authored.height = parse_size_declaration(
        &block,
        "height",
        raw.id,
        "/authored/height",
        correspondences,
    )?;
    authored.position.x = css_px(
        &block,
        "left",
        raw.id,
        "/authored/position/x",
        correspondences,
    )?;
    authored.position.y = css_px(
        &block,
        "top",
        raw.id,
        "/authored/position/y",
        correspondences,
    )?;
    let family = css_declaration(&block, "--nuif-layout-family")?;
    authored.layout.family = parse_layout_family(&family.value, raw.id)?;
    entity_mapped(
        correspondences,
        raw.id,
        "/authored/layout/family",
        family.span,
    );
    let display = css_declaration(&block, "display")?;
    let expected_display = if authored.layout.family == LayoutFamily::Stack {
        "flex"
    } else {
        "block"
    };
    if display.value != expected_display {
        return invalid(
            &entity_pointer(raw.id, "/authored/layout/rendered_display"),
            "display is inconsistent with the mapped layout family",
        );
    }
    entity_mapped(
        correspondences,
        raw.id,
        "/authored/layout/rendered_display",
        display.span,
    );
    let direction = css_declaration(&block, "flex-direction")?;
    authored.layout.direction = parse_direction(&direction.value, raw.id)?;
    entity_mapped(
        correspondences,
        raw.id,
        "/authored/layout/direction",
        direction.span,
    );
    authored.layout.gap = css_px(
        &block,
        "gap",
        raw.id,
        "/authored/layout/gap",
        correspondences,
    )?;
    authored.layout.padding = parse_padding(&block, raw.id, correspondences)?;
    let align = css_declaration(&block, "align-items")?;
    authored.layout.align = parse_align(&align.value, raw.id)?;
    entity_mapped(
        correspondences,
        raw.id,
        "/authored/layout/align",
        align.span,
    );
    let fill = css_declaration(&block, "background-color")?;
    authored.fill = parse_fill(&fill.value, raw.id)?;
    entity_mapped(correspondences, raw.id, "/authored/fill", fill.span);
    Ok(authored)
}

fn parse_padding(
    block: &CssBlock<'_>,
    id: EntityId,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<Edges, AdapterError> {
    Ok(Edges {
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
    })
}

#[derive(Clone, Copy)]
struct CssBlock<'a> {
    value: &'a str,
    start: usize,
}

fn entity_css_block(style: &Attribute, id: EntityId) -> Result<CssBlock<'_>, AdapterError> {
    let begin = format!("/* nuif-entity-begin:{id} */");
    let end = format!("/* nuif-entity-end:{id} */");
    let begin_start = unique_find(&style.value, &begin, &entity_pointer(id, "/authored"))?;
    let content_start = begin_start + begin.len();
    let content_end = style.value[content_start..]
        .find(&end)
        .map(|offset| content_start + offset)
        .ok_or_else(|| {
            invalid_error(
                &entity_pointer(id, "/authored"),
                "CSS end marker is missing",
            )
        })?;
    Ok(CssBlock {
        value: &style.value[content_start..content_end],
        start: style.span.start + content_start,
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

fn validate_derived_css(
    document: &Document,
    style: &Attribute,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<(), AdapterError> {
    for entity in document.entities.values() {
        for (index, rule) in entity.authored.responsive.iter().enumerate() {
            let begin = format!("/* nuif-responsive-begin:{}:{index} */", entity.id);
            let end = format!("/* nuif-responsive-end:{}:{index} */", entity.id);
            let begin_start = unique_find(
                &style.value,
                &begin,
                &entity_pointer(entity.id, "/authored/responsive"),
            )?;
            let start = begin_start + begin.len();
            let finish = style.value[start..]
                .find(&end)
                .map(|offset| start + offset)
                .ok_or_else(|| {
                    invalid_error(
                        &entity_pointer(entity.id, "/authored/responsive"),
                        "responsive CSS end marker is missing",
                    )
                })?;
            let expected = format!("{}    ", render_responsive_body(entity.id, rule));
            if style.value[start..finish] != expected {
                return invalid(
                    &entity_pointer(entity.id, &format!("/authored/responsive/rendered/{index}")),
                    "responsive CSS is inconsistent with the mapped rule",
                );
            }
            entity_mapped(
                correspondences,
                entity.id,
                &format!("/authored/responsive/rendered/{index}"),
                SourceSpan {
                    start: style.span.start + start,
                    end: style.span.start + finish,
                },
            );
        }
    }
    Ok(())
}

fn validate_marker_inventory(document: &Document, source: &str) -> Result<(), AdapterError> {
    let entities = document.entities.len();
    let responsive = document
        .entities
        .values()
        .map(|entity| entity.authored.responsive.len())
        .sum::<usize>();
    for (marker, expected) in [
        ("/* nuif-entity-begin:", entities),
        ("/* nuif-entity-end:", entities),
        ("/* nuif-responsive-begin:", responsive),
        ("/* nuif-responsive-end:", responsive),
    ] {
        let observed = source.matches(marker).count();
        if observed != expected {
            return invalid(
                "/",
                &format!(
                    "reserved CSS marker {marker} occurs {observed} times; expected {expected}"
                ),
            );
        }
    }
    Ok(())
}

fn render_responsive_css(id: EntityId, index: usize, rule: &ResponsiveOverride) -> String {
    format!(
        "    /* nuif-responsive-begin:{id}:{index} */{}    /* nuif-responsive-end:{id}:{index} */\n",
        render_responsive_body(id, rule)
    )
}

fn render_responsive_body(id: EntityId, rule: &ResponsiveOverride) -> String {
    let mut conditions = Vec::new();
    if let Some(minimum) = rule.when.min_width {
        conditions.push(format!("(min-width: {}px)", number(minimum)));
    }
    if let Some(maximum) = rule.when.max_width {
        conditions.push(format!("(max-width: {}px)", number(maximum)));
    }
    let mut result = format!(
        "\n    @media {} {{\n      [data-nuif-id=\"{id}\"] {{\n",
        conditions.join(" and ")
    );
    if let Some(direction) = rule.direction {
        writeln!(
            result,
            "        flex-direction: {};",
            direction_css(direction)
        )
        .expect("writing to a String cannot fail");
    }
    if let Some(gap) = rule.gap {
        writeln!(result, "        gap: {}px;", number(gap))
            .expect("writing to a String cannot fail");
    }
    result.push_str("      }\n    }\n");
    result
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
            let after_colon = after_id
                .strip_prefix(':')
                .ok_or_else(|| invalid_error("/tokens", "token declaration lacks colon"))?;
            let (raw_value, comment) = after_colon
                .split_once(';')
                .ok_or_else(|| invalid_error("/tokens", "token declaration lacks semicolon"))?;
            let value = raw_value.trim();
            let name_marker = "/* nuif-name:";
            let type_marker = "; nuif-type:";
            let name_start = comment
                .find(name_marker)
                .map(|index| index + name_marker.len())
                .ok_or_else(|| invalid_error("/tokens", "token name marker is missing"))?;
            let name_end = comment[name_start..]
                .find(type_marker)
                .map(|index| name_start + index)
                .ok_or_else(|| invalid_error("/tokens", "token type marker is missing"))?;
            let type_start = name_end + type_marker.len();
            let type_end = comment[type_start..]
                .find(" */")
                .map(|index| type_start + index)
                .ok_or_else(|| invalid_error("/tokens", "token marker is unterminated"))?;
            let kind = &comment[type_start..type_end];
            let property_value = match kind {
                "real" => PropertyValue::Real(parse_number(
                    value.strip_suffix("px").ok_or_else(|| {
                        invalid_error(&token_pointer(id, "/value"), "real token must use px")
                    })?,
                    &token_pointer(id, "/value"),
                )?),
                "string" => PropertyValue::String(value.to_owned()),
                _ => return invalid(&token_pointer(id, "/value"), "unsupported token type"),
            };
            let value_relative = after_colon.find(value).expect("trimmed value is present");
            let line_base = style.span.start + offset + leading + "--nuif-token-".len() + 32 + 1;
            let comment_base = line_base + after_colon.find(comment).expect("comment is present");
            correspondences.push(CorrespondenceRecord {
                target: CorrespondenceTarget::Token { id },
                pointer: token_pointer(id, "/value"),
                span: SourceSpan {
                    start: line_base + value_relative,
                    end: line_base + value_relative + value.len(),
                },
            });
            correspondences.push(CorrespondenceRecord {
                target: CorrespondenceTarget::Token { id },
                pointer: token_pointer(id, "/name"),
                span: SourceSpan {
                    start: comment_base + name_start,
                    end: comment_base + name_end,
                },
            });
            if tokens
                .insert(
                    id,
                    Token {
                        id,
                        name: comment[name_start..name_end].to_owned(),
                        value: property_value,
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

fn parse_size_declaration(
    block: &CssBlock<'_>,
    property: &str,
    id: EntityId,
    suffix: &str,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<SizeIntent, AdapterError> {
    let declaration = css_declaration(block, property)?;
    let intent = parse_size(&declaration.value, &entity_pointer(id, suffix))?;
    entity_mapped(correspondences, id, suffix, declaration.span);
    Ok(intent)
}

fn css_px(
    block: &CssBlock<'_>,
    property: &str,
    id: EntityId,
    suffix: &str,
    correspondences: &mut Vec<CorrespondenceRecord>,
) -> Result<f64, AdapterError> {
    let declaration = css_declaration(block, property)?;
    let raw = declaration
        .value
        .strip_suffix("px")
        .ok_or_else(|| invalid_error(&entity_pointer(id, suffix), "CSS length must use px"))?;
    let span = SourceSpan {
        start: declaration.span.start,
        end: declaration.span.end - 2,
    };
    entity_mapped(correspondences, id, suffix, span);
    parse_number(raw, &entity_pointer(id, suffix))
}

fn size_css(value: &SizeIntent) -> String {
    match value {
        SizeIntent::Auto => "auto".to_owned(),
        SizeIntent::Fixed(value) => format!("{}px", number(*value)),
        SizeIntent::Fill => "100% /* nuif:fill */".to_owned(),
        SizeIntent::Intrinsic => "max-content /* nuif:intrinsic */".to_owned(),
        SizeIntent::Percentage(value) => format!("{}%", number(*value)),
        SizeIntent::MinContent => "min-content".to_owned(),
        SizeIntent::MaxContent => "max-content".to_owned(),
        SizeIntent::FitContent(value) => format!("fit-content({}px)", number(*value)),
    }
}

fn parse_size(value: &str, pointer: &str) -> Result<SizeIntent, AdapterError> {
    match value {
        "auto" => Ok(SizeIntent::Auto),
        "100% /* nuif:fill */" => Ok(SizeIntent::Fill),
        "max-content /* nuif:intrinsic */" => Ok(SizeIntent::Intrinsic),
        "min-content" => Ok(SizeIntent::MinContent),
        "max-content" => Ok(SizeIntent::MaxContent),
        _ if value.starts_with("fit-content(") && value.ends_with("px)") => {
            let raw = &value["fit-content(".len()..value.len() - "px)".len()];
            Ok(SizeIntent::FitContent(parse_number(raw, pointer)?))
        }
        _ if value.ends_with("px") => Ok(SizeIntent::Fixed(parse_number(
            &value[..value.len() - 2],
            pointer,
        )?)),
        _ if value.ends_with('%') => Ok(SizeIntent::Percentage(parse_number(
            &value[..value.len() - 1],
            pointer,
        )?)),
        _ => invalid(pointer, "unsupported CSS size intent"),
    }
}

fn fill_css(value: Option<Color>) -> String {
    value.map_or_else(
        || "transparent".to_owned(),
        |color| {
            format!(
                "rgb({} {} {} / {})",
                color.red, color.green, color.blue, color.alpha
            )
        },
    )
}

fn parse_fill(value: &str, id: EntityId) -> Result<Option<Color>, AdapterError> {
    if value == "transparent" {
        return Ok(None);
    }
    let raw = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| {
            invalid_error(
                &entity_pointer(id, "/authored/fill"),
                "fill must use rgb(r g b / a)",
            )
        })?;
    let (channels, alpha) = raw.split_once('/').ok_or_else(|| {
        invalid_error(
            &entity_pointer(id, "/authored/fill"),
            "fill lacks alpha separator",
        )
    })?;
    let channels = channels
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            invalid_error(&entity_pointer(id, "/authored/fill"), &error.to_string())
        })?;
    if channels.len() != 3 {
        return invalid(
            &entity_pointer(id, "/authored/fill"),
            "fill must contain three channels",
        );
    }
    let alpha = alpha.trim().parse::<f32>().map_err(|error| {
        invalid_error(&entity_pointer(id, "/authored/fill"), &error.to_string())
    })?;
    Ok(Some(Color {
        space: nuif_core::ColorSpace::Srgb,
        red: channels[0],
        green: channels[1],
        blue: channels[2],
        alpha,
    }))
}

fn v0_profile_issues(document: &Document) -> Vec<FidelityEntry> {
    let mut issues = validate(document)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| FidelityEntry {
            target: CorrespondenceTarget::Document { id: document.id },
            pointer: diagnostic.pointer.unwrap_or_else(|| "/".to_owned()),
            status: Fidelity::Unsupported {
                reason: diagnostic.message,
            },
        })
        .collect::<Vec<_>>();
    if !document.assets.is_empty() {
        issues.push(unsupported(
            CorrespondenceTarget::Document { id: document.id },
            "/assets".to_owned(),
            "HTML/CSS v0 does not encode the asset table",
        ));
    }
    for token in document.tokens.values() {
        if !safe_token_name(&token.name) {
            issues.push(unsupported(
                CorrespondenceTarget::Token { id: token.id },
                token_pointer(token.id, "/name"),
                "v0 token names use the safe CSS marker alphabet",
            ));
        }
        let supported = match &token.value {
            PropertyValue::Real(value) => value.is_finite(),
            PropertyValue::String(value) => safe_css_atom(value),
            _ => false,
        };
        if !supported {
            issues.push(unsupported(
                CorrespondenceTarget::Token { id: token.id },
                token_pointer(token.id, "/value"),
                "v0 CSS custom properties map finite real lengths and safe string atoms",
            ));
        }
    }
    for entity in document.entities.values() {
        if entity
            .authored
            .text
            .as_ref()
            .is_some_and(|text| text.font_asset.is_some())
        {
            issues.push(unsupported(
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/authored/text/font_asset"),
                "HTML/CSS v0 does not encode text font-asset bindings",
            ));
        }
        if entity.authored.text.is_some() && !entity.children.is_empty() {
            issues.push(unsupported(
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/authored/text"),
                "v0 HTML text-bearing entities cannot also contain mapped child entities",
            ));
        }
        if entity
            .authored
            .fill
            .is_some_and(|color| color.space != ColorSpace::Srgb)
        {
            issues.push(unsupported(
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/authored/fill"),
                "v0 CSS color serialization supports sRGB fills only",
            ));
        }
        for rule in &entity.authored.responsive {
            if rule.when.theme.is_some() || rule.width.is_some() || rule.height.is_some() {
                issues.push(unsupported(
                    CorrespondenceTarget::Entity { id: entity.id },
                    entity_pointer(entity.id, "/authored/responsive"),
                    "v0 responsive CSS maps width predicates, direction and gap only",
                ));
            }
            if rule.when.min_width.is_none() && rule.when.max_width.is_none() {
                issues.push(unsupported(
                    CorrespondenceTarget::Entity { id: entity.id },
                    entity_pointer(entity.id, "/authored/responsive"),
                    "responsive CSS requires a min-width or max-width predicate",
                ));
            }
        }
    }
    issues
}

fn target_fidelity(document: &Document) -> Vec<FidelityEntry> {
    let mut fidelity = Vec::new();
    for namespace in document.extensions.0.keys() {
        fidelity.push(FidelityEntry {
            target: CorrespondenceTarget::Document { id: document.id },
            pointer: format!("/extensions/{namespace}"),
            status: Fidelity::PreservedUnrenderable {
                namespace: namespace.clone(),
            },
        });
    }
    for entity in document.entities.values() {
        for namespace in entity.extensions.0.keys() {
            fidelity.push(FidelityEntry {
                target: CorrespondenceTarget::Entity { id: entity.id },
                pointer: entity_pointer(entity.id, &format!("/extensions/{namespace}")),
                status: Fidelity::PreservedUnrenderable {
                    namespace: namespace.clone(),
                },
            });
        }
        match &entity.kind {
            EntityKind::Unknown(unknown) => fidelity.push(FidelityEntry {
                target: CorrespondenceTarget::Entity { id: entity.id },
                pointer: entity_pointer(entity.id, "/kind"),
                status: Fidelity::PreservedUnrenderable {
                    namespace: unknown.namespace.clone(),
                },
            }),
            EntityKind::Shape(nuif_core::ShapeKind::Path) => fidelity.push(unsupported(
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/kind"),
                "HTML/CSS v0 preserves path identity but has no authored path geometry",
            )),
            EntityKind::Instance { .. } => fidelity.push(unsupported(
                CorrespondenceTarget::Entity { id: entity.id },
                entity_pointer(entity.id, "/kind"),
                "HTML/CSS v0 preserves the instance reference but does not materialize it",
            )),
            _ => {}
        }
    }
    fidelity
}

fn v0_report(
    document: &Document,
    fidelity: Vec<FidelityEntry>,
    correspondences: Vec<CorrespondenceRecord>,
    preserved: bool,
) -> Result<AdapterReport, AdapterError> {
    Ok(AdapterReport {
        schema_version: 1,
        source_format: V0_PROFILE_NAME.to_owned(),
        canonical_hash: Some(
            canonical_hash(document).map_err(|error| AdapterError::Canonical(error.to_string()))?,
        ),
        fidelity,
        correspondences,
        unmapped_source_preserved: preserved,
    })
}

fn validate_css(source: &str) -> Result<(), AdapterError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_css::LANGUAGE.into())
        .map_err(|error| AdapterError::CssSyntax(error.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| AdapterError::CssSyntax("parser returned no tree".to_owned()))?;
    if tree.root_node().has_error() {
        Err(AdapterError::CssSyntax(first_error(tree.root_node())))
    } else {
        Ok(())
    }
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

fn structural_issues(before: &Document, after: &Document) -> Vec<FidelityEntry> {
    let mut issues = Vec::new();
    if before.roots != after.roots {
        issues.push(unsupported(
            CorrespondenceTarget::Document { id: after.id },
            "/roots".to_owned(),
            "retentive synchronization cannot insert, remove or reorder roots",
        ));
    }
    for id in before
        .entities
        .keys()
        .chain(after.entities.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        match (before.entities.get(&id), after.entities.get(&id)) {
            (Some(left), Some(right)) if left.children != right.children => {
                issues.push(unsupported(
                    CorrespondenceTarget::Entity { id },
                    entity_pointer(id, "/children"),
                    "retentive synchronization cannot insert, remove or reorder children",
                ));
            }
            (None, Some(_)) | (Some(_), None) => issues.push(unsupported(
                CorrespondenceTarget::Entity { id },
                entity_pointer(id, ""),
                "retentive synchronization cannot insert or remove entities",
            )),
            _ => {}
        }
    }
    issues
}

fn correspondence_map(
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<Key, CorrespondenceRecord>, AdapterError> {
    let mut map = BTreeMap::new();
    for record in records {
        let key = (record.target.clone(), record.pointer.clone());
        if map.insert(key, record.clone()).is_some() {
            return Err(AdapterError::ProfileMarker(format!(
                "correspondence {} is duplicated",
                record.pointer
            )));
        }
    }
    Ok(map)
}

fn correspondence_values(
    source: &str,
    records: &[CorrespondenceRecord],
) -> Result<BTreeMap<Key, String>, AdapterError> {
    let mut values = BTreeMap::new();
    for record in records {
        let value = source
            .get(record.span.start..record.span.end)
            .ok_or_else(|| AdapterError::StaleSpan {
                pointer: record.pointer.clone(),
            })?
            .to_owned();
        let key = (record.target.clone(), record.pointer.clone());
        if values.insert(key, value).is_some() {
            return Err(AdapterError::ProfileMarker(format!(
                "correspondence {} is duplicated",
                record.pointer
            )));
        }
    }
    Ok(values)
}

fn unmapped_key_report(
    document: &Document,
    before: &BTreeSet<Key>,
    after: &BTreeSet<Key>,
) -> AdapterReport {
    let mut fidelity = before
        .symmetric_difference(after)
        .map(|(target, pointer)| {
            unsupported(
                target.clone(),
                pointer.clone(),
                "edit adds or removes a mapped source property",
            )
        })
        .collect::<Vec<_>>();
    if fidelity.is_empty() {
        fidelity.push(unsupported(
            CorrespondenceTarget::Document { id: document.id },
            "/".to_owned(),
            "source correspondence sets differ",
        ));
    }
    AdapterReport {
        schema_version: 1,
        source_format: V0_PROFILE_NAME.to_owned(),
        canonical_hash: None,
        fidelity,
        correspondences: Vec::new(),
        unmapped_source_preserved: false,
    }
}

fn json_attribute<T: Serialize>(value: &T) -> Result<String, AdapterError> {
    serde_json::to_string(value)
        .map(|value| escape_html(&value))
        .map_err(|error| AdapterError::InvalidValue {
            pointer: "/".to_owned(),
            reason: error.to_string(),
        })
}

fn parse_json<T: DeserializeOwned>(value: &str, pointer: &str) -> Result<T, AdapterError> {
    serde_json::from_str(value).map_err(|error| invalid_error(pointer, &error.to_string()))
}

fn token_type(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Real(_) => "real",
        PropertyValue::String(_) => "string",
        _ => "unsupported",
    }
}

fn token_css_value(value: &PropertyValue) -> Result<String, AdapterError> {
    match value {
        PropertyValue::Real(value) => Ok(format!("{}px", number(*value))),
        PropertyValue::String(value) => Ok(value.clone()),
        _ => Err(AdapterError::InvalidValue {
            pointer: "/tokens/*/value".to_owned(),
            reason: "unsupported v0 token value".to_owned(),
        }),
    }
}

fn layout_family_css(value: LayoutFamily) -> &'static str {
    match value {
        LayoutFamily::Freeform => "freeform",
        LayoutFamily::Stack => "stack",
        LayoutFamily::Flex => "flex",
        LayoutFamily::Grid => "grid",
        LayoutFamily::Constraint => "constraint",
    }
}

fn parse_layout_family(value: &str, id: EntityId) -> Result<LayoutFamily, AdapterError> {
    match value {
        "freeform" => Ok(LayoutFamily::Freeform),
        "stack" => Ok(LayoutFamily::Stack),
        "flex" => Ok(LayoutFamily::Flex),
        "grid" => Ok(LayoutFamily::Grid),
        "constraint" => Ok(LayoutFamily::Constraint),
        _ => invalid(
            &entity_pointer(id, "/authored/layout/family"),
            "unknown layout family",
        ),
    }
}

fn direction_css(value: FlowDirection) -> &'static str {
    match value {
        FlowDirection::Row => "row",
        FlowDirection::Column => "column",
    }
}

fn parse_direction(value: &str, id: EntityId) -> Result<FlowDirection, AdapterError> {
    match value {
        "row" => Ok(FlowDirection::Row),
        "column" => Ok(FlowDirection::Column),
        _ => invalid(
            &entity_pointer(id, "/authored/layout/direction"),
            "unknown flow direction",
        ),
    }
}

fn align_css(value: Align) -> &'static str {
    match value {
        Align::Start => "flex-start",
        Align::Center => "center",
        Align::End => "flex-end",
        Align::Stretch => "stretch",
    }
}

fn parse_align(value: &str, id: EntityId) -> Result<Align, AdapterError> {
    match value {
        "flex-start" => Ok(Align::Start),
        "center" => Ok(Align::Center),
        "flex-end" => Ok(Align::End),
        "stretch" => Ok(Align::Stretch),
        _ => invalid(
            &entity_pointer(id, "/authored/layout/align"),
            "unknown alignment",
        ),
    }
}

fn required_attribute<'a>(
    attributes: &'a BTreeMap<String, Attribute>,
    name: &str,
    target: &str,
) -> Result<&'a Attribute, AdapterError> {
    attributes
        .get(name)
        .ok_or_else(|| AdapterError::ProfileMarker(format!("{target} lacks {name}")))
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
        .and_then(|value| {
            if value.is_finite() {
                Ok(value)
            } else {
                invalid(pointer, "number must be finite")
            }
        })
}

fn document_mapped(
    records: &mut Vec<CorrespondenceRecord>,
    id: EntityId,
    pointer: &str,
    span: SourceSpan,
) {
    records.push(CorrespondenceRecord {
        target: CorrespondenceTarget::Document { id },
        pointer: pointer.to_owned(),
        span,
    });
}

fn entity_mapped(
    records: &mut Vec<CorrespondenceRecord>,
    id: EntityId,
    suffix: &str,
    span: SourceSpan,
) {
    records.push(CorrespondenceRecord {
        target: CorrespondenceTarget::Entity { id },
        pointer: entity_pointer(id, suffix),
        span,
    });
}

fn safe_token_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn safe_css_atom(value: &str) -> bool {
    !value.is_empty()
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b';' | b'{' | b'}'))
        && !value.contains("/*")
        && !value.contains("*/")
}

fn unsupported(target: CorrespondenceTarget, pointer: String, reason: &str) -> FidelityEntry {
    FidelityEntry {
        target,
        pointer,
        status: Fidelity::Unsupported {
            reason: reason.to_owned(),
        },
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_fixture;

    #[test]
    fn every_size_intent_has_an_injective_css_encoding() {
        for intent in [
            SizeIntent::Auto,
            SizeIntent::Fixed(12.5),
            SizeIntent::Fill,
            SizeIntent::Intrinsic,
            SizeIntent::Percentage(62.5),
            SizeIntent::MinContent,
            SizeIntent::MaxContent,
            SizeIntent::FitContent(91.25),
        ] {
            assert_eq!(parse_size(&size_css(&intent), "/size").unwrap(), intent);
        }
    }

    #[test]
    fn declared_v0_subset_exports_and_imports_exactly() {
        let mut document = profile_fixture();
        document
            .entities
            .values_mut()
            .find_map(|entity| entity.authored.text.as_mut())
            .unwrap()
            .content
            .push_str(" \n");
        let exported = export_v0_document(&document).unwrap();
        let imported = import_v0_source(&exported.source).unwrap();
        assert_eq!(imported.document, document);
        for record in &imported.retentive.report.correspondences {
            assert!(record.span.start <= record.span.end);
            assert!(record.span.end <= exported.source.len());
            assert!(exported.source.is_char_boundary(record.span.start));
            assert!(exported.source.is_char_boundary(record.span.end));
        }
    }

    #[test]
    fn unsupported_token_value_is_reported_before_rendering() {
        let mut document = profile_fixture();
        document.tokens.values_mut().next().unwrap().value = PropertyValue::Boolean(true);
        let Err(AdapterError::UnsupportedProfile { report, .. }) = export_v0_document(&document)
        else {
            panic!("boolean token unexpectedly entered the CSS profile");
        };
        assert!(report.fidelity.iter().any(|entry| {
            entry.pointer.ends_with("/value")
                && matches!(entry.status, Fidelity::Unsupported { .. })
        }));
    }

    #[test]
    fn orphaned_reserved_css_marker_is_rejected() {
        let exported = export_v0_document(&profile_fixture()).unwrap();
        let source = exported.source.replace(
            "  </style>",
            "    /* nuif-entity-begin:ffffffffffffffffffffffffffffffff */\n  </style>",
        );
        assert!(matches!(
            import_v0_source(&source),
            Err(AdapterError::InvalidValue { .. })
        ));
    }
}
