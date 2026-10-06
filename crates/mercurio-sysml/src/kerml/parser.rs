mod connectors;
mod relationships;
mod crossing_features;

use mercurio_foundation::language_contracts::ast::{
    AliasDecl, CommentNote, Declaration, Expr, GenericDefinitionDecl, GenericUsageDecl,
    MultiplicityRange, PackageDecl, ParsedModule as SysmlModule, QualifiedName, SourceSpan,
};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
use mercurio_foundation::language_contracts::lexer::{Token, TokenKind};
use crate::xtext_terminal::lex;

pub fn parse_kerml(input: &str) -> Result<SysmlModule, Diagnostic> {
    let tokens = lex(input)?;
    Parser::new(tokens).parse()
}

pub fn parse(input: &str) -> Result<SysmlModule, Diagnostic> {
    parse_kerml(input)
}

/// Continue a shared alias/import header with KerML's own body productions.
/// Both token ownership and cursor are restored even when parsing fails.
pub(crate) fn parse_relationship_body_declarations(
    tokens: &mut Vec<Token>, index: &mut usize,
) -> Result<(Vec<Declaration>, Token), Diagnostic> {
    let mut parser = Parser::new(std::mem::take(tokens));
    parser.index = *index;
    parser.reject_opaque_relationship_members = true;
    let result = (|| {
        parser.expect(TokenKind::LBrace, "expected relationship body")?;
        let mut members = Vec::new();
        while !matches!(parser.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let Some(member) = parser.parse_declaration()? else { break; };
            if !crate::namespace_grammar::permits_relationship_member(true, &member) {
                return Err(Diagnostic::new("expected an owned element or annotation in KerML relationship body", Some(crate::parser::declaration_span(&member).clone())));
            }
            members.push(member);
        }
        let end = parser.expect(TokenKind::RBrace, "expected `}` to close relationship body")?;
        Ok((members, end))
    })();
    *index = parser.index;
    *tokens = parser.tokens;
    result
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    pending_docs: Vec<String>,
    pending_comments: Vec<CommentNote>,
    reject_opaque_relationship_members: bool,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            index: 0,
            pending_docs: Vec::new(),
            pending_comments: Vec::new(),
            reject_opaque_relationship_members: false,
        }
    }

    fn parse(mut self) -> Result<SysmlModule, Diagnostic> {
        let mut module = SysmlModule::default();

        while !self.at_end() {
            self.collect_docs();
            let Some(declaration) = self.parse_declaration()? else {
                break;
            };
            match declaration {
                Declaration::Package(package) => {
                    if module.package.is_none() {
                        module.package = Some(package.clone());
                    }
                    module.members.push(Declaration::Package(package));
                }
                Declaration::Import(import) => {
                    module.imports.push(import.clone());
                    module.members.push(Declaration::Import(import));
                }
                Declaration::GenericDefinition(definition) => {
                    module
                        .members
                        .push(Declaration::GenericDefinition(definition));
                }
                Declaration::GenericUsage(usage) => {
                    module.members.push(Declaration::GenericUsage(usage));
                }
                Declaration::Alias(alias) => module.members.push(Declaration::Alias(alias)),
            }
        }

        Ok(module)
    }

    /// Parses one declaration, harvesting leading own-line comment trivia
    /// from the declaration's first token, mirroring the SysML parser.
    /// v1 scope: leading own-line comments only — trailing same-line and
    /// dangling-before-`}` comments are dropped (documented follow-up).
    fn parse_declaration(&mut self) -> Result<Option<Declaration>, Diagnostic> {
        self.collect_docs();
        let comments = self.take_leading_comments();
        let declaration = self.parse_declaration_inner()?;
        Ok(declaration.map(|declaration| attach_leading_comments(declaration, comments)))
    }

    fn take_leading_comments(&mut self) -> Vec<CommentNote> {
        let mut comments = std::mem::take(&mut self.pending_comments);
        if let Some(token) = self.tokens.get_mut(self.index) {
            comments.extend(
                std::mem::take(&mut token.leading_trivia)
                    .into_iter()
                    .filter(|trivia| trivia.own_line)
                    .map(|trivia| CommentNote {
                        text: trivia.text,
                        kind: trivia.kind,
                    }),
            );
        }
        comments
    }

    fn parse_declaration_inner(&mut self) -> Result<Option<Declaration>, Diagnostic> {
        self.collect_docs();
        let docs = std::mem::take(&mut self.pending_docs);
        let annotation_start = self.current().span.clone();
        let metadata_prefixes = self.parse_metadata_prefixes()?;
        let modifier_start = self.index;
        let modifiers = self.parse_modifiers();
        let metadata_prefixes_after_modifiers = self.parse_metadata_prefixes()?;
        let prefixes = metadata_prefixes.iter().chain(&metadata_prefixes_after_modifiers).cloned().collect::<Vec<_>>();
        if modifiers.iter().any(|modifier| modifier == "end") {
            if let Some(feature) = self.parse_crossing_end_feature(modifier_start, docs.clone())? {
                let declaration = Declaration::GenericUsage(feature);
                return Ok(Some(if prefixes.is_empty() { declaration } else {
                    crate::parser::attach_metadata_prefixes(declaration, &annotation_start, &prefixes)
                }));
            }
        }
        self.skip_multiplicity();
        let declaration = if matches!(self.peek_kind(), TokenKind::Package) {
            self.parse_package(docs, modifiers).map(Declaration::Package).map(Some)
        } else if matches!(self.peek_kind(), TokenKind::Identifier(value) if is_definition_keyword(value)) {
            self.parse_classifier(docs, modifiers).map(Declaration::GenericDefinition).map(Some)
        } else {
            self.parse_non_classifier_declaration(docs, modifiers, !prefixes.is_empty())
        }?;
        Ok(declaration.map(|declaration| if prefixes.is_empty() { declaration } else {
            crate::parser::attach_metadata_prefixes(declaration, &annotation_start, &prefixes)
        }))
    }

    // Keep the large leaf-declaration dispatcher out of recursive namespace/classifier frames.
    #[inline(never)]
    fn parse_non_classifier_declaration(
        &mut self, docs: Vec<String>, modifiers: Vec<String>, has_metadata_prefixes: bool,
    ) -> Result<Option<Declaration>, Diagnostic> {
        match self.peek_kind().clone() {
            TokenKind::BlockDoc(_) => Ok(Some(crate::parser::parse_bare_comment(&self.tokens, &mut self.index, docs, modifiers, true)?)),
            TokenKind::At => Ok(Some(Declaration::GenericUsage(self.parse_metadata(docs, modifiers)?))),
            TokenKind::Identifier(value) if value == "metadata" =>
                Ok(Some(Declaration::GenericUsage(self.parse_metadata(docs, modifiers)?))),
            TokenKind::Import => Ok(Some(Declaration::Import(crate::parser::parse_import_declaration(&mut self.tokens, &mut self.index, docs, modifiers)?))),
            TokenKind::Identifier(value) if value == "alias" => {
                Ok(Some(Declaration::Alias(self.parse_alias(docs, modifiers)?)))
            }
            TokenKind::Identifier(value) if value == "multiplicity" => Ok(Some(
                Declaration::GenericUsage(self.parse_multiplicity_subset(docs, modifiers)?),
            )),
            TokenKind::Identifier(value) if value == "flow"
                || (value == "succession" && matches!(self.next_kind(), Some(TokenKind::Identifier(next)) if next == "flow")) => Ok(Some(
                Declaration::GenericUsage(self.parse_bare_flow(docs, modifiers)?),
            )),
            TokenKind::Identifier(value) if matches!(value.as_str(), "connector" | "binding" | "succession") => {
                Ok(Some(Declaration::GenericUsage(self.parse_connector(docs, modifiers)?)))
            }
            TokenKind::Identifier(value) if matches!(value.as_str(),
                "specialization" | "subtype" | "subclassifier" | "typing" | "subset"
                | "redefinition" | "conjugation" | "conjugate" | "inverting" | "inverse"
                | "featuring" | "dependency" | "disjoining" | "disjoint") => Ok(Some(
                    Declaration::GenericUsage(self.parse_relationship(docs, modifiers)?),
                )),
            TokenKind::Identifier(value) if value == "rep" || value == "language" => {
                let start = self.current().clone();
                self.advance();
                crate::parser::textual_representation::parse(
                    &mut self.tokens, &mut self.index, start, docs, modifiers, true,
                ).map(|usage| Some(Declaration::GenericUsage(usage)))
            }
            TokenKind::Identifier(value)
                if value == "feature" && matches!(self.next_kind(), Some(TokenKind::Def)) =>
            {
                Ok(Some(Declaration::GenericDefinition(
                    self.parse_feature_definition(docs)?,
                )))
            }
            TokenKind::Identifier(value) if matches!(value.as_str(), "feature" | "step" | "expr" | "bool" | "inv") => Ok(Some(
                Declaration::GenericUsage(self.parse_feature_with_modifiers(docs, modifiers)?),
            )),
            TokenKind::Identifier(value) if value == "comment" || value == "locale" || value == "doc" => {
                crate::parser::parse_comment_declaration(
                    &mut self.tokens, &mut self.index, docs, modifiers,
                ).map(|usage| Some(Declaration::GenericUsage(usage)))
            }
            TokenKind::Identifier(value)
                if !modifiers.is_empty() || self.starts_unprefixed_feature(&value) =>
            {
                Ok(Some(Declaration::GenericUsage(
                    self.parse_unprefixed_feature(docs, modifiers)?,
                )))
            }
            TokenKind::Colon | TokenKind::Specializes | TokenKind::Redefines => Ok(Some(
                Declaration::GenericUsage(self.parse_unprefixed_feature(docs, modifiers)?),
            )),
            TokenKind::Semicolon | TokenKind::LBrace if modifiers.iter().any(|m| m == "return") => Ok(Some(
                Declaration::GenericUsage(self.parse_unprefixed_feature(docs, modifiers)?),
            )),
            TokenKind::Eof => Ok(None),
            TokenKind::Identifier(_) => Ok(Some(
                Declaration::GenericUsage(self.parse_opaque_declaration(docs, modifiers)?),
            )),
            TokenKind::LBrace | TokenKind::Semicolon
                if has_metadata_prefixes =>
            {
                Ok(Some(Declaration::GenericUsage(
                    self.parse_opaque_declaration(docs, modifiers)?,
                )))
            }
            TokenKind::RBrace => Ok(None),
            _ => Err(self.error_here(
                "expected a KerML declaration such as `package`, `import`, `classifier`, or `feature`",
            )),
        }
    }

    // KerML.xtext MetadataFeatureDeclaration: optional identification/colon,
    // required metaclass typing, optional annotations, then MetadataBody.
    fn parse_metadata(
        &mut self,
        docs: Vec<String>,
        modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.current().clone();
        self.advance();
        let first = if matches!(self.peek_kind(), TokenKind::Colon) {
            None
        } else {
            Some(self.parse_qualified_name()?)
        };
        let (name, ty, is_implicit_name) = if matches!(self.peek_kind(), TokenKind::Colon)
            && !matches!(self.next_kind(), Some(TokenKind::Equals))
        {
            self.advance();
            let ty = self.parse_qualified_name()?;
            (
                first
                    .as_ref()
                    .map(QualifiedName::as_dot_string)
                    .unwrap_or_else(|| {
                        format!(
                            "metadata_{}_{}",
                            start.span.start_line, start.span.start_col
                        )
                    }),
                ty,
                first.is_none(),
            )
        } else {
            let ty = first.ok_or_else(|| self.error_here("expected metadata type"))?;
            (
                format!(
                    "metadata_{}_{}",
                    start.span.start_line, start.span.start_col
                ),
                ty,
                true,
            )
        };
        let reference_target = if matches!(self.peek_kind(), TokenKind::Identifier(value) if value == "about")
        {
            self.advance();
            Some(self.parse_qualified_name()?)
        } else {
            None
        };
        let mut body_members = Vec::new();
        let end = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                if let Some(member) = self.parse_declaration()? {
                    body_members.push(member);
                }
            }
            self.expect(TokenKind::RBrace, "expected `}` after metadata body")?
        } else {
            self.expect(TokenKind::Semicolon, "expected metadata body or `;`")?
        };
        Ok(GenericUsageDecl {
        annotation_targets: Vec::new(),
            keyword: "metadata".to_string(),
            name,
            is_implicit_name,
            ty: Some(ty),
            reference_target,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: Default::default(),
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
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_package(&mut self, docs: Vec<String>, modifiers: Vec<String>) -> Result<PackageDecl, Diagnostic> {
        let start = self.expect(TokenKind::Package, "expected `package`")?;
        let name = self.parse_qualified_name()?;
        let _specializes = self.parse_optional_specializations()?;
        if matches!(self.peek_kind(), TokenKind::Semicolon) {
            let end = self.expect(TokenKind::Semicolon, "expected `;` after package")?;
            return Ok(PackageDecl {
                name,
                members: Vec::new(),
                imports: Vec::new(),
                definitions: Vec::new(),
                comments: Vec::new(),
                docs,
                modifiers,
                span: merge_span(&start.span, &end.span),
            });
        }
        self.expect(TokenKind::LBrace, "expected `{` after package name")?;

        let mut members = Vec::new();
        let mut imports = Vec::new();
        while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let Some(declaration) = self.parse_declaration()? else {
                break;
            };
            if let Declaration::Import(import) = &declaration {
                imports.push(import.clone());
            }
            members.push(declaration);
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` to close package")?;

        Ok(PackageDecl {
            name,
            members,
            imports,
            definitions: Vec::new(),
            comments: Vec::new(),
            docs,
            modifiers,
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_alias(&mut self, docs: Vec<String>, modifiers: Vec<String>) -> Result<AliasDecl, Diagnostic> {
        crate::parser::parse_alias_declaration(&mut self.tokens, &mut self.index, docs, modifiers)
    }

    // KerML.xtext Flow/SuccessionFlow with the empty and FlowEnd endpoint
    // arms, literal ValuePart, and the typed PayloadFeature arm.
    fn parse_bare_flow(&mut self, docs: Vec<String>, modifiers: Vec<String>) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.current().clone();
        let succession = matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "succession");
        if succession {
            self.advance();
        }
        self.expect_identifier_named("flow", "expected `flow`")?;
        let mut modifiers = modifiers;
        if matches!(self.peek_kind(), TokenKind::LAngle) {
            self.advance();
            let short = self.expect_identifier("expected flow short name")?;
            self.expect(TokenKind::RAngle, "expected `>` after flow short name")?;
            modifiers.push(format!("short_name={short}"));
        }
        let all = matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "all");
        if all { self.advance(); modifiers.push("is_sufficient".into()); }
        let mut endpoints = None;
        if all {
            let source = self.parse_relationship_operand()?;
            self.expect_identifier_named("to", "expected `to` between Flow ends")?;
            let target = self.parse_relationship_operand()?;
            endpoints = Some((source, target));
        } else if !matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "from" || word == "of") {
            let saved = self.index;
            if let Ok(source) = self.parse_relationship_operand() {
                if matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "to") {
                    self.advance();
                    let target = self.parse_relationship_operand()?;
                    endpoints = Some((source, target));
                }
            }
            if endpoints.is_none() { self.index = saved; }
        }
        let explicit_name = if endpoints.is_none() && matches!(self.peek_kind(), TokenKind::Identifier(word)
            if !matches!(word.as_str(), "of" | "from" | "to" | "all" | "default")) {
            Some(self.expect_identifier("expected flow name")?)
        } else { None };
        let expression = if all || endpoints.is_some() { None }
            else { self.parse_feature_initializer(&mut modifiers)? };
        let mut metadata_properties: std::collections::BTreeMap<String, String> = Default::default();
        if !all && endpoints.is_none() && matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "of") {
            self.advance();
            let mut leading_bounds = self.parse_optional_multiplicity()?;
            let mut payload_name = None;
            if leading_bounds.is_none() && matches!(self.peek_kind(), TokenKind::Identifier(_))
                && matches!(self.next_kind(), Some(TokenKind::LBracket)) {
                let saved = self.index;
                let candidate_name = self.expect_identifier("expected payload feature name")?;
                let candidate_bounds = self.parse_optional_multiplicity()?;
                if matches!(self.peek_kind(), TokenKind::Colon) {
                    payload_name = Some(candidate_name);
                    leading_bounds = candidate_bounds;
                } else { self.index = saved; }
            }
            if payload_name.is_none() && matches!(self.peek_kind(), TokenKind::Identifier(_))
                && matches!(self.next_kind(), Some(TokenKind::Colon)) {
                payload_name = Some(self.expect_identifier("expected payload feature name")?);
            }
            if matches!(self.peek_kind(), TokenKind::Colon) { self.advance(); }
            else if payload_name.is_some() {
                return Err(self.error_here("expected `:` after payload feature name"));
            }
            let payload_type = self.parse_qualified_name()?;
            let trailing_bounds = self.parse_optional_multiplicity()?;
            if leading_bounds.is_some() && trailing_bounds.is_some() {
                return Err(self.error_here("PayloadFeature may own only one multiplicity"));
            }
            metadata_properties.insert("__flow_payload_type".into(),
                serde_json::to_string(&payload_type).expect("QualifiedName serializes"));
            if let Some(name) = payload_name {
                metadata_properties.insert("__flow_payload_name".into(), name);
            }
            let bounds_first = leading_bounds.is_some();
            if let Some(bounds) = leading_bounds.or(trailing_bounds) {
                let references = multiplicity_bound_references(&bounds)?;
                metadata_properties.insert("__flow_payload_bounds".into(), bounds.raw);
                metadata_properties.insert("__flow_payload_references".into(),
                    serde_json::to_string(&references).expect("QualifiedName serializes"));
                metadata_properties.insert("__flow_payload_bounds_first".into(),
                    bounds_first.to_string());
            }
        }
        if endpoints.is_none() && matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "from") {
            self.advance();
            let source = self.parse_relationship_operand()?;
            self.expect_identifier_named("to", "expected `to` between Flow ends")?;
            let target = self.parse_relationship_operand()?;
            endpoints = Some((source, target));
        }
        if let Some((source, target)) = endpoints {
            crate::language_frontend::lowering::relationship_declarations::store(
                &mut metadata_properties,
                &crate::language_frontend::lowering::relationship_declarations::Endpoints {
                    sources: vec![source], targets: vec![target],
                }, &start.span,
            )?;
        }
        if !matches!(self.peek_kind(), TokenKind::Semicolon | TokenKind::LBrace) {
            return Err(self.error_here("remaining Flow payload and feature forms are not implemented"));
        }
        let mut body_members = Vec::new();
        let end = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                let Some(member) = self.parse_declaration()? else { break };
                body_members.push(member);
            }
            self.expect(TokenKind::RBrace, "expected `}` after flow body")?
        } else {
            self.expect(TokenKind::Semicolon, "expected flow body or `;`")?
        };
        Ok(GenericUsageDecl {
            annotation_targets: Vec::new(),
            keyword: if succession { "kerml-succession-flow" } else { "kerml-flow" }.into(),
            name: explicit_name.clone().unwrap_or_else(|| format!("flow_{}_{}", start.span.start_line, start.span.start_col)),
            is_implicit_name: explicit_name.is_none(), ty: None, reference_target: None,
            allocation_source: None, allocation_target: None, metadata_properties,
            multiplicity: None, expression, additional_types: Vec::new(),
            specializes: Vec::new(), subsets: Vec::new(), redefines: Vec::new(),
            body_members, comments: Vec::new(), docs, modifiers,
            span: merge_span(&start.span, &end.span),
        })
    }

    // KerML.xtext MultiplicitySubset and the literal-bound MultiplicityRange arm.
    fn parse_multiplicity_subset(&mut self, docs: Vec<String>, mut modifiers: Vec<String>) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.expect_identifier_named("multiplicity", "expected `multiplicity`")?;
        if matches!(self.peek_kind(), TokenKind::LAngle) {
            self.advance();
            let short = self.expect_identifier("expected multiplicity short name")?;
            self.expect(TokenKind::RAngle, "expected `>` after short name")?;
            modifiers.push(format!("short_name={short}"));
        }
        let explicit_name = if matches!(self.peek_kind(), TokenKind::Identifier(word) if word != "subsets") {
            Some(self.expect_identifier("expected multiplicity name")?)
        } else { None };
        let (keyword, subsets, metadata_properties) = if matches!(self.peek_kind(), TokenKind::LBracket) {
            let bounds = self.parse_optional_multiplicity()?.expect("opening bracket checked");
            let references = multiplicity_bound_references(&bounds)?;
            let mut metadata = std::collections::BTreeMap::new();
            metadata.insert("__multiplicity_range_bounds".into(), bounds.raw);
            metadata.insert("__multiplicity_range_references".into(),
                serde_json::to_string(&references).expect("QualifiedName serializes"));
            ("multiplicity-range".to_string(), Vec::new(), metadata)
        } else {
            if matches!(self.peek_kind(), TokenKind::Specializes) {
                self.advance();
            } else if matches!(self.peek_kind(), TokenKind::Identifier(word) if word == "subsets") {
                self.advance();
            } else {
                return Err(self.error_here("expected `subsets`, `:>`, or multiplicity bounds"));
            }
            let operand = self.parse_relationship_operand()?;
            if operand.steps.len() > 1 {
                let mut metadata = std::collections::BTreeMap::new();
                crate::language_frontend::lowering::relationship_declarations::store(
                    &mut metadata,
                    &crate::language_frontend::lowering::relationship_declarations::Endpoints {
                        sources: Vec::new(), targets: vec![operand],
                    },
                    &start.span,
                )?;
                ("multiplicity".to_string(), Vec::new(), metadata)
            } else {
                ("multiplicity".to_string(), operand.steps, Default::default())
            }
        };
        let mut body_members = Vec::new();
        let end = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                let Some(member) = self.parse_declaration()? else { break };
                body_members.push(member);
            }
            self.expect(TokenKind::RBrace, "expected `}` after multiplicity body")?
        } else {
            self.expect(TokenKind::Semicolon, "expected multiplicity body or `;`")?
        };
        Ok(GenericUsageDecl {
            annotation_targets: Vec::new(), keyword,
            name: explicit_name.clone().unwrap_or_else(|| format!("multiplicity_{}_{}", start.span.start_line, start.span.start_col)),
            is_implicit_name: explicit_name.is_none(), ty: None, reference_target: None,
            allocation_source: None, allocation_target: None, metadata_properties,
            multiplicity: None, expression: None, additional_types: Vec::new(),
            specializes: Vec::new(), subsets, redefines: Vec::new(),
            body_members, comments: Vec::new(), docs, modifiers,
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_classifier(
        &mut self,
        docs: Vec<String>,
        mut modifiers: Vec<String>,
    ) -> Result<GenericDefinitionDecl, Diagnostic> {
        modifiers.push("definition_keyword_complete".into());
        let start = self.expect_identifier_token("expected classifier keyword")?;
        let mut keyword = token_identifier(&start).to_string();
        if keyword == "assoc"
            && matches!(self.peek_kind(), TokenKind::Identifier(value) if value == "struct")
        {
            self.advance();
            keyword = "assoc-struct".to_string();
        }
        modifiers.extend(self.parse_modifiers());
        let name = if keyword == "namespace"
            && matches!(self.peek_kind(), TokenKind::Semicolon | TokenKind::LBrace)
        {
            modifiers.push("anonymous_namespace".into());
            format!("namespace_{}_{}", start.span.start_line, start.span.start_col)
        } else {
            let (consumed, name, short) = crate::xtext_fragment::definition_identification(true, &self.tokens[self.index..])?;
            self.index += consumed;
            if let Some(short) = short { modifiers.push(format!("short_name={short}")); }
            name
        };
        let specializes = if keyword == "namespace" {
            Vec::new()
        } else {
            self.skip_multiplicity();
            self.parse_classifier_relations()?
        };

        let mut members = Vec::new();
        let end = match self.peek_kind() {
            TokenKind::Semicolon => self.expect(TokenKind::Semicolon, "expected `;`")?,
            TokenKind::RBrace => self.current().clone(),
            TokenKind::LBrace => {
                self.advance();
                while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                    if self.try_parse_classifier_result(&keyword, &mut members)? { break; }
                    let Some(declaration) = self.parse_declaration()? else {
                        break;
                    };
                    members.push(declaration);
                }
                self.expect(TokenKind::RBrace, "expected `}` to close classifier")?
            }
            _ => return Err(self.error_here("expected `;` or `{` after classifier declaration")),
        };

        Ok(GenericDefinitionDecl {
            keyword,
            name,
            specializes,
            members,
            comments: Vec::new(),
            docs,
            modifiers,
            span: merge_span(&start.span, &end.span),
        })
    }

    #[inline(never)]
    fn try_parse_classifier_result(
        &mut self, keyword: &str, members: &mut Vec<Declaration>,
    ) -> Result<bool, Diagnostic> {
        if matches!(
            keyword,
            "function" | "predicate" | "expression" | "bool"
        ) {
            if let Ok((expression, consumed)) =
                crate::parser::parse_expression_prefix(&self.tokens[self.index..])
            {
                if matches!(
                    self.tokens.get(self.index + consumed).map(|t| &t.kind),
                    Some(TokenKind::RBrace)
                ) {
                    let span = merge_span(
                        &self.current().span,
                        &self.tokens[self.index + consumed - 1].span,
                    );
                    self.index += consumed;
                    if let Some(Declaration::GenericUsage(result)) = members.iter_mut().find(|member| matches!(member, Declaration::GenericUsage(usage) if usage.modifiers.iter().any(|m| m == "return"))) {
                        if result.expression.is_some() { return Err(self.error_here("function has both a return initializer and a result expression")); }
                        result.expression = Some(expression);
                    } else {
                        members.push(Declaration::GenericUsage(GenericUsageDecl {
                annotation_targets: Vec::new(),
                            keyword: "feature".to_string(), name: "result".to_string(), is_implicit_name: true,
                            ty: None, reference_target: None, allocation_source: None, allocation_target: None,
                            metadata_properties: Default::default(), multiplicity: None, expression: Some(expression),
                            additional_types: Vec::new(), specializes: Vec::new(), subsets: Vec::new(), redefines: Vec::new(),
                            body_members: Vec::new(), comments: Vec::new(), docs: Vec::new(), modifiers: vec!["return".to_string()], span,
                        }));
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn parse_feature_definition(
        &mut self,
        docs: Vec<String>,
    ) -> Result<GenericDefinitionDecl, Diagnostic> {
        let start = self.expect_identifier_named("feature", "expected `feature`")?;
        self.expect(TokenKind::Def, "expected `def` after `feature`")?;
        let name = self.expect_identifier("expected feature definition name")?;
        let specializes = self.parse_optional_specializations()?;

        let mut members = Vec::new();
        let end = match self.peek_kind() {
            TokenKind::Semicolon => self.expect(TokenKind::Semicolon, "expected `;`")?,
            TokenKind::RBrace => self.current().clone(),
            TokenKind::LBrace => {
                self.advance();
                while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                    let Some(declaration) = self.parse_declaration()? else {
                        break;
                    };
                    members.push(declaration);
                }
                self.expect(
                    TokenKind::RBrace,
                    "expected `}` to close feature definition",
                )?
            }
            _ => return Err(self.error_here("expected `;` or `{` after feature definition")),
        };

        Ok(GenericDefinitionDecl {
            keyword: "feature".to_string(),
            name,
            specializes,
            members,
            comments: Vec::new(),
            docs,
            modifiers: Vec::new(),
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_feature_with_modifiers(
        &mut self,
        docs: Vec<String>,
        modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.expect_identifier_token("expected feature keyword")?;
        let keyword = token_identifier(&start).to_string();
        let mut modifiers = modifiers;
        if keyword == "inv" && matches!(self.peek_kind(), TokenKind::Identifier(s) if s == "true" || s == "false") {
            if matches!(self.peek_kind(), TokenKind::Identifier(s) if s == "false") { modifiers.push("is_negated".into()); }
            self.advance();
        }
        self.parse_feature_declaration(start, keyword, docs, modifiers)
    }

    fn parse_feature_declaration(
        &mut self,
        start: Token,
        keyword: String,
        docs: Vec<String>,
        mut modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        if matches!(self.peek_kind(), TokenKind::LAngle) {
            self.advance();
            let short_name = self.expect_identifier("expected short name")?;
            self.expect(TokenKind::RAngle, "expected `>` after short name")?;
            modifiers.push(format!("short_name={short_name}"));
        }
        modifiers.extend(self.parse_modifiers());
        let explicit_name = if matches!(self.peek_kind(), TokenKind::Identifier(value) if !is_relation_keyword_name(value))
        {
            Some(self.expect_identifier("expected feature name")?)
        } else {
            None
        };
        let mut relations = self.parse_optional_feature_relations()?;
        let name = explicit_name
            .clone()
            .or_else(|| {
                relations
                    .redefines
                    .first()
                    .and_then(|n| n.segments.last().cloned())
            })
            .unwrap_or_else(|| {
                format!("feature_{}_{}", start.span.start_line, start.span.start_col)
            });
        let is_implicit_name = explicit_name.is_none();
        let mut multiplicity = self.parse_optional_multiplicity()?;
        let mut additional_types = Vec::new();
        let ty = if matches!(self.peek_kind(), TokenKind::Colon)
            && !matches!(self.next_kind(), Some(TokenKind::Equals))
        {
            self.advance();
            let ty = self.parse_qualified_name()?;
            multiplicity = self.parse_optional_multiplicity()?.or(multiplicity);
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                additional_types.push(self.parse_qualified_name()?);
                multiplicity = self.parse_optional_multiplicity()?.or(multiplicity);
            }
            Some(ty)
        } else {
            None
        };
        modifiers.extend(self.parse_modifiers());
        let trailing_relations = self.parse_optional_feature_relations()?;
        relations.specializes.extend(trailing_relations.specializes);
        relations.subsets.extend(trailing_relations.subsets);
        relations.redefines.extend(trailing_relations.redefines);
        if trailing_relations.references.is_some() {
            if relations.references.is_some() {
                return Err(self.error_here("a feature cannot own two reference subsettings"));
            }
            relations.references = trailing_relations.references;
        }
        modifiers.extend(self.parse_modifiers());
        let mut expression = self.parse_feature_initializer(&mut modifiers)?;
        if expression.is_none() {
            self.skip_feature_tail();
        }

        let mut body_members = Vec::new();
        let end = match self.peek_kind() {
            TokenKind::Semicolon => self.expect(TokenKind::Semicolon, "expected `;`")?,
            TokenKind::RBrace => self.current().clone(),
            TokenKind::LBrace => {
                self.advance();
                while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                    if matches!(keyword.as_str(), "expr" | "bool" | "inv") {
                        if let Ok((result, consumed)) =
                            crate::parser::parse_expression_prefix(&self.tokens[self.index..])
                        {
                            if matches!(
                                self.tokens.get(self.index + consumed).map(|t| &t.kind),
                                Some(TokenKind::RBrace)
                            ) {
                                if expression.is_some() {
                                    return Err(self.error_here(
                                        "expression has both an initializer and a result body",
                                    ));
                                }
                                modifiers.push("expression_is_result".into());
                                expression = Some(result);
                                self.index += consumed;
                                break;
                            }
                        }
                    }
                    let Some(declaration) = self.parse_declaration()? else {
                        break;
                    };
                    body_members.push(declaration);
                }
                self.expect(TokenKind::RBrace, "expected `}` to close feature")?
            }
            _ => return Err(self.error_here("expected `;` or `{` after feature declaration")),
        };

        Ok(GenericUsageDecl {
        annotation_targets: Vec::new(),
            keyword,
            name,
            is_implicit_name,
            ty,
            reference_target: relations.references,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: Default::default(),
            multiplicity,
            expression,
            additional_types,
            specializes: relations.specializes,
            subsets: relations.subsets,
            redefines: relations.redefines,
            body_members,
            comments: Vec::new(),
            docs,
            modifiers,
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_unprefixed_feature(
        &mut self,
        docs: Vec<String>,
        modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        self.parse_feature_declaration(self.current().clone(), "feature".into(), docs, modifiers)
    }

    fn parse_opaque_declaration(
        &mut self,
        docs: Vec<String>,
        modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        if self.reject_opaque_relationship_members {
            return Err(self.error_here("unsupported KerML relationship-body declaration"));
        }
        let start = self.current().clone();
        let keyword = match self.peek_kind().clone() {
            TokenKind::Identifier(value) => {
                self.advance();
                value
            }
            TokenKind::Specializes => {
                self.advance();
                "specialization".to_string()
            }
            TokenKind::Redefines => {
                self.advance();
                "redefinition".to_string()
            }
            _ => "declaration".to_string(),
        };
        self.skip_angle_metadata();
        // These relationship forms start with an operand, not an Identification.
        // Until their relationship lowering is complete, never index that operand
        // as a new feature that shadows the actual referenced declaration.
        let is_implicit_name = matches!(keyword.as_str(),
            "subtype" | "subclassifier" | "typing" | "subset" | "redefinition"
                | "conjugate" | "disjoint" | "inverse");
        let name = if is_implicit_name {
            format!("{keyword}_{}_{}", start.span.start_line, start.span.start_col)
        } else {
            match self.peek_kind().clone() {
                TokenKind::Identifier(value) => { self.advance(); value }
                _ => keyword.clone(),
            }
        };

        let end = self.consume_declaration_tail();
        Ok(GenericUsageDecl {
        annotation_targets: Vec::new(),
            keyword,
            name,
            is_implicit_name,
            ty: None,
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: Default::default(),
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
            span: merge_span(&start.span, &end.span),
        })
    }

    fn parse_optional_specializations(&mut self) -> Result<Vec<QualifiedName>, Diagnostic> {
        let mut specializes = Vec::new();
        if matches!(self.peek_kind(), TokenKind::Specializes)
            || matches!(self.peek_kind(), TokenKind::Identifier(value) if value == "specializes")
        {
            self.advance();
            specializes.push(self.parse_qualified_name()?);
            while matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
                specializes.push(self.parse_qualified_name()?);
            }
        }
        Ok(specializes)
    }

    fn parse_classifier_relations(&mut self) -> Result<Vec<QualifiedName>, Diagnostic> {
        let mut specializes = Vec::new();
        loop {
            match self.peek_kind().clone() {
                TokenKind::Tilde => {
                    // Symbolic spelling of the existing conjugates clause.
                    // Relationship-object lowering remains a separate gap.
                    self.advance();
                    let _ = self.parse_relation_targets()?;
                }
                TokenKind::Specializes => {
                    self.advance();
                    specializes.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "specializes" => {
                    self.advance();
                    specializes.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value)
                    if matches!(
                        value.as_str(),
                        "unions"
                            | "intersects"
                            | "differences"
                            | "disjoint"
                            | "conjugates"
                            | "inverse"
                            | "chains"
                            | "ordered"
                    ) =>
                {
                    self.advance();
                    if value == "disjoint"
                        && matches!(self.peek_kind(), TokenKind::Identifier(next) if next == "from")
                    {
                        self.advance();
                    }
                    if value == "inverse"
                        && matches!(self.peek_kind(), TokenKind::Identifier(next) if next == "of")
                    {
                        self.advance();
                    }
                    let _ = self.parse_relation_targets()?;
                }
                _ => break,
            }
        }
        Ok(specializes)
    }

    fn parse_optional_feature_relations(&mut self) -> Result<FeatureRelations, Diagnostic> {
        let mut relations = FeatureRelations::default();
        loop {
            match self.peek_kind().clone() {
                TokenKind::Specializes => {
                    self.advance();
                    relations.specializes.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "specializes" => {
                    self.advance();
                    relations.specializes.extend(self.parse_relation_targets()?);
                }
                TokenKind::Redefines => {
                    self.advance();
                    relations.redefines.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "redefines" => {
                    self.advance();
                    relations.redefines.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "subsets" => {
                    self.advance();
                    relations.subsets.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "typed" => {
                    self.advance();
                    self.expect_identifier_named("by", "expected `by` after `typed`")?;
                    relations.specializes.extend(self.parse_relation_targets()?);
                }
                TokenKind::Identifier(value) if value == "references" => {
                    self.advance();
                    if relations.references.is_some() {
                        return Err(self.error_here("a feature cannot own two reference subsettings"));
                    }
                    relations.references = Some(self.parse_qualified_name()?);
                }
                TokenKind::ScopeSep if matches!(self.next_kind(), Some(TokenKind::RAngle)) => {
                    self.advance();
                    self.advance();
                    if relations.references.is_some() {
                        return Err(self.error_here("a feature cannot own two reference subsettings"));
                    }
                    relations.references = Some(self.parse_qualified_name()?);
                }
                TokenKind::Identifier(value) if value == "featured" => {
                    self.advance();
                    self.expect_identifier_named("by", "expected `by` after `featured`")?;
                    let _ = self.parse_relation_targets()?;
                }
                TokenKind::Identifier(value)
                    if matches!(
                        value.as_str(),
                        "unions" | "intersects" | "differences" | "disjoint" | "conjugates"
                    ) =>
                {
                    self.advance();
                    if value == "disjoint"
                        && matches!(self.peek_kind(), TokenKind::Identifier(next) if next == "from")
                    {
                        self.advance();
                    }
                    let _ = self.parse_relation_targets()?;
                }
                _ => break,
            }
        }
        Ok(relations)
    }

    fn parse_relation_targets(&mut self) -> Result<Vec<QualifiedName>, Diagnostic> {
        let mut targets = vec![self.parse_qualified_name()?];
        while matches!(self.peek_kind(), TokenKind::Comma) {
            self.advance();
            targets.push(self.parse_qualified_name()?);
        }
        Ok(targets)
    }

    fn parse_qualified_name(&mut self) -> Result<QualifiedName, Diagnostic> {
        let first = self.expect_name_token("expected name")?;
        let mut segments = vec![token_name(&first).to_string()];
        let mut end = first.span.clone();
        while matches!(self.peek_kind(), TokenKind::ScopeSep | TokenKind::Dot) {
            self.advance();
            let next = self.expect_name_token("expected name segment")?;
            segments.push(token_name(&next).to_string());
            end = next.span.clone();
        }
        Ok(QualifiedName {
            segments,
            span: merge_span(&first.span, &end),
        })
    }

    fn parse_modifiers(&mut self) -> Vec<String> {
        let mut modifiers = Vec::new();
        while let TokenKind::Identifier(value) = self.peek_kind().clone() {
            if is_modifier(&value) {
                modifiers.push(value);
                self.advance();
            } else {
                break;
            }
        }
        modifiers
    }

    fn parse_metadata_prefixes(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut prefixes = Vec::new();
        while matches!(self.peek_kind(), TokenKind::Hash) {
            self.advance();
            let (consumed, name) = crate::xtext_fragment::qualified_name(&self.tokens[self.index..], true)?;
            self.index += consumed;
            prefixes.push(name.as_colon_string());
        }
        Ok(prefixes)
    }

    fn starts_unprefixed_feature(&self, value: &str) -> bool {
        !is_definition_keyword(value)
            && !matches!(value, "alias" | "feature")
            && matches!(
                self.next_kind(),
                Some(
                    TokenKind::Colon
                        | TokenKind::Specializes
                        | TokenKind::Redefines
                        | TokenKind::Equals
                        | TokenKind::Semicolon
                        | TokenKind::LBrace
                )
            )
    }

    fn skip_angle_metadata(&mut self) {
        while matches!(self.peek_kind(), TokenKind::LAngle) {
            self.skip_balanced(TokenKind::LAngle, TokenKind::RAngle);
        }
    }

    fn skip_multiplicity(&mut self) {
        if matches!(self.peek_kind(), TokenKind::LBracket) {
            self.skip_balanced(TokenKind::LBracket, TokenKind::RBracket);
        }
    }

    fn parse_optional_multiplicity(&mut self) -> Result<Option<MultiplicityRange>, Diagnostic> {
        if !matches!(self.peek_kind(), TokenKind::LBracket) {
            return Ok(None);
        }
        let (range, consumed) =
            crate::parser::parse_multiplicity_prefix(&self.tokens[self.index..])?;
        self.index += consumed;
        Ok(Some(range))
    }

    fn parse_feature_initializer(
        &mut self,
        modifiers: &mut Vec<String>,
    ) -> Result<Option<Expr>, Diagnostic> {
        let is_default =
            matches!(self.peek_kind(), TokenKind::Identifier(value) if value == "default");
        if is_default {
            self.advance();
        }
        let is_initial = matches!(self.peek_kind(), TokenKind::Colon)
            && matches!(self.next_kind(), Some(TokenKind::Equals));
        if is_initial {
            self.advance();
        }
        if matches!(self.peek_kind(), TokenKind::Equals) {
            self.advance();
        } else if !is_default {
            return Ok(None);
        }
        let (expression, consumed) =
            crate::parser::parse_expression_prefix(&self.tokens[self.index..])?;
        self.index += consumed;
        if is_default {
            modifiers.push("feature_value_is_default".to_string());
        }
        if is_initial {
            modifiers.push("feature_value_is_initial".to_string());
        }
        Ok(Some(expression))
    }

    fn skip_feature_tail(&mut self) {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        while !self.at_end() {
            match self.peek_kind() {
                TokenKind::Semicolon
                    if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 =>
                {
                    break;
                }
                TokenKind::LBrace if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 => {
                    break;
                }
                TokenKind::RBrace if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 => {
                    break;
                }
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LAngle => angle_depth += 1,
                TokenKind::RAngle => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
            self.advance();
        }
    }

    fn consume_declaration_tail(&mut self) -> Token {
        let mut end = self.current().clone();
        let started_with_lbrace = matches!(self.peek_kind(), TokenKind::LBrace);
        let mut brace_depth = 0usize;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;

        while !self.at_end() {
            end = self.current().clone();
            match self.peek_kind() {
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                }
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LAngle => angle_depth += 1,
                TokenKind::RAngle => angle_depth = angle_depth.saturating_sub(1),
                TokenKind::Semicolon
                    if brace_depth == 0
                        && paren_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    self.advance();
                    return end;
                }
                _ => {}
            }
            self.advance();
            if started_with_lbrace && matches!(end.kind, TokenKind::RBrace) && brace_depth == 0 {
                return end;
            }
        }

        end
    }

    fn skip_balanced(&mut self, open: TokenKind, close: TokenKind) {
        let open_discriminant = std::mem::discriminant(&open);
        let close_discriminant = std::mem::discriminant(&close);
        if std::mem::discriminant(self.peek_kind()) != open_discriminant {
            return;
        }
        let mut depth = 0usize;
        while !self.at_end() {
            let discriminant = std::mem::discriminant(self.peek_kind());
            if discriminant == open_discriminant {
                depth += 1;
            } else if discriminant == close_discriminant {
                depth = depth.saturating_sub(1);
                self.advance();
                if depth == 0 {
                    break;
                }
                continue;
            }
            self.advance();
        }
    }

    fn collect_docs(&mut self) {
        while let TokenKind::Doc(text) = self.peek_kind().clone() {
            // Comments preceding a `doc` line ride on the Doc token's trivia;
            // queue them so they attach to the declaration the docs belong to.
            if let Some(token) = self.tokens.get_mut(self.index) {
                self.pending_comments.extend(
                    std::mem::take(&mut token.leading_trivia)
                        .into_iter()
                        .filter(|trivia| trivia.own_line)
                        .map(|trivia| CommentNote {
                            text: trivia.text,
                            kind: trivia.kind,
                        }),
                );
            }
            self.pending_docs.push(text);
            self.advance();
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        let token = self.expect_identifier_token(message)?;
        Ok(token_identifier(&token).to_string())
    }

    fn expect_identifier_named(
        &mut self,
        expected: &str,
        message: &str,
    ) -> Result<Token, Diagnostic> {
        let token = self.expect_identifier_token(message)?;
        if token_identifier(&token) == expected {
            Ok(token)
        } else {
            Err(Diagnostic::new(message, Some(token.span)))
        }
    }

    fn expect_identifier_token(&mut self, message: &str) -> Result<Token, Diagnostic> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Identifier(_) => {
                self.advance();
                Ok(token)
            }
            _ => Err(Diagnostic::new(message, Some(token.span))),
        }
    }

    fn expect_name_token(&mut self, message: &str) -> Result<Token, Diagnostic> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Identifier(_) | TokenKind::String(_) | TokenKind::Dollar => {
                self.advance();
                Ok(token)
            }
            _ => Err(Diagnostic::new(message, Some(token.span))),
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, Diagnostic> {
        let token = self.current().clone();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&kind) {
            self.advance();
            Ok(token)
        } else {
            Err(Diagnostic::new(message, Some(token.span)))
        }
    }

    fn error_here(&self, message: &str) -> Diagnostic {
        Diagnostic::new(message, Some(self.current().span.clone()))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn next_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.index + 1).map(|token| &token.kind)
    }

    fn advance(&mut self) {
        if !self.at_end() {
            self.index += 1;
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }
}

#[derive(Debug, Default)]
struct FeatureRelations {
    references: Option<QualifiedName>,
    specializes: Vec<QualifiedName>,
    subsets: Vec<QualifiedName>,
    redefines: Vec<QualifiedName>,
}

fn multiplicity_bound_references(bounds: &MultiplicityRange) -> Result<Vec<Option<QualifiedName>>, Diagnostic> {
    let valid_integer = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let parts = crate::parser::split_multiplicity_bound_text(&bounds.raw);
    if !(1..=2).contains(&parts.len()) || parts.iter().any(|part| part.is_empty()) {
        return Err(Diagnostic::new("invalid MultiplicityRange bounds", Some(bounds.span.clone())));
    }
    parts.iter().map(|part| {
        if valid_integer(part) || *part == "*" || matches!(*part, "true" | "false")
            || (part.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && part.parse::<f64>().is_ok())
            || (part.starts_with('"') && serde_json::from_str::<String>(part).is_ok()) {
            return Ok(None);
        }
        let mut parser = Parser::new(lex(part)?);
        let mut name = parser.parse_qualified_name()?;
        if !matches!(parser.peek_kind(), TokenKind::Eof) {
            return Err(Diagnostic::new("unsupported MultiplicityRange bound expression", Some(bounds.span.clone())));
        }
        name.span = bounds.span.clone();
        Ok(Some(name))
    }).collect()
}

fn token_identifier(token: &Token) -> &str {
    match &token.kind {
        TokenKind::Identifier(value) => value,
        _ => unreachable!(),
    }
}

fn token_name(token: &Token) -> &str {
    match &token.kind {
        TokenKind::Identifier(value) | TokenKind::String(value) => value,
        TokenKind::Dollar => "$",
        _ => unreachable!(),
    }
}

fn is_definition_keyword(value: &str) -> bool {
    matches!(
        value,
        "type"
            | "classifier"
            | "class"
            | "struct"
            | "datatype"
            | "behavior"
            | "function"
            | "predicate"
            | "interaction"
            | "association"
            | "assoc"
            | "metaclass"
            | "namespace"
    )
}

fn is_modifier(value: &str) -> bool {
    if crate::enum_grammar::declaration_modifier(true, value) { return true; }
    matches!(
        value,
        "library"
            | "standard"
            | "abstract"
            | "all"
            | "composite"
            | "portion"
            | "const"
            | "var"
            | "member"
            | "readonly"
            | "derived"
            | "end"
            | "return"
            | "ref"
            | "nonunique"
            | "ordered"
    )
}

fn is_relation_keyword_name(value: &str) -> bool {
    matches!(
        value,
        "redefines" | "subsets" | "specializes" | "typed" | "featured" | "references"
    )
}

fn merge_span(left: &SourceSpan, right: &SourceSpan) -> SourceSpan {
    SourceSpan {
        start_line: left.start_line,
        start_col: left.start_col,
        end_line: right.end_line,
        end_col: right.end_col,
    }
}

/// Attaches harvested leading comments to whichever declaration variant was
/// parsed, preserving top-to-bottom order.
fn attach_leading_comments(declaration: Declaration, comments: Vec<CommentNote>) -> Declaration {
    if comments.is_empty() {
        return declaration;
    }
    let mut declaration = declaration;
    let slot = match &mut declaration {
        Declaration::Package(package) => &mut package.comments,
        Declaration::Import(import) => &mut import.comments,
        Declaration::GenericDefinition(definition) => &mut definition.comments,
        Declaration::GenericUsage(usage) => &mut usage.comments,
        Declaration::Alias(alias) => &mut alias.comments,
    };
    let mut comments = comments;
    comments.append(slot);
    *slot = comments;
    declaration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_classifier_and_feature() {
        let module = parse_kerml(
            "package Demo {
                classifier Vehicle {
                    feature engine : Engine;
                }
                classifier Engine;
            }",
        )
        .unwrap();

        let package = module.package.unwrap();
        assert_eq!(package.name.as_dot_string(), "Demo");
        assert_eq!(package.members.len(), 2);
    }

    #[test]
    fn parses_wildcard_imports() {
        let module = parse_kerml(
            "package Demo {
                import Domain::Vehicles::*;
                import Domain.Analysis.**;
            }",
        )
        .unwrap();

        let package = module.package.unwrap();
        let imports = package.imports;
        assert_eq!(imports[0].path.as_dot_string(), "Domain.Vehicles.*");
        assert_eq!(imports[1].path.as_dot_string(), "Domain.Analysis.**");
    }

    #[test]
    fn parses_feature_relationship_tails() {
        let module = parse_kerml(
            "package Demo {
                classifier Vehicle {
                    feature base;
                    feature engine : Engine :> poweredFeature subsets base redefines oldEngine;
                }
                classifier Engine;
            }",
        )
        .unwrap();

        let package = module.package.unwrap();
        let classifier = match &package.members[0] {
            Declaration::GenericDefinition(definition) => definition,
            _ => panic!("expected classifier"),
        };
        let feature = match &classifier.members[1] {
            Declaration::GenericUsage(usage) => usage,
            _ => panic!("expected feature"),
        };
        assert_eq!(feature.name, "engine");
        assert_eq!(feature.specializes[0].as_dot_string(), "poweredFeature");
        assert_eq!(feature.subsets[0].as_dot_string(), "base");
        assert_eq!(feature.redefines[0].as_dot_string(), "oldEngine");
    }
}
