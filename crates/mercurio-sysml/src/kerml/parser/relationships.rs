//! Explicit relationship productions in pinned KerML.xtext (2026-08).
use super::*;
use crate::language_frontend::lowering::relationship_declarations::{Endpoints, Operand, store};

impl Parser {
    fn parse_relationship_name(&mut self) -> Result<String, Diagnostic> {
        let (consumed, name) = crate::xtext_fragment::declared_name(&self.tokens[self.index..], true)?;
        self.index += consumed;
        Ok(name)
    }

    pub(super) fn parse_relationship_operand(&mut self) -> Result<Operand<QualifiedName>, Diagnostic> {
        let (consumed, names) = crate::xtext_fragment::feature_chain_names(&self.tokens[self.index..], true)?;
        // Keep each imported QualifiedName distinct: `::` belongs to a name,
        // while `.` changes the scope to the preceding resolved Feature.
        let steps = names;
        self.index += consumed;
        Ok(Operand { steps, type_ref: None })
    }

    pub(super) fn parse_relationship(
        &mut self,
        docs: Vec<String>,
        mut modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.current().clone();
        let first = token_identifier(&start).to_string();
        self.advance();
        let mut name = None;
        let keyword = match first.as_str() {
            "disjoining" => {
                if matches!(self.peek_kind(), TokenKind::LAngle) {
                    self.advance();
                    let short = self.parse_relationship_name()?;
                    self.expect(TokenKind::RAngle, "expected `>` after short name")?;
                    modifiers.push(format!("short_name={short}"));
                }
                if !matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "disjoint") {
                    name = Some(self.parse_relationship_name()?);
                }
                self.expect_identifier_named("disjoint", "expected `disjoint`")?;
                first.clone()
            }
            "disjoint" => "disjoining".to_string(),
            "specialization" | "conjugation" | "inverting" => {
                if matches!(self.peek_kind(), TokenKind::LAngle) {
                    self.advance();
                    let short = self.parse_relationship_name()?;
                    self.expect(TokenKind::RAngle, "expected `>` after short name")?;
                    modifiers.push(format!("short_name={short}"));
                }
                let is_role = |word: &str| match first.as_str() {
                    "specialization" => matches!(
                        word,
                        "subtype" | "subclassifier" | "typing" | "subset" | "redefinition"
                    ),
                    "conjugation" => word == "conjugate",
                    _ => word == "inverse",
                };
                if !matches!(self.peek_kind(), TokenKind::Identifier(word) if is_role(word)) {
                    name = Some(self.parse_relationship_name()?);
                }
                let role = self.expect_identifier("expected relationship role")?;
                if !is_role(&role) {
                    return Err(self.error_here("invalid relationship role"));
                }
                role
            }
            "featuring" | "dependency" => {
                let marker = if first == "featuring" { "of" } else { "from" };
                let identified = matches!(self.peek_kind(), TokenKind::LAngle)
                    || matches!(self.peek_kind(), TokenKind::Identifier(word) if word == marker)
                    || matches!(self.next_kind(), Some(TokenKind::Identifier(word)) if word == marker);
                if identified {
                    if matches!(self.peek_kind(), TokenKind::LAngle) {
                        self.advance();
                        let short = self.parse_relationship_name()?;
                        self.expect(TokenKind::RAngle, "expected `>` after short name")?;
                        modifiers.push(format!("short_name={short}"));
                    }
                    if !matches!(self.peek_kind(), TokenKind::Identifier(word) if word == marker) {
                        name = Some(self.parse_relationship_name()?);
                    }
                    self.expect_identifier_named(marker, "expected relationship operand marker")?;
                }
                first.clone()
            }
            _ => first.clone(),
        };
        let mut sources = vec![self.parse_relationship_operand()?];
        if keyword == "dependency" {
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                sources.push(self.parse_relationship_operand()?);
            }
        }
        match keyword.as_str() {
            "subtype" | "subclassifier" | "subset" => {
                let word = if keyword == "subset" {
                    "subsets"
                } else {
                    "specializes"
                };
                if matches!(self.peek_kind(), TokenKind::Specializes) {
                    self.advance();
                } else {
                    self.expect_identifier_named(
                        word,
                        "expected relationship specialization separator",
                    )?;
                }
            }
            "redefinition" => {
                if matches!(self.peek_kind(), TokenKind::Redefines) {
                    self.advance();
                } else {
                    self.expect_identifier_named("redefines", "expected `redefines` or `:>>`")?;
                }
            }
            "typing" => {
                if matches!(self.peek_kind(), TokenKind::Colon) {
                    self.advance();
                } else {
                    self.expect_identifier_named("typed", "expected `typed by` or `:`")?;
                    self.expect_identifier_named("by", "expected `by`")?;
                }
            }
            "conjugate" => {
                if matches!(self.peek_kind(), TokenKind::Tilde) {
                    self.advance();
                } else {
                    self.expect_identifier_named("conjugates", "expected `conjugates` or `~`")?;
                }
            }
            "inverse" => {
                self.expect_identifier_named("of", "expected `of`")?;
            }
            "featuring" => {
                self.expect_identifier_named("by", "expected `by`")?;
            }
            "dependency" => {
                self.expect_identifier_named("to", "expected `to`")?;
            }
            "disjoining" => {
                self.expect_identifier_named("from", "expected `from`")?;
            }
            _ => return Err(self.error_here("unsupported relationship role")),
        }
        let mut targets = vec![self.parse_relationship_operand()?];
        if keyword == "dependency" {
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                targets.push(self.parse_relationship_operand()?);
            }
        }
        let source_chain = matches!(
            keyword.as_str(),
            "subtype" | "subset" | "redefinition" | "conjugate" | "inverse" | "disjoining"
        );
        let target_chain = source_chain || keyword == "typing";
        if (!source_chain && sources.iter().any(|p| p.steps.len() > 1))
            || (!target_chain && targets.iter().any(|p| p.steps.len() > 1))
        {
            return Err(
                self.error_here("feature chain is not allowed in this relationship operand")
            );
        }
        let mut body_members = Vec::new();
        let end = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                let Some(member) = self.parse_declaration()? else {
                    break;
                };
                body_members.push(member);
            }
            self.expect(TokenKind::RBrace, "expected `}` after relationship body")?
        } else {
            self.expect(TokenKind::Semicolon, "expected `;` or relationship body")?
        };
        let span = merge_span(&start.span, &end.span);
        let mut metadata_properties = Default::default();
        store(
            &mut metadata_properties,
            &Endpoints { sources, targets },
            &span,
        )?;
        Ok(GenericUsageDecl {
        annotation_targets: Vec::new(),
            keyword,
            is_implicit_name: name.is_none(),
            name: name.unwrap_or_else(|| {
                format!(
                    "relationship_{}_{}",
                    start.span.start_line, start.span.start_col
                )
            }),
            ty: None,
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties,
            multiplicity: None,
            expression: None,
            additional_types: Vec::new(),
            specializes: Vec::new(),
            subsets: Vec::new(),
            redefines: Vec::new(),
            body_members,
            comments: Vec::new(),
            docs,
            modifiers,
            span,
        })
    }
}
