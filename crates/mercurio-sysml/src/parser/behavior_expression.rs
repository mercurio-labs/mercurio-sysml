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
