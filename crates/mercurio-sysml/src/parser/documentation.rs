//! Authoring-AST adapters for imported Comment, Documentation and TextualRepresentation rules.
//! Grammar recognition/assignments are generated; raw body/locale storage and
//! anonymous authoring identity remain explicit representation adapters.
use mercurio_foundation::language_contracts::{
    ast::{GenericUsageDecl, QualifiedName, SourceSpan},
    diagnostics::Diagnostic,
    lexer::Token,
};
use std::collections::BTreeMap;

pub(super) fn parse(
    tokens: &[Token],
    kerml: bool,
    docs: Vec<String>,
    modifiers: Vec<String>,
) -> Result<(usize, GenericUsageDecl), Diagnostic> {
    parse_kind(tokens, kerml, docs, modifiers, "Documentation")
}

pub(super) fn parse_textual(
    tokens: &[Token],
    kerml: bool,
    docs: Vec<String>,
    modifiers: Vec<String>,
) -> Result<(usize, GenericUsageDecl), Diagnostic> {
    parse_kind(tokens, kerml, docs, modifiers, "TextualRepresentation")
}

pub(super) fn parse_comment(
    tokens: &[Token],
    kerml: bool,
    docs: Vec<String>,
    modifiers: Vec<String>,
) -> Result<(usize, GenericUsageDecl), Diagnostic> {
    parse_kind(tokens, kerml, docs, modifiers, "Comment")
}

fn parse_kind(
    tokens: &[Token],
    kerml: bool,
    docs: Vec<String>,
    mut modifiers: Vec<String>,
    kind: &str,
) -> Result<(usize, GenericUsageDecl), Diagnostic> {
    let fail = |message: String| Diagnostic::new(message, tokens.first().map(|t| t.span.clone()));
    let textual = kind == "TextualRepresentation";
    let grammar = if kerml {
        "org.omg.kerml.xtext.KerML"
    } else {
        "org.omg.sysml.xtext.SysML"
    };
    let rule = format!("{grammar}::{kind}");
    let matched = crate::xtext_fragment::execute_in_context(&rule, tokens, kind, kerml)
        .map_err(&fail)?
        .ok_or_else(|| fail("expected annotation declaration".into()))?;
    let end = matched
        .consumed
        .checked_sub(1)
        .and_then(|i| tokens.get(i))
        .ok_or_else(|| fail("annotation consumed no source".into()))?;
    let start = tokens
        .first()
        .ok_or_else(|| fail("missing annotation source".into()))?;
    let body = matched
        .fields
        .get("body")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.strip_prefix("/*"))
        .and_then(|s| s.strip_suffix("*/"))
        .ok_or_else(|| fail("invalid generated annotation body".into()))?;
    let mut metadata_properties = BTreeMap::from([("body".into(), body.into())]);
    for field in ["locale", "language"] {
        if let Some(value) = matched.fields.get(field) {
            let raw = value
                .as_str()
                .and_then(|s| s.strip_prefix('"'))
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| fail(format!("invalid generated {field}")))?;
            metadata_properties.insert(field.into(), raw.into());
        }
    }
    if let Some(short) = matched
        .fields
        .get("declared_short_name")
        .and_then(serde_json::Value::as_str)
    {
        modifiers.push(format!("short_name={short}"));
    }
    // Grammar constructs typed Annotation children; this adapter only projects
    // unresolved references into the authoring AST. Native lowering resolves them.
    let mut annotation_targets = Vec::new();
    for child in matched
        .children
        .get("owned_relationship")
        .into_iter()
        .flatten()
    {
        if child.object_kind.as_deref() != Some("Annotation") {
            return Err(fail("unexpected generated annotation child".into()));
        }
        let links = child
            .links
            .get("annotated_element")
            .ok_or_else(|| fail("missing generated annotation target".into()))?;
        if links.len() != 1 {
            return Err(fail("invalid annotation target cardinality".into()));
        }
        let reference = &links[0];
        annotation_targets.push(QualifiedName {
            segments: crate::xtext_fragment::reference_name_segments(&reference.spelling, kerml)
                .map_err(&fail)?,
            span: reference
                .span
                .as_deref()
                .cloned()
                .ok_or_else(|| fail("missing annotation target span".into()))?,
        });
    }
    let name = matched
        .fields
        .get("declared_name")
        .and_then(serde_json::Value::as_str);
    Ok((
        matched.consumed,
        GenericUsageDecl {
            keyword: match kind {
                "TextualRepresentation" => "rep",
                "Documentation" => "doc",
                _ => "comment",
            }
            .into(),
            name: name.map(str::to_owned).unwrap_or_else(|| {
                if textual {
                    format!("rep_{}_{}", start.span.start_line, start.span.start_col)
                } else {
                    "comment".into()
                }
            }),
            is_implicit_name: name.is_none(),
            reference_target: annotation_targets.first().cloned(),
            annotation_targets,
            ty: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties,
            multiplicity: None,
            expression: None,
            additional_types: Vec::new(),
            specializes: Vec::new(),
            subsets: Vec::new(),
            redefines: Vec::new(),
            body_members: Vec::new(),
            comments: Vec::new(),
            docs,
            modifiers,
            span: SourceSpan {
                end_line: end.span.end_line,
                end_col: end.span.end_col,
                ..start.span.clone()
            },
        },
    ))
}
