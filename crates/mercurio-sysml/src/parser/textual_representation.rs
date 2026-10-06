//! Shared KerML/SysML TextualRepresentation grammar. Keep raw text for authoring.
use mercurio_foundation::language_contracts::{
    ast::GenericUsageDecl, diagnostics::Diagnostic, lexer::Token,
};

pub(crate) fn parse(
    tokens: &mut [Token],
    index: &mut usize,
    start: Token,
    docs: Vec<String>,
    modifiers: Vec<String>,
    kerml: bool,
) -> Result<GenericUsageDecl, Diagnostic> {
    let begin = index.checked_sub(1).ok_or_else(|| {
        Diagnostic::new(
            "missing textual representation keyword",
            Some(start.span.clone()),
        )
    })?;
    let (consumed, declaration) =
        super::documentation::parse_textual(&tokens[begin..], kerml, docs, modifiers)?;
    *index = begin + consumed;
    Ok(declaration)
}

/// ElementUtil.processCommentBody in the pinned Pilot strips leading whitespace
/// and multiline star decoration, but retains trailing spaces and body newlines.
pub(crate) fn semantic_body(raw: &str) -> String {
    let body =
        raw.trim_start_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000B}' | '\u{000C}'));
    let mut lines = body
        .split('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .collect::<Vec<_>>();
    while lines.len() > 1 && lines.last() == Some(&"") {
        lines.pop();
    }
    if lines.len() == 1 {
        return lines[0].to_string();
    }
    lines
        .into_iter()
        .map(|line| {
            let line = line
                .trim_start_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\u{000B}' | '\u{000C}'));
            line.strip_prefix("* ")
                .or_else(|| line.strip_prefix('*'))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// STRING_VALUE escape alphabet and ElementUtil.unescapeString semantics.
/// The AST keeps its raw spelling for lossless authoring.
pub(crate) fn language_value(raw: &str) -> Result<String, Diagnostic> {
    let mut value = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        value.push(match chars.next() {
            Some('b') => '\u{0008}',
            Some('t') => '\t',
            Some('n') => '\n',
            Some('f') => '\u{000c}',
            Some('r') => '\r',
            Some('"') => '"',
            Some('\'') => '\'',
            Some('\\') => '\\',
            _ => return Err(Diagnostic::new("invalid escape in language string", None)),
        });
    }
    Ok(value)
}
