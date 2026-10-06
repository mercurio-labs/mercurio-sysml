//! KerML.xtext Connector, BindingConnector and Succession declarations.
//! Endpoint features retain their reference subsettings and source order.
use super::*;

impl Parser {
    pub(super) fn parse_connector(
        &mut self,
        docs: Vec<String>,
        modifiers: Vec<String>,
    ) -> Result<GenericUsageDecl, Diagnostic> {
        let start_index = self.index;
        let start = self.current().clone();
        let keyword = token_identifier(&start).to_string();
        let prefix = match keyword.as_str() {
            "binding" => "of",
            "succession" => "first",
            _ => "from",
        };
        let separator = match keyword.as_str() {
            "binding" => "=",
            "succession" => "then",
            _ => "to",
        };
        // A header precedes from/of/first or a n-ary endpoint tuple. Without
        // that marker, a binary declaration starts directly with its first end.
        let mut marker = None;
        let mut anonymous_binary = false;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        for index in start_index + 1..self.tokens.len() {
            match &self.tokens[index].kind {
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LAngle => angle_depth += 1,
                TokenKind::RAngle => angle_depth = angle_depth.saturating_sub(1),
                _ if bracket_depth > 0 || angle_depth > 0 => {}
                TokenKind::Semicolon | TokenKind::LBrace | TokenKind::RBrace | TokenKind::Eof => {
                    break;
                }
                TokenKind::Identifier(word) if word == prefix => {
                    marker = Some(index);
                    break;
                }
                TokenKind::LParen if keyword == "connector" => {
                    marker = Some(index);
                    break;
                }
                TokenKind::Identifier(word) if word == separator => {
                    anonymous_binary = true;
                    break;
                }
                TokenKind::Equals => {
                    anonymous_binary = keyword == "binding";
                    break;
                }
                _ => {}
            }
        }
        if marker.is_none() && !anonymous_binary {
            return self.parse_feature_with_modifiers(docs, modifiers);
        }
        let mut declaration = if let Some(marker) = marker {
            let mut header = self.tokens[start_index..marker].to_vec();
            let mut terminator = self.tokens[marker].clone();
            terminator.kind = TokenKind::Semicolon;
            header.push(terminator.clone());
            terminator.kind = TokenKind::Eof;
            header.push(terminator);
            let result = Parser::new(header).parse_feature_with_modifiers(docs, modifiers)?;
            self.index = marker;
            result
        } else {
            self.advance();
            let mut modifiers = modifiers;
            modifiers.extend(self.parse_modifiers());
            GenericUsageDecl {
        annotation_targets: Vec::new(),
                keyword,
                name: format!(
                    "connector_{}_{}",
                    start.span.start_line, start.span.start_col
                ),
                is_implicit_name: true,
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
                span: start.span.clone(),
            }
        };
        if matches!(self.peek_kind(), TokenKind::LParen) {
            self.advance();
            declaration
                .body_members
                .push(Declaration::GenericUsage(self.parse_connector_end()?));
            self.expect(
                TokenKind::Comma,
                "a connector tuple needs at least two ends",
            )?;
            loop {
                declaration
                    .body_members
                    .push(Declaration::GenericUsage(self.parse_connector_end()?));
                if !matches!(self.peek_kind(), TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::RParen, "expected `)` after connector ends")?;
        } else {
            if matches!(self.peek_kind(), TokenKind::Identifier(word) if word == prefix) {
                self.advance();
            }
            declaration
                .body_members
                .push(Declaration::GenericUsage(self.parse_connector_end()?));
            if separator == "=" {
                self.expect(TokenKind::Equals, "expected `=` between binding ends")?;
            } else {
                self.expect_identifier_named(separator, "expected connector end separator")?;
            }
            declaration
                .body_members
                .push(Declaration::GenericUsage(self.parse_connector_end()?));
        }
        let end = if matches!(self.peek_kind(), TokenKind::LBrace) {
            self.advance();
            while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
                let Some(member) = self.parse_declaration()? else {
                    break;
                };
                declaration.body_members.push(member);
            }
            self.expect(TokenKind::RBrace, "expected `}` after connector body")?
        } else {
            self.expect(TokenKind::Semicolon, "expected `;` or a connector body")?
        };
        declaration.span = merge_span(&start.span, &end.span);
        Ok(declaration)
    }

    fn parse_connector_end(&mut self) -> Result<GenericUsageDecl, Diagnostic> {
        let start = self.current().clone();
        let multiplicity = self.parse_optional_multiplicity()?;
        let explicit = matches!(self.next_kind(), Some(TokenKind::Identifier(word)) if word == "references")
            || (matches!(self.next_kind(), Some(TokenKind::ScopeSep))
                && matches!(
                    self.tokens.get(self.index + 2).map(|token| &token.kind),
                    Some(TokenKind::RAngle)
                ));
        let name = if explicit {
            let name =
                token_name(&self.expect_name_token("expected connector end name")?).to_string();
            if matches!(self.peek_kind(), TokenKind::ScopeSep) {
                self.advance();
                self.advance();
            } else {
                self.expect_identifier_named("references", "expected `references`")?;
            }
            name
        } else {
            format!("end_{}_{}", start.span.start_line, start.span.start_col)
        };
        let reference = self.parse_qualified_name()?;
        let span = merge_span(&start.span, &reference.span);
        let mut modifiers = vec!["end".to_string()];
        if multiplicity.is_some() {
            modifiers.push("crossing_multiplicity".to_string());
        }
        Ok(GenericUsageDecl {
        annotation_targets: Vec::new(),
            keyword: "feature".into(),
            name,
            is_implicit_name: !explicit,
            ty: None,
            reference_target: Some(reference),
            allocation_source: None,
            allocation_target: None,
            metadata_properties: Default::default(),
            multiplicity,
            expression: None,
            additional_types: Vec::new(),
            specializes: Vec::new(),
            subsets: Vec::new(),
            redefines: Vec::new(),
            body_members: Vec::new(),
            comments: Vec::new(),
            docs: Vec::new(),
            modifiers,
            span,
        })
    }
}
