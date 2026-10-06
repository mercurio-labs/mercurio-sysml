//! Expression grammar additions from KerMLExpressions.xtext, Pilot 2026-08.
//! Precedence and operand structure are retained; unsupported evaluation is explicit.
use super::*;

impl Parser {
    pub(super) fn parse_conditional_expression(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.peek_kind(), TokenKind::Identifier(value) if value == "if") {
            self.advance();
            let condition = self.parse_coalescing_expression()?;
            self.expect(TokenKind::Question, "expected `?` after conditional test")?;
            let yes = self.parse_expression()?;
            self.expect_identifier_named("else", "expected `else` in conditional expression")?;
            let no = self.parse_expression()?;
            return Ok(expression_operation("if", vec![condition, yes, no]));
        }
        self.parse_coalescing_expression()
    }
    pub(super) fn parse_coalescing_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_expression_chain(Self::parse_implication_expression, &["??"])
    }
    pub(super) fn parse_implication_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_expression_chain(Self::parse_or_expression, &["implies"])
    }
    pub(super) fn parse_xor_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_expression_chain(Self::parse_and_expression, &["xor"])
    }
    pub(super) fn parse_expression_chain(
        &mut self,
        operand: fn(&mut Self) -> Result<Expr, Diagnostic>,
        operators: &[&str],
    ) -> Result<Expr, Diagnostic> {
        let mut result = operand(self)?;
        loop {
            let mut width = 1;
            let operator = match (self.peek_kind(), self.next_kind()) {
                (TokenKind::DoubleEquals, Some(TokenKind::Equals)) => {
                    width = 2;
                    "===".to_string()
                }
                (TokenKind::BangEquals, Some(TokenKind::Equals)) => {
                    width = 2;
                    "!==".to_string()
                }
                (TokenKind::Question, Some(TokenKind::Question)) => {
                    width = 2;
                    "??".to_string()
                }
                _ => token_text(self.peek_kind()),
            };
            if !operators.contains(&operator.as_str()) {
                break;
            }
            for _ in 0..width {
                self.advance();
            }
            let right = operand(self)?;
            result = expression_operation(&operator, vec![result, right]);
        }
        Ok(result)
    }
    pub(super) fn parse_classification_expression(&mut self) -> Result<Expr, Diagnostic> {
        let result = self.parse_comparison_expression()?;
        let operator = match self.peek_kind() {
            TokenKind::At => {
                if matches!(self.next_kind(), Some(TokenKind::At)) {
                    self.advance();
                    "@@".to_string()
                } else {
                    "@".to_string()
                }
            }
            TokenKind::Identifier(value)
                if matches!(value.as_str(), "as" | "hastype" | "istype" | "meta") =>
            {
                value.clone()
            }
            _ => return Ok(result),
        };
        self.advance();
        let reference = Expr::TypeReference(self.parse_qualified_name()?);
        Ok(expression_operation(&operator, vec![result, reference]))
    }
    pub(super) fn parse_lambda_expression(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect(TokenKind::LBrace, "expected expression body")?;
        let mut parameters = Vec::new();
        while matches!(self.peek_kind(), TokenKind::Identifier(value) if matches!(value.as_str(), "in" | "out" | "inout" | "return" | "feature"))
        {
            let token = self.current().clone();
            let keyword = token_text(&token.kind);
            self.advance();
            let mut modifiers = vec![keyword.clone()];
            while matches!(self.peek_kind(), TokenKind::Identifier(value) if matches!(value.as_str(), "ref" | "var" | "readonly" | "derived" | "composite"))
            {
                modifiers.push(token_text(self.peek_kind()));
                self.advance();
            }
            let name = if matches!(self.peek_kind(), TokenKind::Identifier(_)) {
                self.expect_identifier("expected parameter name")?
            } else {
                format!("parameter_{}", parameters.len())
            };
            let tail = self.parse_usage_tail(&[])?;
            tail.append_value_modifiers(&mut modifiers);
            let end = self.expect(
                TokenKind::Semicolon,
                "expected `;` after expression parameter",
            )?;
            parameters.push(GenericUsageDecl {
        annotation_targets: Vec::new(),
                keyword: keyword.clone(),
                name,
                is_implicit_name: false,
                ty: tail.ty,
                expression: tail.expression,
                multiplicity: tail.multiplicity,
                additional_types: tail.additional_types,
                specializes: tail.specializes,
                subsets: tail.subsets,
                redefines: tail.redefines,
                body_members: tail.body_members,
                reference_target: None,
                allocation_source: None,
                allocation_target: None,
                metadata_properties: BTreeMap::new(),
                comments: Vec::new(),
                docs: tail.owner_docs,
                modifiers,
                span: merge_span(&token.span, &end.span),
            });
        }
        let body = self.parse_expression()?;
        if matches!(self.peek_kind(), TokenKind::Semicolon) {
            self.advance();
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` after expression result")?;
        Ok(Expr::Lambda {
            parameters,
            body: Box::new(body),
            span: merge_span(&start.span, &end.span),
        })
    }
}
