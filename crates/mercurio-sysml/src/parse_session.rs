use std::collections::HashMap;

use mercurio_foundation::language_contracts::{
    Declaration, GenericDefinitionDecl, GenericUsageDecl, PackageDecl, ParseSessionError,
    ParseSessionStatus, ParseSnapshot, ParsedModule as SysmlModule, SourceSpan, TextEdit,
    TextRange,
};
use mercurio_foundation::outline::{EditorOutlineNodeDto, build_editor_outline};

use crate::parse_sysml_recovering;

#[derive(Debug, Clone)]
pub struct SysmlParseSession {
    source_name: String,
    text: String,
    revision: u64,
    snapshot: ParseSnapshot,
}

impl SysmlParseSession {
    pub fn open(source_name: impl Into<String>, text: impl Into<String>) -> Self {
        let source_name = source_name.into();
        let text = text.into();
        let revision = 0;
        let snapshot = parse_snapshot(&source_name, revision, &text, Vec::new(), None);
        Self {
            source_name,
            text,
            revision,
            snapshot,
        }
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn snapshot(&self) -> &ParseSnapshot {
        &self.snapshot
    }

    pub fn syntax_outline(&self) -> Vec<EditorOutlineNodeDto> {
        build_sysml_syntax_outline(&self.source_name, &self.snapshot.module)
    }

    pub fn apply_edits(
        &mut self,
        base_revision: u64,
        edits: &[TextEdit],
    ) -> Result<&ParseSnapshot, ParseSessionError> {
        if base_revision != self.revision {
            return Err(ParseSessionError::RevisionMismatch {
                expected: self.revision,
                actual: base_revision,
            });
        }

        if edits.is_empty() {
            self.snapshot.changed_ranges.clear();
            return Ok(&self.snapshot);
        }

        let changed_ranges = apply_text_edits(&mut self.text, edits)?;
        self.revision += 1;
        let previous_module = self.snapshot.module.clone();
        self.snapshot = parse_snapshot(
            &self.source_name,
            self.revision,
            &self.text,
            changed_ranges,
            Some(&previous_module),
        );
        Ok(&self.snapshot)
    }
}

pub fn build_sysml_syntax_outline(
    source_name: &str,
    module: &SysmlModule,
) -> Vec<EditorOutlineNodeDto> {
    let element_index = HashMap::new();
    build_editor_outline(source_name, module, &element_index)
}

fn parse_snapshot(
    source_name: &str,
    revision: u64,
    text: &str,
    changed_ranges: Vec<TextRange>,
    fallback_module: Option<&SysmlModule>,
) -> ParseSnapshot {
    match parse_sysml_recovering(text) {
        Ok(report) => {
            let status = if report.diagnostics.is_empty() {
                ParseSessionStatus::Ok
            } else {
                ParseSessionStatus::Partial
            };
            let module = if !report.diagnostics.is_empty() && module_is_empty(&report.module) {
                fallback_module
                    .cloned()
                    .unwrap_or_else(|| report.module.clone())
            } else {
                report.module
            };
            ParseSnapshot {
                source_name: source_name.to_string(),
                revision,
                status,
                changed_declaration_ranges: changed_declaration_ranges(
                    text,
                    &module,
                    &changed_ranges,
                ),
                module,
                diagnostics: report.diagnostics,
                changed_ranges,
            }
        }
        Err(diagnostic) => {
            let (status, module) = fallback_module
                .map(|module| (ParseSessionStatus::Partial, module.clone()))
                .unwrap_or_else(|| (ParseSessionStatus::Failed, SysmlModule::default()));
            ParseSnapshot {
                source_name: source_name.to_string(),
                revision,
                status,
                changed_declaration_ranges: changed_declaration_ranges(
                    text,
                    &module,
                    &changed_ranges,
                ),
                module,
                diagnostics: vec![diagnostic],
                changed_ranges,
            }
        }
    }
}

fn module_is_empty(module: &SysmlModule) -> bool {
    module.package.is_none()
        && module.members.is_empty()
        && module.imports.is_empty()
        && module.definitions.is_empty()
}

fn changed_declaration_ranges(
    text: &str,
    module: &SysmlModule,
    changed_ranges: &[TextRange],
) -> Vec<TextRange> {
    if changed_ranges.is_empty() {
        return Vec::new();
    }

    let line_starts = line_start_offsets(text);
    let mut ranges = Vec::new();
    if let Some(package) = module.package.as_ref() {
        collect_changed_package_ranges(
            package,
            changed_ranges,
            &line_starts,
            text.len(),
            &mut ranges,
        );
    } else {
        collect_changed_declaration_ranges(
            &module.members,
            changed_ranges,
            &line_starts,
            text.len(),
            &mut ranges,
        );
    }

    ranges.sort_by_key(|range| (range.start_byte, range.end_byte));
    ranges.dedup();
    ranges
}

fn collect_changed_package_ranges(
    package: &PackageDecl,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    let before = ranges.len();
    collect_changed_declaration_ranges(
        &package.members,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
    );
    push_span_if_changed_outside_children(
        &package.span,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
        before,
    );
    ranges.len() > before
}

fn collect_changed_declaration_ranges(
    declarations: &[Declaration],
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    let before = ranges.len();
    for declaration in declarations {
        collect_changed_declaration_range(
            declaration,
            changed_ranges,
            line_starts,
            text_len,
            ranges,
        );
    }
    ranges.len() > before
}

fn collect_changed_declaration_range(
    declaration: &Declaration,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    match declaration {
        Declaration::Package(package) => {
            collect_changed_package_ranges(package, changed_ranges, line_starts, text_len, ranges)
        }
        Declaration::GenericDefinition(definition) => collect_changed_definition_range(
            definition,
            changed_ranges,
            line_starts,
            text_len,
            ranges,
        ),
        Declaration::GenericUsage(usage) => {
            collect_changed_usage_range(usage, changed_ranges, line_starts, text_len, ranges)
        }
        Declaration::Import(import) => {
            push_span_if_changed(&import.span, changed_ranges, line_starts, text_len, ranges)
        }
        Declaration::Alias(alias) => {
            push_span_if_changed(&alias.span, changed_ranges, line_starts, text_len, ranges)
        }
    }
}

fn collect_changed_definition_range(
    definition: &GenericDefinitionDecl,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    let before = ranges.len();
    collect_changed_declaration_ranges(
        &definition.members,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
    );
    push_span_if_changed_outside_children(
        &definition.span,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
        before,
    );
    ranges.len() > before
}

fn collect_changed_usage_range(
    usage: &GenericUsageDecl,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    let before = ranges.len();
    collect_changed_declaration_ranges(
        &usage.body_members,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
    );
    push_span_if_changed_outside_children(
        &usage.span,
        changed_ranges,
        line_starts,
        text_len,
        ranges,
        before,
    );
    ranges.len() > before
}

fn push_span_if_changed(
    span: &SourceSpan,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
) -> bool {
    let Some(range) = span_to_text_range(span, line_starts, text_len) else {
        return false;
    };
    if changed_ranges
        .iter()
        .any(|changed_range| text_ranges_touch(range, *changed_range))
    {
        ranges.push(range);
        true
    } else {
        false
    }
}

fn push_span_if_changed_outside_children(
    span: &SourceSpan,
    changed_ranges: &[TextRange],
    line_starts: &[usize],
    text_len: usize,
    ranges: &mut Vec<TextRange>,
    child_start_index: usize,
) -> bool {
    let Some(range) = span_to_text_range(span, line_starts, text_len) else {
        return false;
    };
    let child_ranges = &ranges[child_start_index..];
    if changed_ranges.iter().any(|changed_range| {
        text_ranges_touch(range, *changed_range)
            && !child_ranges
                .iter()
                .any(|child_range| text_ranges_touch(*child_range, *changed_range))
    }) {
        ranges.push(range);
        true
    } else {
        false
    }
}

fn span_to_text_range(
    span: &SourceSpan,
    line_starts: &[usize],
    text_len: usize,
) -> Option<TextRange> {
    let start_line = span.start_line.checked_sub(1)?;
    let end_line = span.end_line.checked_sub(1)?;
    let start = line_starts
        .get(start_line)
        .copied()
        .unwrap_or(text_len)
        .saturating_add(span.start_col.saturating_sub(1))
        .min(text_len);
    let end = line_starts
        .get(end_line)
        .copied()
        .unwrap_or(text_len)
        .saturating_add(span.end_col)
        .min(text_len);
    (start <= end).then_some(TextRange::new(start, end))
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn text_ranges_touch(left: TextRange, right: TextRange) -> bool {
    if left.is_empty() {
        right.start_byte <= left.start_byte && left.start_byte <= right.end_byte
    } else if right.is_empty() {
        left.start_byte <= right.start_byte && right.start_byte <= left.end_byte
    } else {
        left.start_byte < right.end_byte && right.start_byte < left.end_byte
    }
}

fn apply_text_edits(
    text: &mut String,
    edits: &[TextEdit],
) -> Result<Vec<TextRange>, ParseSessionError> {
    let mut sorted = edits.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|edit| (edit.range.start_byte, edit.range.end_byte));

    let text_len = text.len();
    let mut previous_range: Option<TextRange> = None;
    let mut changed_ranges = Vec::with_capacity(sorted.len());
    let mut byte_delta: isize = 0;

    for edit in &sorted {
        let range = edit.range;
        if range.start_byte > range.end_byte || range.end_byte > text_len {
            return Err(ParseSessionError::InvalidEditRange { range, text_len });
        }
        if !text.is_char_boundary(range.start_byte) {
            return Err(ParseSessionError::NonBoundaryEdit {
                offset: range.start_byte,
            });
        }
        if !text.is_char_boundary(range.end_byte) {
            return Err(ParseSessionError::NonBoundaryEdit {
                offset: range.end_byte,
            });
        }
        if let Some(previous) = previous_range
            && previous.end_byte > range.start_byte
        {
            return Err(ParseSessionError::OverlappingEditRanges {
                previous,
                next: range,
            });
        }

        let new_start = (range.start_byte as isize + byte_delta) as usize;
        let new_end = new_start + edit.replacement.len();
        changed_ranges.push(TextRange::new(new_start, new_end));
        byte_delta += edit.replacement.len() as isize
            - (range.end_byte.saturating_sub(range.start_byte) as isize);
        previous_range = Some(range);
    }

    for edit in sorted.iter().rev() {
        text.replace_range(
            edit.range.start_byte..edit.range.end_byte,
            &edit.replacement,
        );
    }

    Ok(changed_ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_sysml_recovering;

    #[test]
    fn open_builds_parse_snapshot_and_syntax_outline() {
        let session = SysmlParseSession::open(
            "model.sysml",
            "package Demo { part def Vehicle; part vehicle : Vehicle; }",
        );

        assert_eq!(session.revision(), 0);
        assert_eq!(session.snapshot().status, ParseSessionStatus::Ok);
        assert_eq!(session.syntax_outline().len(), 1);
        assert_eq!(session.syntax_outline()[0].label, "Demo");
    }

    #[test]
    fn edit_snapshot_matches_full_recovering_parse() {
        let mut session = SysmlParseSession::open(
            "model.sysml",
            "package Demo { part def Vehicle; part vehicle : Vehicle; }",
        );
        let start = session.text().find("Vehicle").unwrap();
        let end = start + "Vehicle".len();
        let snapshot = session
            .apply_edits(0, &[TextEdit::new(TextRange::new(start, end), "Car")])
            .unwrap()
            .clone();

        let full_report = parse_sysml_recovering(session.text()).unwrap();
        assert_eq!(session.revision(), 1);
        assert_eq!(snapshot.module, full_report.module);
        assert_eq!(
            snapshot.changed_ranges,
            vec![TextRange::new(start, start + 3)]
        );
        let declaration_start = "package Demo { ".len();
        let declaration_end = source_first_semicolon(session.text());
        assert_eq!(
            snapshot.changed_declaration_ranges,
            vec![TextRange::new(declaration_start, declaration_end)]
        );
    }

    #[test]
    fn multi_edit_changed_ranges_are_reported_in_post_edit_coordinates() {
        let source = "package Demo { part def Vehicle; part vehicle : Vehicle; }";
        let mut session = SysmlParseSession::open("model.sysml", source);
        let package_start = source.find("Demo").unwrap();
        let usage_start = source.find("vehicle").unwrap();

        let snapshot = session
            .apply_edits(
                0,
                &[
                    TextEdit::new(
                        TextRange::new(package_start, package_start + "Demo".len()),
                        "Fleet",
                    ),
                    TextEdit::new(
                        TextRange::new(usage_start, usage_start + "vehicle".len()),
                        "car",
                    ),
                ],
            )
            .unwrap()
            .clone();

        assert_eq!(
            session.text(),
            "package Fleet { part def Vehicle; part car : Vehicle; }"
        );
        assert_eq!(snapshot.status, ParseSessionStatus::Ok);
        assert_eq!(
            snapshot.changed_ranges,
            vec![
                TextRange::new(package_start, package_start + "Fleet".len()),
                TextRange::new(usage_start + 1, usage_start + 1 + "car".len()),
            ]
        );
        assert_eq!(
            snapshot.changed_declaration_ranges,
            vec![
                TextRange::new(0, session.text().len()),
                TextRange::new(
                    session.text().find("part car").unwrap(),
                    session.text().rfind(";").unwrap() + 1
                ),
            ]
        );
    }

    #[test]
    fn edits_after_non_ascii_text_use_utf8_byte_offsets() {
        let source = "package Demo { doc /* Café */ part def Vehicle; }";
        let mut session = SysmlParseSession::open("model.sysml", source);
        let start = source.find("Vehicle").unwrap();
        let end = start + "Vehicle".len();

        assert!(start > source[..start].chars().count());

        let snapshot = session
            .apply_edits(0, &[TextEdit::new(TextRange::new(start, end), "Car")])
            .unwrap()
            .clone();

        assert_eq!(
            session.text(),
            "package Demo { doc /* Café */ part def Car; }"
        );
        assert_eq!(snapshot.status, ParseSessionStatus::Ok);
        assert_eq!(
            snapshot.changed_ranges,
            vec![TextRange::new(start, start + "Car".len())]
        );

        let non_boundary = source.find('é').unwrap() + 1;
        let err = SysmlParseSession::open("model.sysml", source)
            .apply_edits(
                0,
                &[TextEdit::new(
                    TextRange::new(non_boundary, non_boundary),
                    "x",
                )],
            )
            .unwrap_err();
        assert!(matches!(err, ParseSessionError::NonBoundaryEdit { .. }));
    }

    #[test]
    fn stale_revision_is_rejected_without_mutating_session() {
        let mut session = SysmlParseSession::open("model.sysml", "package Demo { }");
        let result = session.apply_edits(7, &[TextEdit::new(TextRange::new(8, 12), "Other")]);

        assert!(matches!(
            result,
            Err(ParseSessionError::RevisionMismatch {
                expected: 0,
                actual: 7
            })
        ));
        assert_eq!(session.revision(), 0);
        assert_eq!(session.text(), "package Demo { }");
    }

    #[test]
    fn empty_recovering_parse_after_edit_keeps_previous_partial_model() {
        let source = "package Demo { part def Vehicle; }";
        let mut session = SysmlParseSession::open("model.sysml", source);
        let previous_module = session.snapshot().module.clone();
        let close_brace = source.rfind('}').unwrap();

        let snapshot = session
            .apply_edits(
                0,
                &[TextEdit::new(
                    TextRange::new(close_brace, close_brace + 1),
                    "",
                )],
            )
            .unwrap()
            .clone();

        assert_eq!(session.revision(), 1);
        assert_eq!(snapshot.status, ParseSessionStatus::Partial);
        assert_eq!(snapshot.module, previous_module);
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert_eq!(session.syntax_outline().len(), 1);
        assert_eq!(session.syntax_outline()[0].label, "Demo");
    }

    #[test]
    fn hard_parse_failure_on_open_has_no_fallback_model() {
        let session = SysmlParseSession::open("model.sysml", "package Demo { doc /*");

        assert_eq!(session.snapshot().status, ParseSessionStatus::Failed);
        assert!(session.snapshot().module.members.is_empty());
        assert!(session.syntax_outline().is_empty());
    }

    #[test]
    fn empty_recovering_parse_on_open_has_no_fallback_model() {
        let session = SysmlParseSession::open("model.sysml", "package Demo {");

        assert_eq!(session.snapshot().status, ParseSessionStatus::Partial);
        assert!(session.snapshot().module.members.is_empty());
        assert!(session.syntax_outline().is_empty());
    }

    #[test]
    fn semantic_errors_do_not_block_syntax_outline() {
        let session = SysmlParseSession::open(
            "model.sysml",
            "package Demo { part missing : MissingType; }",
        );
        let outline = session.syntax_outline();

        assert_eq!(session.snapshot().status, ParseSessionStatus::Ok);
        assert_eq!(outline.len(), 1);
        assert_eq!(outline[0].label, "Demo");
        assert_eq!(outline[0].children.len(), 1);
        assert_eq!(outline[0].children[0].label, "missing");
    }

    #[test]
    fn invalid_and_overlapping_edit_ranges_are_rejected() {
        let mut session = SysmlParseSession::open("model.sysml", "package Demo { }");

        let invalid = session.apply_edits(
            0,
            &[TextEdit::new(TextRange::new(0, 100), "package Other { }")],
        );
        assert!(matches!(
            invalid,
            Err(ParseSessionError::InvalidEditRange { .. })
        ));

        let overlapping = session.apply_edits(
            0,
            &[
                TextEdit::new(TextRange::new(8, 12), "Other"),
                TextEdit::new(TextRange::new(10, 14), "Thing"),
            ],
        );
        assert!(matches!(
            overlapping,
            Err(ParseSessionError::OverlappingEditRanges { .. })
        ));
    }

    fn source_first_semicolon(text: &str) -> usize {
        text.find(';').unwrap() + 1
    }
}
