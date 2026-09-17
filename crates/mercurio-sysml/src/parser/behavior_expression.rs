//! Lower the existing parser's expression AST for dynamic snapshot bindings.
use super::*;
use mercurio_foundation::{
    BinaryExpressionOp as Op, ExpressionIr, ExpressionPathRoot, ExpressionPathSegment,
    UnaryExpressionOp,
};

pub(crate) fn expression(text: &str) -> Result<Value, Diagnostic> {
    let mut parser = Parser::new(lex(text)?, false);
    let ast = parser.parse_expression()?;
    if !matches!(parser.peek_kind(), TokenKind::Eof) {
        return Err(parser.error_here("unsupported trailing expression syntax"));
    }
    lower(&ast)?
        .to_value()
        .map_err(|error| Diagnostic::new(error.to_string(), None))
}

pub(crate) fn assignment(text: &str) -> Result<(String, Value), Diagnostic> {
    let mut parser = Parser::new(lex(text)?, false);
    parser.expect_identifier_named("assign", "expected assignment effect")?;
    let target = parser.parse_qualified_name()?.as_dot_string();
    parser.expect(TokenKind::Colon, "expected := after assignment target")?;
    parser.expect(TokenKind::Equals, "expected := after assignment target")?;
    let ast = parser.parse_expression()?;
    if !matches!(parser.peek_kind(), TokenKind::Eof) {
        return Err(parser.error_here("unsupported trailing assignment syntax"));
    }
    let target = target.strip_prefix("self.").unwrap_or(&target);
    if target.contains('.') {
        return Err(Diagnostic::new(
            "only local feature assignment targets are supported",
            None,
        ));
    }
    Ok((
        target.to_string(),
        lower(&ast)?
            .to_value()
            .map_err(|error| Diagnostic::new(error.to_string(), None))?,
    ))
}

fn lower(expr: &Expr) -> Result<ExpressionIr, Diagnostic> {
    Ok(match expr {
        Expr::Literal(value) => ExpressionIr::Literal {
            value: match value {
                LiteralExpr::Integer(n) => Value::from(*n),
                LiteralExpr::Real(n) => {
                    let number = n
                        .parse::<f64>()
                        .map_err(|_| Diagnostic::new("invalid real", None))?;
                    if !number.is_finite() {
                        return Err(Diagnostic::new("non-finite real", None));
                    }
                    Value::from(number)
                }
                LiteralExpr::Boolean(b) => Value::Bool(*b),
                LiteralExpr::String(s) => Value::String(s.clone()),
            },
        },
        Expr::SelfRef(_) => ExpressionIr::SelfRef,
        Expr::Name(name) => ExpressionIr::Path {
            root: ExpressionPathRoot::SelfRef,
            segments: name
                .segments
                .iter()
                .map(|name| ExpressionPathSegment::Name(name.clone()))
                .collect(),
        },
        Expr::Path { root, segment, .. } => {
            let mut segments = match lower(root)? {
                ExpressionIr::SelfRef => Vec::new(),
                ExpressionIr::Path { segments, .. } => segments,
                _ => return Err(Diagnostic::new("unsupported expression path root", None)),
            };
            segments.push(ExpressionPathSegment::Name(segment.clone()));
            ExpressionIr::Path {
                root: ExpressionPathRoot::SelfRef,
                segments,
            }
        }
        Expr::Tuple { items, .. } => ExpressionIr::Tuple {
            items: items.iter().map(lower).collect::<Result<_, _>>()?,
        },
        Expr::Unary { op, expr, .. } => ExpressionIr::Unary {
            op: match op {
                UnaryOp::Negate => UnaryExpressionOp::Negate,
                UnaryOp::Not => UnaryExpressionOp::Not,
            },
            expr: Box::new(lower(expr)?),
        },
        Expr::Binary {
            left, op, right, ..
        } => ExpressionIr::Binary {
            left: Box::new(lower(left)?),
            right: Box::new(lower(right)?),
            op: match op {
                BinaryOp::Add => Op::Add,
                BinaryOp::Subtract => Op::Subtract,
                BinaryOp::Multiply => Op::Multiply,
                BinaryOp::Divide => Op::Divide,
                BinaryOp::Power => Op::Power,
                BinaryOp::Equal => Op::Equal,
                BinaryOp::NotEqual => Op::NotEqual,
                BinaryOp::Less => Op::Less,
                BinaryOp::LessEqual => Op::LessEqual,
                BinaryOp::Greater => Op::Greater,
                BinaryOp::GreaterEqual => Op::GreaterEqual,
                BinaryOp::And => Op::And,
                BinaryOp::Or => Op::Or,
            },
        },
        Expr::Call { function, args, .. } => ExpressionIr::Call {
            function: function.clone(),
            args: args.iter().map(lower).collect::<Result<_, _>>()?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_foundation::{ExpressionEvaluationContext, ExpressionEvaluationError};
    use serde_json::json;

    struct Bindings;
    impl ExpressionEvaluationContext for Bindings {
        fn owner_id(&self) -> &str {
            "test"
        }
        fn resolve_path(
            &mut self,
            path: &[ExpressionPathSegment],
        ) -> Result<Vec<Value>, ExpressionEvaluationError> {
            let name = path
                .iter()
                .map(ExpressionPathSegment::name)
                .collect::<Vec<_>>()
                .join(".");
            match name.as_str() {
                "items" => Ok(vec![json!(10), json!(20), json!(30)]),
                _ => Err(ExpressionEvaluationError::MissingBinding(name)),
            }
        }
    }

    #[test]
    fn expression_conformance_supported_syntax_and_precedence() {
        for (source, expected) in [
            ("1e3", json!(1000.0)),
            (".25E+2", json!(25.0)),
            ("2.5e-2", json!(0.025)),
            ("+2", json!(2)),
            ("7 % 3", json!(1)),
            ("1..3", json!([1, 2, 3])),
            ("1 + 1..3 + 1", json!([2, 3, 4])),
            ("items#(2)", json!(20)),
            ("(10,20,30)#(1 + 1)", json!(20)),
            ("null ?? 3", json!(3)),
            ("if false ? 1 / 0 else 9", json!(9)),
            ("if true ? if false ? 1 else 2 else 3", json!(2)),
            ("false implies (1 / 0 > 0)", json!(true)),
            ("false implies true implies false", json!(false)),
            ("true | false & false", json!(true)),
            ("true xor true and false", json!(true)),
            ("false and (1 / 0 > 0)", json!(false)),
            ("true or (1 / 0 > 0)", json!(true)),
            ("-2 ** 2", json!(4)),
            ("2 ** 3 ** 2", json!(512)),
            ("((1,2),3)", json!([1, 2, 3])),
            ("null", json!([])),
        ] {
            let ir = ExpressionIr::from_value(&expression(source).unwrap()).unwrap();
            let actual = ir
                .evaluate(&mut Bindings)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            // JSON distinguishes 2 from 2.0 even though the language does not.
            if let (Some(actual), Some(expected)) = (actual.as_f64(), expected.as_f64()) {
                assert_eq!(actual, expected, "{source}");
            } else {
                assert_eq!(actual, expected, "{source}");
            }
            let module = super::super::parse_sysml(&format!(
                "package Demo {{ attribute result = {source}; }}"
            ))
            .unwrap();
            let usage = module.package.unwrap().members[0].as_usage_like().unwrap();
            let attribute_ir = lower(usage.expression.as_ref().unwrap()).unwrap();
            assert_eq!(
                attribute_ir, ir,
                "attribute / dynamic parser disagreement: {source}"
            );
        }
    }

    #[test]
    fn expression_conformance_unsupported_syntax_is_never_truncated() {
        for source in [
            "1e",
            "1e+",
            "1e-",
            "1 trailing",
            "items[1]",
            "2[m]",
            "items.?{in v; v > 1}",
            "items.{in v; v + 1}",
            "items as Integer",
            "items istype Integer",
            "f(x = 1)",
            "items->sum()",
            "1 === 1",
            "1..2..3",
            "1 ? 2 else 3",
        ] {
            assert!(
                expression(source).is_err(),
                "dynamic expression accepted {source}"
            );
            assert!(
                super::super::parse_sysml(&format!(
                    "package Demo {{ attribute result = {source}; }}"
                ))
                .is_err(),
                "attribute expression accepted {source}"
            );
            assert!(
                super::super::parse_sysml(&format!(
                    "package Demo {{ assert constraint {{ {source} }} }}"
                ))
                .is_err(),
                "constraint expression accepted {source}"
            );
            assert!(
                super::super::parse_sysml(&format!(
                    "package Demo {{ constraint c {{ {source} }} }}"
                ))
                .is_err(),
                "named constraint expression accepted {source}"
            );
        }
    }

    #[test]
    fn expression_conformance_reconstructed_behavior_tokens_keep_operators() {
        for source in [
            "items#(2)",
            "null ?? 3",
            "if true ? 7 % 4 else 0",
            "1..3",
            r#""a\nb" == "a\nb""#,
        ] {
            let reconstructed = lex(source)
                .unwrap()
                .iter()
                .filter(|token| !matches!(token.kind, TokenKind::Eof))
                .map(|token| super::super::token_text(&token.kind))
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(
                expression(source).unwrap(),
                expression(&reconstructed).unwrap(),
                "{source} -> {reconstructed}"
            );
        }
    }
}
