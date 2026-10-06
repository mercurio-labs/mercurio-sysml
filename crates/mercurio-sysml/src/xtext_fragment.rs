//! Native execution of a bounded, resolved Xtext fragment program.
//! Control flow and attribute assignments come from the imported definitions.
//! Token/value adaptation, diagnostics and the authoring-AST bridge are explicit
//! handwritten boundaries. This is not general ANTLR prediction or recovery.
use crate::language_frontend::lowering::{
    ecore_model, relationship_declarations::metaclass_conforms,
};
use mercurio_foundation::language_contracts::{
    diagnostics::Diagnostic,
    lexer::{Token, TokenKind},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

#[path = "xtext_value_dispatch_generated.rs"]
mod value_dispatch;

/// Explicit native implementations for upstream-selected scalar converters.
/// Callers have already recognized the grammar. Identity preserves lexical
/// strings (or represented names supplied by the named semantic adapter).
fn converted_value(
    algorithm: value_dispatch::Algorithm,
    value: Value,
) -> Result<(Value, Option<String>), String> {
    use value_dispatch::Algorithm;
    let spelling = value.as_str().ok_or("Missing scalar spelling")?;
    let converted = match algorithm {
        Algorithm::Identity => return Ok((value, None)),
        Algorithm::Nonunique if spelling == "nonunique" => Value::Bool(false),
        Algorithm::Nonunique => return Err("Unsupported Nonunique spelling".into()),
        Algorithm::Boolean => Value::Bool(match spelling {
            "true" => true,
            "false" => false,
            _ => return Err("Unsupported EBoolean spelling".into()),
        }),
        Algorithm::Integer => Value::from(
            spelling
                .parse::<i32>()
                .map_err(|_| "Value outside EInt domain")?,
        ),
        Algorithm::FiniteDouble => {
            let number = spelling
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .ok_or("Value outside supported finite EDouble domain")?;
            return Ok((Value::Number(number), Some(spelling.into())));
        }
    };
    Ok((converted, None))
}

#[derive(Deserialize)]
struct Programs {
    #[serde(default)]
    experimental_graph: Option<crate::xtext_nfa_experiment::Graph>,
    #[serde(default)]
    experimental_guards: BTreeMap<String, DecisionCondition>,
    #[serde(default)]
    language_programs: BTreeMap<String, Programs>,
    #[serde(default)]
    entry_rules: BTreeMap<String, String>,
    rules: BTreeMap<String, Rule>,
    #[serde(default)]
    capture_actions: BTreeMap<String, Node>,
    #[serde(default)]
    operator_captures: BTreeMap<String, String>,
    #[serde(default)]
    literal_roots: Vec<String>,
}
impl Programs {
    fn language(&self, kerml: bool) -> Result<&Programs, String> {
        self.language_programs
            .get(if kerml { "kerml" } else { "sysml" })
            .ok_or_else(|| "Missing Xtext language program".into())
    }
}

#[derive(Deserialize)]
struct Rule {
    #[serde(default)]
    converter: Option<String>,
    owner: String,
    scalar: bool,
    #[serde(default)]
    construct: bool,
    body: Node,
}
// Imported upstream DFA state numbers are normalized during export. The Rust
// interpreter is shared; lexer-carrier adaptation remains explicitly handwritten.
#[derive(Deserialize)]
struct Decision {
    #[serde(default)]
    experimental_nfa: Option<String>,
    exit_alternative: Option<usize>,
    start: usize,
    states: Vec<DecisionState>,
    keywords: Vec<String>,
}
#[derive(Deserialize)]
struct DecisionState {
    #[serde(default)]
    gate: Option<DecisionCondition>,
    #[serde(default)]
    predicate_edges: Vec<PredicateEdge>,
    accept: Option<usize>,
    edges: BTreeMap<String, usize>,
}

#[derive(Deserialize)]
struct PredicateEdge {
    condition: DecisionCondition,
    target: usize,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DecisionCondition {
    Always,
    Syntax {
        #[serde(default)]
        first_set: bool,
        source_id: String,
        probe: Option<Box<Node>>,
    },
}

#[derive(Deserialize)]
struct Node {
    #[serde(default)]
    unsupported: Option<String>,
    #[serde(default)]
    decision: Option<Decision>,
    #[serde(default)]
    prediction: Vec<Vec<String>>,
    #[serde(default)]
    predicated: bool,
    #[serde(default)]
    first_set_predicated: bool,
    id: String,
    cardinality: String,
    #[serde(flatten)]
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Operation {
    Keyword {
        value: String,
    },
    Name,
    TokenDatatype {
        syntax: Box<Node>,
    },
    CrossReference {
        target: String,
        terminal: Box<Node>,
    },
    Contain {
        feature: String,
        feature_id: String,
        operator: String,
        terminal: Box<Node>,
    },
    AppendOperand {
        feature_id: String,
        terminal: Box<Node>,
    },
    Link {
        feature: String,
        feature_id: String,
        operator: String,
        terminal: Box<Node>,
    },
    Datatype {
        syntax: Box<Node>,
    },
    Create {
        classifier: String,
    },
    Capture {
        classifier: String,
        feature_id: String,
        captured_types: Vec<String>,
    },
    Terminal {
        rule: String,
        carrier: String,
    },
    Enum {
        token: String,
        literal: String,
    },
    Sequence {
        elements: Vec<Node>,
    },
    Choice {
        elements: Vec<Node>,
    },
    Call {
        rule: String,
    },
    Assign {
        feature: String,
        feature_id: String,
        operator: String,
        terminal: Box<Node>,
    },
}

#[path = "xtext_fragment_machine.rs"]
mod machine;

fn programs() -> Result<&'static Programs, String> {
    static PROGRAMS: OnceLock<Result<Programs, String>> = OnceLock::new();
    PROGRAMS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
            ))
            .map_err(|e| format!("Invalid embedded Xtext fragment program: {e}"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Preserve the legacy Number carrier, deriving its real-number shape from
/// resolved Xtext syntax. Numeric token arbitration is checked against the
/// original remaining source, including incomplete committed exponents.
pub(crate) fn numeric_prefix(input: &str) -> Result<usize, &'static str> {
    let rules = &programs()
        .map_err(|_| "invalid numeric grammar program")?
        .rules;
    let rule = rules
        .get("org.omg.kerml.expressions.xtext.KerMLExpressions::RealValue")
        .ok_or("missing imported RealValue rule")?;
    let Operation::Datatype { syntax } = &rule.body.operation else {
        return Err("invalid imported RealValue body");
    };
    let (_, first) = crate::xtext_terminal::numeric_component(input)?;
    let real = Executor::lexical(syntax, input, 0)
        .map_err(|_| "invalid imported numeric syntax")?
        .into_iter()
        .max()
        .unwrap_or(0);
    let length = first.max(real);
    let mut cursor = 0;
    while cursor < length {
        let (_, component) = crate::xtext_terminal::numeric_component(&input[cursor..])?;
        cursor += component;
    }
    if cursor != length {
        return Err("numeric syntax splits a committed token");
    }
    Ok(length)
}

fn check_capture(
    classifier: &str,
    feature_id: &str,
    captured_types: &[String],
    current: &str,
) -> Result<(), String> {
    let target = classifier
        .rsplit('/')
        .next()
        .ok_or("Invalid capture classifier")?;
    let feature = ecore_model::feature(target, "operand").ok_or("Missing capture feature")?;
    if feature.id != feature_id
        || captured_types.is_empty()
        || !captured_types
            .iter()
            .any(|t| metaclass_conforms(current, t.rsplit('/').next().unwrap()))
    {
        return Err("Capture current-type/feature binding mismatch".into());
    }
    ecore_model::validate_reference_endpoint(target, "operand", current)?;
    Ok(())
}

/// Exact source action identity, selected only when the adjacent pinned operator
/// assignment has a unique binding. This does not claim enclosing-rule parsing.
pub(crate) fn operator_capture(operator: &str) -> Result<Option<&'static str>, String> {
    Ok(programs()?
        .operator_captures
        .get(operator)
        .map(String::as_str))
}

pub(crate) fn capture_classifier(action: &str, current: &str) -> Result<&'static str, String> {
    let node = programs()?
        .capture_actions
        .get(action)
        .ok_or("Unknown pinned capture action")?;
    let Operation::Capture {
        classifier,
        feature_id,
        captured_types,
    } = &node.operation
    else {
        return Err("Action is not a capture".into());
    };
    check_capture(classifier, feature_id, captured_types, current)?;
    Ok(classifier.rsplit('/').next().unwrap())
}

/// A syntax-level reference. Its Ecore target is known; resolution, ambiguity,
/// external loading and identity-based uniqueness belong to the native linker.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingReference {
    pub target_type: String,
    pub spelling: String,
    pub span: Option<Box<mercurio_foundation::language_contracts::ast::SourceSpan>>,
}

/// Grammar order across stored containment and delegate-generated operands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ConstructionStep {
    Child { field: String, index: usize },
    Operand(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Match {
    pub consumed: usize,
    pub object_kind: Option<String>,
    pub fields: BTreeMap<String, Value>,
    // Completed syntax objects are immutable. Branch snapshots share descendants;
    // their scalar fields, links and construction order stay private to each root.
    pub children: BTreeMap<String, Vec<Arc<Match>>>,
    pub links: BTreeMap<String, Vec<PendingReference>>,
    pub container_feature: Option<String>,
    // Staged semantic append commands, NOT stored Ecore operand containment.
    pub captured_operands: Vec<Arc<Match>>,
    pub construction_order: Box<Vec<ConstructionStep>>,
    reference: Option<PendingReference>,
    value: Option<Value>,
    lexeme: Option<String>,
}

struct Executor<'a> {
    programs: &'a Programs,
    tokens: &'a [Token],
    kind: &'a str,
    kerml: bool,
    speculative: bool,
    call_stack: &'a RefCell<Vec<(String, usize)>>,
    // Successful top-level syntax decisions are pure for this immutable input.
    prediction_cache: &'a RefCell<BTreeMap<(String, usize, bool), bool>>,
}

// Keep error formatting out of recursive evaluator frames.
#[inline(never)]
fn ensure_prediction_supported(node: &Node) -> Result<(), String> {
    match &node.unsupported {
        Some(reason) => Err(format!("Unimplemented prediction at {}: {reason}", node.id)),
        None => Ok(()),
    }
}

// Syntax-only lookahead deliberately has no model state or Ecore services.
// Actions are epsilon; assignments recognize only their terminal. A predicate
// recognizes one occurrence, even when the guarded node itself is repeated.
struct SyntaxProbe<'a, 'p> {
    executor: &'a Executor<'p>,
    active: Vec<(String, usize)>,
    memo: BTreeMap<(String, usize), BTreeSet<usize>>,
}

#[path = "xtext_syntax_probe.rs"]
mod syntax_probe;

// The existing lexer retains the raw interior spelling but omits quote
// delimiters from Identifier values. Its inclusive source span includes those
// delimiters. Keep this adapter explicit until tokens carry their terminal ID.
fn quoted_identifier(token: &Token, value: &str) -> bool {
    token.span.start_line != token.span.end_line
        || token.span.end_col.saturating_sub(token.span.start_col) + 1 != value.chars().count()
}
impl Executor<'_> {
    fn rule(&self, identity: &str, state: &Match) -> Result<Option<Match>, String> {
        machine::run(self, identity, state)
    }

    fn finish_rule_value(
        &self,
        rule: &Rule,
        identity: &str,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        let Some(mut matched) = completed else {
            return Ok(None);
        };
        if rule.scalar {
            let algorithm = value_dispatch::algorithm(self.kerml, identity, &rule.owner)
                .ok_or_else(|| format!("No qualified upstream converter binding in {identity}"))?;
            if rule.converter.as_deref().is_some_and(|converter| {
                converter != "pilot_nonunique_boolean"
                    || algorithm != value_dispatch::Algorithm::Nonunique
            }) {
                return Err(format!("Changed custom scalar converter in {identity}"));
            }
            let value = matched
                .value
                .take()
                .ok_or_else(|| format!("Missing scalar value in {identity}"))?;
            let (value, lexeme) = converted_value(algorithm, value)
                .map_err(|error| format!("{error} in {identity}"))?;
            matched.value = Some(value);
            if lexeme.is_some() {
                matched.lexeme = lexeme;
            }
        }
        Ok(Some(matched))
    }

    fn node(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        match node.cardinality.as_str() {
            "" => self.body(node, state),
            "?" => Ok(Some(
                self.body(node, state)?.unwrap_or_else(|| state.clone()),
            )),
            "*" | "+" => {
                let mut current = state.clone();
                let mut count = 0;
                while let Some(next) = self.body(node, &current)? {
                    if next.consumed <= current.consumed {
                        return Err(format!("Non-progressing Xtext repetition {}", node.id));
                    }
                    current = next;
                    count += 1;
                }
                Ok((count > 0 || node.cardinality == "*").then_some(current))
            }
            other => Err(format!("Unsupported Xtext cardinality {other}")),
        }
    }

    // Keep the legacy parser's partial token renderer out of grammar matching.
    // These carriers are emitted by the lexer but omitted by token_text.
    fn token_spelling(kind: &TokenKind) -> String {
        let missing = match kind {
            TokenKind::Package => Some("package"),
            TokenKind::Import => Some("import"),
            TokenKind::Part => Some("part"),
            TokenKind::Def => Some("def"),
            TokenKind::LBrace => Some("{"),
            TokenKind::RBrace => Some("}"),
            TokenKind::Dollar => Some("$"),
            TokenKind::At => Some("@"),
            TokenKind::Hash => Some("#"),
            TokenKind::Question => Some("?"),
            TokenKind::Semicolon => Some(";"),
            _ => None,
        };
        missing
            .map(str::to_owned)
            .unwrap_or_else(|| crate::parser::token_text(kind))
    }

    fn lexical_spelling(token: &Token) -> String {
        // This legacy carrier merges the word and shorthand. Its source span
        // distinguishes the two exact spellings admitted by the shared scanner.
        if matches!(token.kind, TokenKind::Specializes)
            && token.span.start_line == token.span.end_line
            && token.span.end_col.checked_sub(token.span.start_col) == Some("specializes".len() - 1)
        {
            return "specializes".into();
        }
        Self::token_spelling(&token.kind)
    }

    fn lexer_keyword(&self, offset: usize) -> Option<&'static str> {
        let limit = crate::xtext_terminal::keyword_limit(self.kerml).ok()?;
        let mut text = String::new();
        let mut previous: Option<&Token> = None;
        for token in self.tokens.get(offset..)? {
            if let Some(prior) = previous {
                if token.span.start_line != prior.span.end_line
                    || token.span.start_col != prior.span.end_col + 1
                {
                    break;
                }
            }
            if matches!(
                token.kind,
                TokenKind::Eof | TokenKind::String(_) | TokenKind::BlockDoc(_) | TokenKind::Doc(_)
            ) {
                break;
            }
            if let TokenKind::Identifier(value) = &token.kind {
                if quoted_identifier(token, value) {
                    break;
                }
            }
            let piece = Self::lexical_spelling(token);
            if piece.is_empty() {
                break;
            }
            text.push_str(&piece);
            if text.len() >= limit
                || matches!(
                    token.kind,
                    TokenKind::Identifier(_)
                        | TokenKind::Number(_)
                        | TokenKind::Package
                        | TokenKind::Import
                        | TokenKind::Part
                        | TokenKind::Def
                )
            {
                break;
            }
            previous = Some(token);
        }
        if text.is_empty() {
            return None;
        }
        crate::xtext_terminal::keyword_prefix(self.kerml, &text)
            .ok()
            .flatten()
    }

    fn literal(&self, spelling: &str, state: &Match) -> Option<Match> {
        // The legacy lexer splits some punctuation keywords (for example '..').
        // Assemble only adjacent punctuation, never tokens separated by trivia
        // or quoted identifiers. This adapter is separate from token arbitration.
        if spelling
            .as_bytes()
            .first()
            .is_some_and(|c| !c.is_ascii_alphanumeric() && *c != b'_')
            && self.lexer_keyword(state.consumed) != Some(spelling)
        {
            return None;
        }
        let mut text = String::new();
        let mut previous: Option<&Token> = None;
        for (offset, token) in self.tokens[state.consumed..].iter().enumerate() {
            if let Some(prior) = previous {
                if token.span.start_line != prior.span.end_line
                    || token.span.start_col != prior.span.end_col + 1
                {
                    return None;
                }
            }
            if let TokenKind::Identifier(value) = &token.kind {
                if quoted_identifier(token, value) {
                    return None;
                }
            }
            let piece = Self::lexical_spelling(token);
            if piece.is_empty() {
                return None;
            }
            text.push_str(&piece);
            if text == spelling {
                let mut next = state.clone();
                next.consumed += offset + 1;
                next.value = Some(Value::String(spelling.into()));
                return Some(next);
            }
            if !spelling.starts_with(&text)
                || matches!(
                    &token.kind,
                    TokenKind::Identifier(_)
                        | TokenKind::String(_)
                        | TokenKind::Number(_)
                        | TokenKind::Eof
                )
            {
                return None;
            }
            previous = Some(token);
        }
        None
    }

    // Numeric datatype rules concatenate lexical components before conversion.
    // Execute their imported syntax over the legacy Number carrier. Terminal
    // calls use the generated recognizer; no component is converted to EInt.
    fn lexical(node: &Node, input: &str, offset: usize) -> Result<BTreeSet<usize>, String> {
        let body = |start| Self::lexical_body(node, input, start);
        match node.cardinality.as_str() {
            "" => body(offset),
            "?" => {
                let mut ends = body(offset)?;
                ends.insert(offset);
                Ok(ends)
            }
            "*" | "+" => {
                let mut ends = BTreeSet::new();
                let mut pending = vec![offset];
                if node.cardinality == "*" {
                    ends.insert(offset);
                }
                while let Some(start) = pending.pop() {
                    for end in body(start)? {
                        if end <= start {
                            return Err(format!("Non-progressing datatype repetition {}", node.id));
                        }
                        if ends.insert(end) {
                            pending.push(end);
                        }
                    }
                }
                Ok(ends)
            }
            _ => Err(format!("Unsupported datatype cardinality {}", node.id)),
        }
    }

    fn lexical_body(node: &Node, input: &str, offset: usize) -> Result<BTreeSet<usize>, String> {
        match &node.operation {
            Operation::Keyword { value } => Ok(input[offset..]
                .starts_with(value)
                .then_some(offset + value.len())
                .into_iter()
                .collect()),
            Operation::Terminal { rule, .. } => Ok(crate::xtext_terminal::terminal_prefix(
                rule,
                &input[offset..],
            )
            .map(|length| offset + length)
            .into_iter()
            .collect()),
            Operation::Sequence { elements } => {
                let mut positions = BTreeSet::from([offset]);
                for child in elements {
                    let mut next = BTreeSet::new();
                    for position in positions {
                        next.extend(Self::lexical(child, input, position)?);
                    }
                    positions = next;
                    if positions.is_empty() {
                        break;
                    }
                }
                Ok(positions)
            }
            Operation::Choice { elements } => {
                let mut ends = BTreeSet::new();
                for child in elements {
                    ends.extend(Self::lexical(child, input, offset)?);
                }
                Ok(ends)
            }
            _ => Err(format!("Unsupported datatype syntax {}", node.id)),
        }
    }

    // Datatype parser rules may need more than one-token lookahead, e.g.
    // Qualification? Name. Keep every matching end position until the complete
    // rule is matched. This recognizer has no model mutation side effects.
    fn token_ends(&self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        let body = |start| self.token_body_ends(node, start);
        match node.cardinality.as_str() {
            "" => body(offset),
            "?" => {
                let mut ends = body(offset)?;
                ends.insert(offset);
                Ok(ends)
            }
            "*" | "+" => {
                let mut ends = BTreeSet::new();
                let mut pending = vec![offset];
                if node.cardinality == "*" {
                    ends.insert(offset);
                }
                while let Some(start) = pending.pop() {
                    for end in body(start)? {
                        if end <= start {
                            return Err(format!("Non-progressing token datatype {}", node.id));
                        }
                        if ends.insert(end) {
                            pending.push(end);
                        }
                    }
                }
                Ok(ends)
            }
            _ => Err(format!(
                "Unsupported token datatype cardinality {}",
                node.id
            )),
        }
    }

    fn token_body_ends(&self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        match &node.operation {
            Operation::Keyword { .. } | Operation::Name => Ok(self
                .body(
                    node,
                    &Match {
                        consumed: offset,
                        ..Match::default()
                    },
                )?
                .map(|m| m.consumed)
                .into_iter()
                .collect()),
            Operation::TokenDatatype { syntax } => self.token_ends(syntax, offset),
            Operation::Call { rule } => {
                let called = self
                    .programs
                    .rules
                    .get(rule)
                    .ok_or("Unknown token datatype call")?;
                if !called.scalar {
                    return Err("Object call in token datatype".into());
                }
                self.token_ends(&called.body, offset)
            }
            Operation::Sequence { elements } => {
                let mut positions = BTreeSet::from([offset]);
                for child in elements {
                    let mut next = BTreeSet::new();
                    for position in positions {
                        next.extend(self.token_ends(child, position)?);
                    }
                    positions = next;
                    if positions.is_empty() {
                        break;
                    }
                }
                Ok(positions)
            }
            Operation::Choice { elements } => {
                let mut ends = BTreeSet::new();
                for child in elements {
                    ends.extend(self.token_ends(child, offset)?);
                }
                Ok(ends)
            }
            _ => Err(format!("Unsupported token datatype operation {}", node.id)),
        }
    }

    fn validate_reference_count(kind: &str, field: &str, count: usize) -> Result<(), String> {
        let contract = ecore_model::feature(kind, field).ok_or("Missing reference contract")?;
        // New contained objects have distinct identity even with equal values.
        // Deferred references cannot be checked for alias identity until linked.
        let value = if contract.upper == 1 {
            if count != 1 {
                return Err(format!("Singular reference count for {kind}.{field}"));
            }
            Value::String("pending:0".into())
        } else {
            Value::Array(
                (0..count)
                    .map(|i| Value::String(format!("pending:{i}")))
                    .collect(),
            )
        };
        ecore_model::validate_value(kind, field, &value)
    }

    // Separate operation frames keep recursive debug builds within the explicit
    // rule-depth budget; a single large match reserves every arm\'s stack locals.
    fn decision_condition(
        &self,
        condition: &DecisionCondition,
        offset: usize,
    ) -> Result<bool, String> {
        let DecisionCondition::Syntax {
            source_id,
            probe,
            first_set,
        } = condition
        else {
            return Ok(true);
        };
        let node = probe
            .as_ref()
            .ok_or("Unbound prediction syntax predicate")?;
        let key = (format!("predicate::{source_id}"), offset);
        let cache_key = (source_id.clone(), offset, *first_set);
        // Only immutable guards owned by this program may enter the cache.
        // Temporary/ad-hoc conditions can reuse identities with different bodies.
        // Nested guards retain their context-sensitive recursion/depth checks.
        let cacheable = self.programs.experimental_guards.get(source_id)
            .is_some_and(|bound| std::ptr::eq(bound, condition))
            && !self.call_stack.borrow().iter()
                .any(|(name, _)| name.starts_with("predicate::"));
        {
            let mut stack = self.call_stack.borrow_mut();
            if stack.contains(&key)
                || stack
                    .iter()
                    .filter(|(name, _)| name.starts_with("predicate::"))
                    .count()
                    >= 64
            {
                return Err("Recursive or excessive prediction predicate".into());
            }
            if cacheable {
                if let Some(value) = self.prediction_cache.borrow().get(&cache_key) {
                    return Ok(*value);
                }
            }
            stack.push(key);
        }
        // Upstream prediction rewinds to the decision entry for predicates.
        // SyntaxProbe stages no objects and does not advance the model cursor.
        let mut syntax = SyntaxProbe {
            executor: self,
            active: Vec::new(),
            memo: BTreeMap::new(),
        };
        let result = if *first_set {
            syntax.first(node, offset).map(|(matches, _)| matches)
        } else {
            syntax.body(node, offset).map(|ends| !ends.is_empty())
        };
        self.call_stack.borrow_mut().pop();
        if cacheable {
            if let Ok(value) = &result {
                let mut cache = self.prediction_cache.borrow_mut();
                // Saturation affects reuse only; never grammar recognition.
                if cache.len() < 65_536 {
                    cache.insert(cache_key, *value);
                }
            }
        }
        result
    }

    fn decide(&self, decision: &Decision, offset: usize) -> Result<Option<usize>, String> {
        if let Some(identity) = &decision.experimental_nfa {
            let graph = self
                .programs
                .experimental_graph
                .as_ref()
                .ok_or("Missing experimental graph")?;
            let (mut cursor, mut number) = (offset, 0);
            let selected = graph
                .predict_with_input(identity,
                    || self.decision_token(decision, &mut cursor, &mut number),
                    100_000, |condition| {
                    let identity = condition["source_id"]
                        .as_str()
                        .ok_or("Unbound experimental source guard")?;
                    let guard = self
                        .programs
                        .experimental_guards
                        .get(identity)
                        .ok_or("Missing experimental native guard")?;
                    if let DecisionCondition::Syntax { first_set, .. } = guard {
                        if condition["first_set"].as_bool() != Some(*first_set) {
                            return Err("Changed experimental predicate kind".into());
                        }
                    }
                    self.decision_condition(guard, offset)
                })
                .map_err(|error| format!("{error} at {identity} token {offset}"))?;
            return Ok(
                selected.filter(|alternative| Some(*alternative) != decision.exit_alternative)
            );
        }
        let mut state = decision.start;
        let mut cursor = offset;
        let mut inside_number = 0;
        let mut predicate_steps = 0;
        loop {
            let row = decision
                .states
                .get(state)
                .ok_or("Invalid prediction state")?;
            if let Some(gate) = &row.gate {
                if !self.decision_condition(gate, offset)? {
                    return Ok(None);
                }
            }
            if !row.predicate_edges.is_empty() {
                predicate_steps += 1;
                if predicate_steps > decision.states.len() {
                    return Err("Non-progressing prediction predicate cycle".into());
                }
                if !row.edges.is_empty() || row.accept.is_some() {
                    return Err("Mixed prediction transition kinds".into());
                }
                let mut target = None;
                for edge in &row.predicate_edges {
                    if self.decision_condition(&edge.condition, offset)? {
                        target = Some(edge.target);
                        break;
                    }
                }
                let Some(next) = target else {
                    return Ok(None);
                };
                state = next;
                continue;
            }
            if let Some(alternative) = row.accept {
                return Ok((Some(alternative) != decision.exit_alternative).then_some(alternative));
            }
            predicate_steps = 0;
            let before = (cursor, inside_number);
            let Some(symbol) = self.decision_token(decision, &mut cursor, &mut inside_number)?
            else {
                return Ok(None);
            };
            if (cursor, inside_number) <= before {
                return Err("Non-progressing prediction automaton".into());
            }
            let Some(target) = row.edges.get(&symbol) else {
                return Ok(None);
            };
            state = *target;
        }
    }

    // Legacy Number carriers group decimal, dot and exponent components. Split
    // them for upstream lookahead using the imported terminal recognizers.
    // This is a carrier adapter, not a replacement lexer or recovery policy.
    fn decision_token(
        &self,
        decision: &Decision,
        cursor: &mut usize,
        inside_number: &mut usize,
    ) -> Result<Option<String>, String> {
        let Some(token) = self.tokens.get(*cursor) else {
            return Ok(None);
        };
        let (symbol, end) = match &token.kind {
            TokenKind::Number(value) => {
                let remaining = value
                    .get(*inside_number..)
                    .ok_or("Invalid numeric prediction cursor")?;
                let (symbol, length) = crate::xtext_terminal::numeric_component(remaining)?;
                *inside_number += length;
                if *inside_number == value.len() {
                    *cursor += 1;
                    *inside_number = 0;
                }
                return Ok(Some(symbol));
            }
            TokenKind::String(value) => {
                let raw = format!(r#""{value}""#);
                if crate::xtext_terminal::terminal_prefix("STRING_VALUE", &raw) != Some(raw.len()) {
                    return Err("Invalid string prediction token".into());
                }
                (String::from("terminal:STRING_VALUE"), *cursor + 1)
            }
            TokenKind::BlockDoc(value) => {
                let raw=format!("/*{value}*/");
                if crate::xtext_terminal::terminal_prefix("REGULAR_COMMENT",&raw)!=Some(raw.len()) {
                    return Err("Invalid documentation prediction token".into());
                }
                (String::from("terminal:REGULAR_COMMENT"),*cursor+1)
            }
            TokenKind::Eof => (String::from("eof"), *cursor + 1),
            TokenKind::Identifier(value) if quoted_identifier(token, value) => {
                let raw = format!("'{value}'");
                if crate::xtext_terminal::terminal_prefix("UNRESTRICTED_NAME", &raw)
                    != Some(raw.len())
                {
                    return Err("Invalid quoted-name prediction token".into());
                }
                (String::from("terminal:UNRESTRICTED_NAME"), *cursor + 1)
            }
            _ if self
                .name(&Match {
                    consumed: *cursor,
                    ..Match::default()
                })
                .is_some() =>
            {
                (String::from("terminal:ID"), *cursor + 1)
            }
            _ => self
                .decision_keyword(decision, *cursor)
                .ok_or_else(||format!("Unqualified prediction token at {}:{}",token.span.start_line,token.span.start_col))?,
        };
        *cursor = end;
        Ok(Some(symbol))
    }

    fn decision_keyword(&self, decision: &Decision, offset: usize) -> Option<(String, usize)> {
        let input = Match {
            consumed: offset,
            ..Match::default()
        };
        let keyword = self.lexer_keyword(offset)?;
        if !decision
            .keywords
            .iter()
            .any(|candidate| candidate == keyword)
        {
            return None;
        }
        self.literal(keyword, &input)
            .map(|next| (format!("keyword:{keyword}"), next.consumed))
    }

    // Generated decisions are proven disjoint before emission. Match only the
    // bounded prefix, so a selected but malformed branch remains an error.
    fn predicts(&self, paths: &[Vec<String>], offset: usize) -> bool {
        paths.iter().any(|path| {
            let mut state = Match {
                consumed: offset,
                ..Match::default()
            };
            for symbol in path {
                let next = match symbol.as_str() {
                    "<Name>" => self.name(&state),
                    "<number:1>" | "<number:2>" => {
                        self.tokens.get(state.consumed).and_then(|token| {
                            let TokenKind::Number(value) = &token.kind else {
                                return None;
                            };
                            let digits =
                                !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit());
                            if digits != (symbol == "<number:1>") {
                                return None;
                            }
                            let mut next = state.clone();
                            next.consumed += 1;
                            Some(next)
                        })
                    }
                    "<string>" | "<comment>" => self.tokens.get(state.consumed).and_then(|token| {
                        if !matches!(
                            (symbol.as_str(), &token.kind),
                            ("<string>", TokenKind::String(_))
                                | ("<comment>", TokenKind::BlockDoc(_))
                        ) {
                            return None;
                        }
                        let mut next = state.clone();
                        next.consumed += 1;
                        Some(next)
                    }),
                    _ => self.literal(symbol, &state),
                };
                let Some(next) = next else {
                    return false;
                };
                state = next;
            }
            true
        })
    }

    fn body(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        ensure_prediction_supported(node)?;
        if !matches!(node.operation, Operation::Choice { .. }) {
            if let Some(decision) = &node.decision {
                return self.body_entry(node, state, decision);
            }
        }
        self.body_without_entry(node, state)
    }

    #[inline(never)]
    fn body_entry(
        &self,
        node: &Node,
        state: &Match,
        decision: &Decision,
    ) -> Result<Option<Match>, String> {
        match self.decide(decision, state.consumed)? {
            None => Ok(None),
            Some(0) => self
                .body_without_entry(node, state)?
                .map(Some)
                .ok_or_else(|| "Committed entry prediction failed".into()),
            Some(_) => Err("Invalid entry prediction alternative".into()),
        }
    }

    fn body_without_entry(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        if !node.prediction.is_empty() && !self.predicts(&node.prediction, state.consumed) {
            return Ok(None);
        }
        if node.predicated || node.first_set_predicated {
            let mut probe = SyntaxProbe {
                executor: self,
                active: Vec::new(),
                memo: BTreeMap::new(),
            };
            let accepts = if node.first_set_predicated {
                // Repetition cardinality does not make its per-occurrence guard
                // succeed: only a matching first token enters the occurrence.
                probe.first(node, state.consumed)?.0
            } else {
                !probe.body(node, state.consumed)?.is_empty()
            };
            if !accepts {
                return Ok(None);
            }
        }
        match &node.operation {
            Operation::TokenDatatype { .. } => self.body_token_datatype(node, state),
            Operation::CrossReference { .. } => self.body_cross_reference(node, state),
            Operation::Contain { .. } => self.body_contain(node, state),
            Operation::AppendOperand { .. } => self.body_append_operand(node, state),
            Operation::Link { .. } => self.body_link(node, state),
            Operation::Datatype { .. } => self.body_datatype(node, state),
            Operation::Capture { .. } => self.body_capture(node, state),
            Operation::Create { .. } => self.body_create(node, state),
            Operation::Keyword { .. } => self.body_keyword(node, state),
            Operation::Enum { .. } => self.body_enum(node, state),
            Operation::Name => self.body_name(node, state),
            Operation::Terminal { .. } => self.body_terminal(node, state),
            Operation::Call { .. } => self.body_call(node, state),
            Operation::Sequence { .. } => self.body_sequence(node, state),
            Operation::Choice { .. } => self.body_choice(node, state),
            Operation::Assign { .. } => self.body_assign(node, state),
        }
    }

    #[inline(never)]
    fn body_token_datatype(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::TokenDatatype { syntax } = &node.operation else {
            unreachable!()
        };
        {
            let Some(end) = self.token_ends(syntax, state.consumed)?.into_iter().max() else {
                return Ok(None);
            };
            let spelling = self.tokens[state.consumed..end]
                .iter()
                .map(|token| {
                    if let TokenKind::Identifier(value) = &token.kind {
                        if quoted_identifier(token, value) {
                            return format!("'{value}'");
                        }
                    }
                    Self::token_spelling(&token.kind)
                })
                .collect::<String>();
            let mut next = state.clone();
            next.consumed = end;
            next.value = Some(Value::String(spelling));
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_cross_reference(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        let Operation::CrossReference { terminal, .. } = &node.operation else {
            unreachable!()
        };

        let completed = self.node(terminal, state)?;
        self.finish_cross_reference(node, state, completed)
    }

    #[inline(never)]
    fn finish_cross_reference(
        &self,
        node: &Node,
        state: &Match,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::CrossReference { target, terminal } = &node.operation else {
            unreachable!()
        };
        {
            let Some(mut next) = completed else {
                return Ok(None);
            };
            let spelling = next
                .value
                .take()
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or("Cross-reference has no EString spelling")?;
            next.reference = Some(PendingReference {
                target_type: target.clone(),
                spelling,
                span: Some(Box::new({
                    let first = self.tokens.get(state.consumed).ok_or("Missing reference start")?;
                    let last = next.consumed.checked_sub(1).and_then(|i| self.tokens.get(i))
                        .ok_or("Missing reference end")?;
                    mercurio_foundation::language_contracts::ast::SourceSpan {
                        end_line: last.span.end_line, end_col: last.span.end_col,
                        ..first.span.clone()
                    }
                })),
            });
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_contain(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        let Operation::Contain { terminal, .. } = &node.operation else {
            unreachable!()
        };

        let completed = self.node(
            terminal,
            &Match {
                consumed: state.consumed,
                ..Match::default()
            },
        )?;
        self.finish_contain(node, state, completed)
    }

    #[inline(never)]
    fn finish_contain(
        &self,
        node: &Node,
        state: &Match,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Contain {
            feature,
            feature_id,
            operator,
            terminal,
        } = &node.operation
        else {
            unreachable!()
        };
        {
            let Some(mut child) = completed else {
                return Ok(None);
            };
            let kind = state.object_kind.as_deref().unwrap_or(self.kind);
            let contract =
                ecore_model::feature(kind, feature).ok_or("Unknown containment assignment")?;
            if contract.id != feature_id {
                return Err(format!("Changed containment identity {}", node.id));
            }
            let child_kind = child
                .object_kind
                .as_deref()
                .ok_or("Containment did not construct an object")?;
            let inverse = ecore_model::containment_inverse(kind, feature, child_kind)?;
            if child.container_feature.is_some() {
                return Err("Child already has a container".into());
            }
            child.container_feature = Some(inverse.field.into());
            let mut next = state.clone();
            next.consumed = child.consumed;
            next.object_kind.get_or_insert_with(|| self.kind.into());
            let children = next.children.entry(feature.clone()).or_default();
            match operator.as_str() {
                "=" => {
                    children.clear();
                    next.construction_order.retain(|step| {
                        !matches!(step,
                        ConstructionStep::Child { field, .. } if field == feature)
                    });
                }
                "+=" => {}
                _ => return Err("Unsupported containment operator".into()),
            }
            next.construction_order.push(ConstructionStep::Child {
                field: feature.clone(),
                index: children.len(),
            });
            children.push(Arc::new(child));
            Self::validate_reference_count(kind, feature, children.len())?;
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_append_operand(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        let Operation::AppendOperand { terminal, .. } = &node.operation else {
            unreachable!()
        };

        let completed = self.node(
            terminal,
            &Match {
                consumed: state.consumed,
                ..Match::default()
            },
        )?;
        self.finish_append_operand(node, state, completed)
    }

    #[inline(never)]
    fn finish_append_operand(
        &self,
        node: &Node,
        state: &Match,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::AppendOperand {
            feature_id,
            terminal,
        } = &node.operation
        else {
            unreachable!()
        };
        {
            let Some(child) = completed else {
                return Ok(None);
            };
            let kind = state.object_kind.as_deref().unwrap_or(self.kind);
            let feature =
                ecore_model::feature(kind, "operand").ok_or("Missing operand assignment")?;
            if feature.id != feature_id || child.container_feature.is_some() {
                return Err("Operand assignment contract mismatch".into());
            }
            ecore_model::validate_reference_endpoint(
                kind,
                "operand",
                child
                    .object_kind
                    .as_deref()
                    .ok_or("Operand call constructed no object")?,
            )?;
            let mut next = state.clone();
            next.object_kind.get_or_insert_with(|| self.kind.into());
            next.consumed = child.consumed;
            next.construction_order
                .push(ConstructionStep::Operand(next.captured_operands.len()));
            next.captured_operands.push(Arc::new(child));
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_link(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        let Operation::Link { terminal, .. } = &node.operation else {
            unreachable!()
        };
        let mut input = state.clone();
        input.reference = None;
        let completed = self.node(terminal, &input)?;
        self.finish_link(node, state, completed)
    }

    #[inline(never)]
    fn finish_link(
        &self,
        node: &Node,
        state: &Match,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Link {
            feature,
            feature_id,
            operator,
            terminal,
        } = &node.operation
        else {
            unreachable!()
        };
        {
            let Some(mut next) = completed else {
                return Ok(None);
            };
            let reference = next.reference.take().ok_or("Missing deferred reference")?;
            let kind = state.object_kind.as_deref().unwrap_or(self.kind);
            let contract =
                ecore_model::feature(kind, feature).ok_or("Unknown reference assignment")?;
            if contract.id != feature_id
                || contract.containment
                || contract.container
                || contract.derived
            {
                return Err(format!("Unsupported reference contract {}", node.id));
            }
            ecore_model::validate_reference_endpoint(
                kind,
                feature,
                reference.target_type.rsplit('/').next().unwrap(),
            )?;
            next.object_kind.get_or_insert_with(|| self.kind.into());
            let references = next.links.entry(feature.clone()).or_default();
            match operator.as_str() {
                "=" => references.clear(),
                "+=" => {}
                _ => return Err("Unsupported reference operator".into()),
            }
            references.push(reference);
            Self::validate_reference_count(kind, feature, references.len())?;
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_datatype(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Datatype { syntax } = &node.operation else {
            unreachable!()
        };
        {
            let Some(Token {
                kind: TokenKind::Number(spelling),
                ..
            }) = self.tokens.get(state.consumed)
            else {
                return Ok(None);
            };
            if !Self::lexical(syntax, spelling, 0)?.contains(&spelling.len()) {
                return Ok(None);
            }
            let mut next = state.clone();
            next.consumed += 1;
            next.value = Some(Value::String(spelling.clone()));
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_capture(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Capture {
            classifier,
            feature_id,
            captured_types,
        } = &node.operation
        else {
            unreachable!()
        };
        {
            let current = state
                .object_kind
                .as_deref()
                .ok_or("Capture requires a current object")?;
            check_capture(classifier, feature_id, captured_types, current)?;
            if state.container_feature.is_some() {
                return Err("Cannot capture an already-contained object".into());
            }
            Ok(Some(Match {
                consumed: state.consumed,
                object_kind: Some(classifier.rsplit('/').next().unwrap().into()),
                captured_operands: vec![Arc::new(state.clone())],
                construction_order: Box::new(vec![ConstructionStep::Operand(0)]),
                ..Match::default()
            }))
        }
    }

    #[inline(never)]
    fn body_create(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Create { classifier } = &node.operation else {
            unreachable!()
        };
        {
            let kind = classifier
                .rsplit('/')
                .next()
                .ok_or("Invalid action classifier")?;
            if !metaclass_conforms(kind, self.kind) {
                return Err(format!(
                    "Incompatible action class {classifier} for {}",
                    self.kind
                ));
            }
            Ok(Some(Match {
                consumed: state.consumed,
                object_kind: Some(kind.into()),
                ..Match::default()
            }))
        }
    }

    #[inline(never)]
    fn body_keyword(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Keyword { value } = &node.operation else {
            unreachable!()
        };
        Ok(self.literal(value, state))
    }

    #[inline(never)]
    fn body_enum(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Enum { token, literal } = &node.operation else {
            unreachable!()
        };
        Ok(self.literal(token, state).map(|mut next| {
            next.value = Some(Value::String(literal.clone()));
            next
        }))
    }

    #[inline(never)]
    fn body_name(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Name = &node.operation else {
            unreachable!()
        };
        Ok(self.name(state))
    }

    fn name(&self, state: &Match) -> Option<Match> {
        let token = self.tokens.get(state.consumed)?;
        let value = match &token.kind {
            TokenKind::Identifier(value) => value.clone(),
            TokenKind::Package
            | TokenKind::Import
            | TokenKind::Part
            | TokenKind::Def
            | TokenKind::Specializes => Self::lexical_spelling(token),
            _ => return None,
        };
        let quoted =
            matches!(token.kind, TokenKind::Identifier(_)) && quoted_identifier(token, &value);
        if !quoted
            && (crate::xtext_terminal::terminal_prefix("ID", &value) != Some(value.len())
                || crate::xtext_terminal::is_keyword(self.kerml, &value))
        {
            return None;
        }
        let mut next = state.clone();
        next.consumed += 1;
        // Represented names exclude delimiters and resolve escapes; carriers
        // retain raw spelling independently of language keyword classification.
        let represented = if quoted {
            crate::parser::textual_representation::language_value(&value).ok()?
        } else {
            value
        };
        next.value = Some(Value::String(represented));
        Some(next)
    }

    #[inline(never)]
    fn body_terminal(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Terminal { rule, carrier } = &node.operation else {
            unreachable!()
        };
        {
            let Some(token) = self.tokens.get(state.consumed) else {
                return Ok(None);
            };
            let (spelling, value) = match (carrier.as_str(), &token.kind) {
                ("number", TokenKind::Number(value)) => (value.clone(), value.clone()),
                // The resolved DefaultTerminalConverter preserves raw EString
                // spelling. Semantic unescaping is a separate model operation.
                ("string", TokenKind::String(value)) => {
                    let raw = format!(r#""{value}""#);
                    (raw.clone(), raw)
                }
                ("comment", TokenKind::BlockDoc(value)) => {
                    let raw = format!("/*{value}*/");
                    (raw.clone(), raw)
                }
                _ => return Ok(None),
            };
            if crate::xtext_terminal::terminal_prefix(rule, &spelling) != Some(spelling.len()) {
                return Ok(None);
            }
            let mut next = state.clone();
            next.consumed += 1;
            next.value = Some(Value::String(value));
            Ok(Some(next))
        }
    }

    #[inline(never)]
    fn body_call(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Call { rule } = &node.operation else {
            unreachable!()
        };
        let identity = self.programs.entry_rules.get(rule).unwrap_or(rule);
        let callee = self
            .programs
            .rules
            .get(identity)
            .ok_or("Unknown called rule")?;
        if !callee.construct && !callee.scalar && state.object_kind.is_none() {
            // Xtext creates the caller's current object before an unassigned
            // fragment call, even when the fragment matches no assignments.
            // Stage it locally: failed optional calls cannot leak creation.
            let mut current = state.clone();
            current.object_kind = Some(self.kind.into());
            return self.rule(rule, &current);
        }
        self.rule(rule, state)
    }

    #[inline(never)]
    fn body_sequence(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Sequence { elements } = &node.operation else {
            unreachable!()
        };
        {
            let mut current = state.clone();
            for child in elements {
                let Some(next) = self.node(child, &current)? else {
                    if !self.speculative && current.consumed > state.consumed {
                        return Err(format!("Incomplete Xtext group at {}", child.id));
                    }
                    return Ok(None);
                };
                current = next;
            }
            Ok(Some(current))
        }
    }

    #[inline(never)]
    fn body_choice(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Choice { elements } = &node.operation else {
            unreachable!()
        };
        if let Some(decision) = &node.decision {
            let Some(alternative) = self.decide(decision, state.consumed)? else {
                return Ok(None);
            };
            let selected = elements
                .get(alternative)
                .ok_or("Invalid prediction alternative")?;
            return self
                .node(selected, state)?
                .map(Some)
                .ok_or_else(|| "Committed prediction branch failed".into());
        }
        {
            for child in elements {
                if let Some(next) = self.node(child, state)? {
                    return Ok(Some(next));
                }
            }
            Ok(None)
        }
    }

    #[inline(never)]
    fn body_assign(&self, node: &Node, state: &Match) -> Result<Option<Match>, String> {
        let Operation::Assign { terminal, .. } = &node.operation else {
            unreachable!()
        };
        let mut input = state.clone();
        input.value = None;
        let completed = self.node(terminal, &input)?;
        self.finish_assign(node, state, completed)
    }

    #[inline(never)]
    fn finish_assign(
        &self,
        node: &Node,
        _state: &Match,
        completed: Option<Match>,
    ) -> Result<Option<Match>, String> {
        #[allow(unused_variables)]
        let Operation::Assign {
            feature,
            feature_id,
            operator,
            terminal,
        } = &node.operation
        else {
            unreachable!()
        };
        {
            let Some(mut next) = completed else {
                return Ok(None);
            };
            let current_kind = next.object_kind.as_deref().unwrap_or(self.kind);
            let contract = ecore_model::feature(current_kind, feature)
                .ok_or_else(|| format!("Unknown assignment {}.{feature}", self.kind))?;
            if contract.id != feature_id {
                return Err(format!("Changed Ecore identity for {}", node.id));
            }
            let value = match operator.as_str() {
                "?=" => Value::Bool(true),
                "=" => next
                    .value
                    .take()
                    .ok_or_else(|| format!("Missing assignment value {}", node.id))?,
                "+=" => {
                    let mut values = match next.fields.get(feature) {
                        None => Vec::new(),
                        Some(Value::Array(values)) => values.clone(),
                        _ => return Err(format!("Non-list assignment {}", node.id)),
                    };
                    values.push(
                        next.value
                            .take()
                            .ok_or_else(|| format!("Missing assignment value {}", node.id))?,
                    );
                    Value::Array(values)
                }
                _ => return Err(format!("Unsupported assignment operator {operator}")),
            };
            ecore_model::validate_value(current_kind, feature, &value)?;
            next.object_kind.get_or_insert_with(|| self.kind.into());
            next.fields.insert(feature.clone(), value);
            Ok(Some(next))
        }
    }
}

/// Experimental graph is embedded only by tests until complete-rule qualification.
#[cfg(test)]
pub(crate) fn experimental_expression(source: &str, kerml: bool) -> Result<Option<Match>, String> {
    static EXPERIMENTAL: OnceLock<Result<Programs, String>> = OnceLock::new();
    let programs = EXPERIMENTAL
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-partial-programs.json"
            ))
            .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let input = crate::xtext_terminal::lex(source).map_err(|e| format!("{e:?}"))?;
    let stack = RefCell::new(Vec::new());
    let executor = Executor {
        programs: programs.language(kerml)?,
        tokens: &input,
        kind: "Expression",
        kerml,
        speculative: false,
        call_stack: &stack,
        prediction_cache: &RefCell::new(BTreeMap::new()),
    };
    let result = executor.rule(
        "org.omg.kerml.expressions.xtext.KerMLExpressions::OwnedExpression",
        &Match::default(),
    )?;
    if result
        .as_ref()
        .is_some_and(|m| m.consumed + 1 != input.len())
    {
        return Err("Trailing expression tokens".into());
    }
    Ok(result)
}

/// Candidate execution preserves syntax failures separately from unsupported
/// prediction/semantic paths and resource limits. It never invokes the stable
/// authoring parser as a fallback.
#[derive(Debug)]
pub(crate) enum CandidateDocumentError {
    Syntax(String),
    Lexical(Diagnostic),
    Unsupported(String),
    ResourceLimit(String),
    Artifact(String),
}

impl CandidateDocumentError {
    fn execution(message: String) -> Self {
        if message == "Committed entry prediction failed"
            || message == "Committed prediction branch failed"
            || message == "Incomplete Xtext group"
            || message.starts_with("Incomplete Xtext group at ")
        {
            Self::Syntax(message)
        } else if message.contains("budget exceeded")
            || message.starts_with("Graph call depth exceeded")
        {
            Self::ResourceLimit(message)
        } else {
            // Unknown runtime failures must never count as rejected source.
            Self::Unsupported(message)
        }
    }
}

fn load_candidate_programs() -> Result<Programs, String> {
    let mut partial: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-finite-programs.experimental.json"
    ))
    .map_err(|error| format!("Invalid candidate Xtext programs: {error}"))?;
    let graphs: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction-nfa.experimental.json"
    ))
    .map_err(|error| format!("Invalid candidate prediction graph: {error}"))?;
    if graphs["provenance"]["grammar_sha256"].as_str().is_none()
        || graphs["provenance"]["grammar_sha256"]
            != partial["source_sha256"]["grammar.structure.extract.json"]
    {
        return Err("Candidate grammar and prediction provenance disagree".into());
    }
    for (language, context) in [
        ("kerml", "org.omg.kerml.xtext.KerML"),
        ("sysml", "org.omg.sysml.xtext.SysML"),
    ] {
        let program = partial["language_programs"][language]
            .as_object_mut()
            .ok_or("Missing candidate language program")?;
        if program.get("context").and_then(Value::as_str) != Some(context) {
            return Err("Candidate program has the wrong grammar context".into());
        }
        let graph = graphs["contexts"].get(context)
            .ok_or("Missing candidate prediction context")?;
        program.insert("experimental_graph".into(), graph.clone());
    }
    serde_json::from_value(partial).map_err(|error| format!("Invalid candidate bindings: {error}"))
}

fn candidate_programs() -> Result<&'static Programs, CandidateDocumentError> {
    static CANDIDATE: OnceLock<Result<Programs, String>> = OnceLock::new();
    CANDIDATE.get_or_init(load_candidate_programs).as_ref()
        .map_err(|error| CandidateDocumentError::Artifact(error.clone()))
}

fn candidate_model(source: &str, entry: &str, kerml: bool) -> Result<Match, CandidateDocumentError> {
    // Explicit candidate bounds, in addition to independent machine/prediction
    // budgets. An oversized source is unsupported execution, never bad syntax.
    if source.len() > 1_048_576 {
        return Err(CandidateDocumentError::ResourceLimit("Candidate source exceeds 1 MiB".into()));
    }
    let programs = candidate_programs()?.language(kerml)
        .map_err(CandidateDocumentError::Artifact)?;
    let rule = programs.rules.get(entry)
        .ok_or_else(|| CandidateDocumentError::Artifact(format!("Missing candidate rule {entry}")))?;
    if !rule.construct || rule.scalar {
        return Err(CandidateDocumentError::Artifact("Document entry requires a constructing object rule".into()));
    }
    let input = crate::xtext_terminal::lex(source).map_err(CandidateDocumentError::Lexical)?;
    let stack = RefCell::new(Vec::new());
    let kind = rule.owner.rsplit("#//").next()
        .ok_or_else(|| CandidateDocumentError::Artifact("Missing entry metaclass".into()))?;
    let executor = Executor {
        programs,
        tokens: &input,
        kind,
        kerml,
        speculative: false,
        call_stack: &stack,
        prediction_cache: &RefCell::new(BTreeMap::new()),
    };
    let result = executor.rule(entry, &Match::default())
        .map_err(CandidateDocumentError::execution)?
        .ok_or_else(|| CandidateDocumentError::Syntax("Candidate syntax did not match".into()))?;
    if result.consumed + 1 != input.len() {
        return Err(CandidateDocumentError::Syntax("Trailing candidate document tokens".into()));
    }
    Ok(result)
}

/// Parse a complete document through the pinned RootNamespace rule and imported
/// finite prediction graph. The result is a staged Ecore-shaped construction
/// tree with unresolved typed references; graph publication/linking belongs to
/// the canonical model service. Availability is not complete-rule qualification.
pub(crate) fn candidate_document(source: &str, kerml: bool) -> Result<Match, CandidateDocumentError> {
    candidate_model(source, if kerml {
        "org.omg.kerml.xtext.KerML::RootNamespace"
    } else {
        "org.omg.sysml.xtext.SysML::RootNamespace"
    }, kerml)
}

/// Tests for smaller bounded entries share the exact candidate runtime.
#[cfg(test)]
pub(crate) fn experimental_model(source: &str, entry: &str, kerml: bool) -> Result<Match, String> {
    candidate_model(source, entry, kerml).map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_prediction_preserves_documentation_terminals() {
        let artifact:Value=serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/comment-values-pilot-controls.json")).unwrap();
        let cases=artifact["cases"].as_array().unwrap().iter().filter(|c|c["kind"]=="RootNamespace").collect::<Vec<_>>();
        assert_eq!(cases.len(),8);
        for case in cases {
            let kerml=case["language"]=="kerml";let source=case["source"].as_str().unwrap();
            let tree=candidate_document(source,kerml);assert!(tree.is_ok(),"{source}: {tree:?}");
            let doc=crate::definition_document::parse_and_link(source,if kerml {crate::SourceLanguage::Kerml}else{crate::SourceLanguage::Sysml}).unwrap();
            let bodies=doc.elements.iter().filter(|e|e.kind.ends_with("::Documentation")).map(|e|e.properties["body"].clone()).collect::<Vec<_>>();
            assert_eq!(json!(bodies),case["bodies"]);
        }
        for kerml in [true,false] {
            assert!(matches!(candidate_document("package P { doc /*unterminated",kerml),Err(CandidateDocumentError::Lexical(_))));
        }
    }

    #[test]
    fn real_datatype_model_boundaries_match_independent_pilot_parsing() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/value-converter-dispatch.extract.json"
        ))
        .unwrap();
        let controls = evidence["numeric_models"].as_array().unwrap();
        assert_eq!(controls.len(), 36);
        for control in controls {
            let source = control["source"].as_str().unwrap();
            let input = crate::xtext_terminal::lex(source).unwrap();
            let matched = execute_in_context(
                "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralReal",
                &input,
                "LiteralRational",
                control["context"] == "org.omg.kerml.xtext.KerML",
            )
            .unwrap();
            let complete = matched.as_ref().filter(|result| {
                input
                    .get(result.consumed)
                    .is_some_and(|token| matches!(token.kind, TokenKind::Eof))
            });
            assert_eq!(
                complete.is_some(),
                control["accepted"].as_bool().unwrap(),
                "{control}"
            );
            if let Some(result) = complete {
                assert_eq!(
                    result.fields.get("value"),
                    Some(&control["value"]),
                    "{control}"
                );
            }
        }
    }

    #[test]
    fn resolved_scalar_converters_match_independent_pilot_observations() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/value-converter-dispatch.extract.json"
        ))
        .unwrap();
        let bindings = evidence["bindings"].as_array().unwrap();
        let controls = evidence["controls"].as_array().unwrap();
        assert_eq!(bindings.len(), 175);
        assert_eq!(controls.len(), 667);
        let mut matched = 0;
        let mut unsupported_nonfinite = 0;
        for control in controls {
            let binding = bindings
                .iter()
                .find(|binding| {
                    binding["context"] == control["context"] && binding["rule"] == control["rule"]
                })
                .unwrap();
            let algorithm = value_dispatch::algorithm(
                control["context"] == "org.omg.kerml.xtext.KerML",
                control["rule"].as_str().unwrap(),
                binding["datatype"].as_str().unwrap(),
            )
            .unwrap();
            let actual = converted_value(algorithm, control["input"].clone());
            match control["outcome"].as_str().unwrap() {
                "value" => {
                    assert_eq!(actual.unwrap().0, control["value"], "{control}");
                    matched += 1;
                }
                "rejected" => {
                    assert!(actual.is_err(), "{control}");
                    matched += 1;
                }
                "nonfinite" => {
                    assert!(actual.unwrap_err().contains("finite EDouble"));
                    unsupported_nonfinite += 1;
                }
                other => panic!("Unknown oracle outcome {other}"),
            }
        }
        assert_eq!(matched, 665);
        assert_eq!(unsupported_nonfinite, 2);
        assert!(
            value_dispatch::algorithm(
                true,
                "unknown",
                "http://www.eclipse.org/emf/2002/Ecore#//EString"
            )
            .is_none()
        );
    }

    use serde_json::json;
    fn tokens(text: &str) -> Vec<Token> {
        crate::xtext_terminal::lex(text).unwrap()
    }

    #[test]
    fn imported_lexer_controls_keyword_boundaries_and_language_specific_names() {
        let stack = RefCell::new(Vec::new());
        for kerml in [false, true] {
            for (source, selected, shorter) in [
                ("===x", "===", "=="),
                ("!==x", "!==", "!="),
                ("::>x", "::>", "::"),
                ("..2", "..", "."),
                ("??x", "??", "?"),
            ] {
                let input = tokens(source);
                let executor = Executor {
                    programs: programs().unwrap(),
                    tokens: &input,
                    kind: "Element",
                    kerml,
                    speculative: false,
                    call_stack: &stack,
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                assert_eq!(executor.lexer_keyword(0), Some(selected), "{source}");
                assert!(
                    executor.literal(selected, &Match::default()).is_some(),
                    "{source}"
                );
                assert!(
                    executor.literal(shorter, &Match::default()).is_none(),
                    "{source}"
                );
            }
            for source in ["part", "def"] {
                let input = tokens(source);
                let executor = Executor {
                    programs: programs().unwrap(),
                    tokens: &input,
                    kind: "Element",
                    kerml,
                    speculative: false,
                    call_stack: &stack,
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                assert_eq!(
                    executor.name(&Match::default()).is_some(),
                    kerml,
                    "{source}"
                );
                if kerml {
                    let model = execute_in_context(
                        "org.omg.kerml.xtext.KerML::Identification",
                        &input,
                        "Element",
                        true,
                    )
                    .unwrap()
                    .unwrap();
                    assert_eq!(model.consumed, 1);
                    assert_eq!(model.fields["declared_name"], json!(source));
                }
            }
            for source in ["specializes", ":>"] {
                let input = tokens(source);
                let executor = Executor {
                    programs: programs().unwrap(),
                    tokens: &input,
                    kind: "Element",
                    kerml,
                    speculative: false,
                    call_stack: &stack,
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                assert_eq!(executor.lexer_keyword(0), Some(source));
                assert!(executor.literal(source, &Match::default()).is_some());
            }
            let input = tokens("= =");
            let executor = Executor {
                programs: programs().unwrap(),
                tokens: &input,
                kind: "Element",
                kerml,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert!(executor.literal("==", &Match::default()).is_none());
            assert!(executor.literal("=", &Match::default()).is_some());
        }
    }

    #[test]
    fn stack_machine_work_budget_accepts_large_progressing_input() {
        let program: Programs = serde_json::from_value(json!({"rules":{
            "many":{"owner":"https://www.omg.org/spec/SysML/20250201#//Feature", "scalar":false,"construct":true,
                "body":{"id":"many/body","kind":"keyword","value":"x","cardinality":"*"}}
        }})).unwrap();
        let input = tokens(&"x ".repeat(40_000));
        let stack = RefCell::new(Vec::new());
        let executor = Executor {
            programs: &program,
            tokens: &input,
            kind: "Feature",
            kerml: true,
            speculative: false,
            call_stack: &stack,
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let state = Match::default();
        let parsed = executor.rule("many", &state).unwrap().unwrap();
        assert_eq!(parsed.consumed, 40_000);
        assert!(stack.borrow().is_empty());
        assert_eq!(state, Match::default());
    }

    #[test]
    fn bound_prediction_cache_preserves_offsets_errors_and_recursion() {
        let program: Programs = serde_json::from_value(json!({"rules":{},
            "experimental_guards":{
                "guard":{"kind":"syntax","source_id":"guard","probe":{
                    "id":"guard","cardinality":"","kind":"keyword","value":"x"}},
                "bad":{"kind":"syntax","source_id":"bad","probe":{
                    "id":"bad","cardinality":"","kind":"call","rule":"absent"}}
            }})).unwrap();
        let input = tokens("x y");
        let stack = RefCell::new(Vec::new());
        let cache = RefCell::new(BTreeMap::new());
        let executor = Executor { programs: &program, tokens: &input, kind: "Element",
            kerml: true, speculative: false, call_stack: &stack, prediction_cache: &cache };
        let guard = &program.experimental_guards["guard"];
        for _ in 0..2 {
            assert!(executor.decision_condition(guard, 0).unwrap());
            assert!(!executor.decision_condition(guard, 1).unwrap());
        }
        assert_eq!(cache.borrow().len(), 2);
        stack.borrow_mut().push(("predicate::guard".into(), 0));
        assert!(executor.decision_condition(guard, 0).unwrap_err().contains("Recursive"));
        stack.borrow_mut().clear();
        let bad = &program.experimental_guards["bad"];
        assert!(executor.decision_condition(bad, 0).is_err());
        assert!(stack.borrow().is_empty());
        assert_eq!(cache.borrow().len(), 2, "errors are never cached");
        let changed: DecisionCondition = serde_json::from_value(json!({
            "kind":"syntax","source_id":"guard","probe":{
                "id":"guard","cardinality":"","kind":"keyword","value":"y"}})).unwrap();
        assert!(!executor.decision_condition(&changed, 0).unwrap());
        assert_eq!(cache.borrow().len(), 2, "ad-hoc identities cannot read/write bound cache");
        let other_input = tokens("y x");
        let other_cache = RefCell::new(BTreeMap::new());
        let other = Executor { programs: &program, tokens: &other_input, kind: "Element",
            kerml: true, speculative: false, call_stack: &stack, prediction_cache: &other_cache };
        assert!(!other.decision_condition(guard, 0).unwrap());
        assert!(other.decision_condition(guard, 1).unwrap());
    }

    #[test]
    fn unassigned_fragment_call_creates_caller_type_without_leaking_failed_attempts() {
        for kind in ["Feature", "PartUsage"] {
            let program: Programs = serde_json::from_value(json!({"rules":{
                "fragment":{"owner":"https://www.omg.org/spec/SysML/20250201#//Feature", "scalar":false,
                    "body":{"id":"fragment/body","kind":"keyword","value":"z","cardinality":""}},
                "caller":{"owner":format!("https://www.omg.org/spec/SysML/20250201#//{kind}"),"scalar":false,"construct":true,
                    "body":{"id":"caller/body","kind":"sequence","cardinality":"","elements":[
                        {"id":"fragment/call","kind":"call","rule":"fragment","cardinality":"?"},
                        {"id":"end","kind":"keyword","value":"end","cardinality":""}]}}
            }})).unwrap();
            for source in ["z end", "end"] {
                let input = tokens(source);
                let stack = RefCell::new(Vec::new());
                let executor = Executor {
                    programs: &program,
                    tokens: &input,
                    kind: "Element",
                    kerml: true,
                    speculative: false,
                    call_stack: &stack,
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                let state = Match::default();
                let matched = executor.rule("caller", &state).unwrap().unwrap();
                assert_eq!(
                    matched.object_kind.as_deref(),
                    if source == "end" { None } else { Some(kind) }
                );
                assert_eq!(matched.consumed, input.len() - 1);
                assert_eq!(state, Match::default());
                assert!(stack.borrow().is_empty());
            }
        }
    }

    #[test]
    fn every_experimental_failure_site_raises_before_speculative_execution() {
        let raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-partial-programs.json"
        ))
        .unwrap();
        fn visit(value: &Value, nodes: &mut BTreeMap<String, Node>) {
            if value.get("unsupported").is_some() {
                nodes.insert(
                    value["id"].as_str().unwrap().into(),
                    serde_json::from_value(value.clone()).unwrap(),
                );
            }
            match value {
                Value::Object(fields) => {
                    for child in fields.values() {
                        visit(child, nodes);
                    }
                }
                Value::Array(children) => {
                    for child in children {
                        visit(child, nodes);
                    }
                }
                _ => (),
            }
        }
        let input = tokens("");
        for (language, expected) in [("kerml", 4), ("sysml", 15)] {
            let mut nodes = BTreeMap::new();
            visit(&raw["language_programs"][language]["rules"], &mut nodes);
            assert_eq!(nodes.len(), expected);
            let program: Programs =
                serde_json::from_value(raw["language_programs"][language].clone()).unwrap();
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Element",
                kerml: language == "kerml",
                speculative: true,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            for (id, node) in nodes {
                assert!(
                    executor
                        .node(&node, &Match::default())
                        .unwrap_err()
                        .contains(&id)
                );
                let mut probe = SyntaxProbe {
                    executor: &executor,
                    active: Vec::new(),
                    memo: BTreeMap::new(),
                };
                assert!(probe.node(&node, 0).unwrap_err().contains(&id));
                assert!(stack.borrow().is_empty());
            }
        }
    }

    pub(super) fn finite_graph_programs() -> Programs {
        load_candidate_programs().unwrap()
    }

    #[test]
    fn candidate_documents_construct_package_type_and_feature_models() {
        fn collect<'a>(model: &'a Match, rows: &mut Vec<&'a Match>) {
            rows.push(model);
            for children in model.children.values() {
                for child in children { collect(child, rows); }
            }
        }
        for (kerml, source, expected) in [
            (true, "package Outer { class Thing; feature elementValue : Thing; }", vec![
                ("Package", "Outer"), ("Class", "Thing"), ("Feature", "elementValue")]),
            (false, "package Outer { part def Thing; part elementValue : Thing; }", vec![
                ("Package", "Outer"), ("PartDefinition", "Thing"), ("PartUsage", "elementValue")]),
        ] {
            let model = candidate_document(source, kerml).unwrap();
            assert_eq!(model.object_kind.as_deref(), Some("Namespace"));
            let mut rows = Vec::new();
            collect(&model, &mut rows);
            for (kind, name) in expected {
                assert!(rows.iter().any(|row| row.object_kind.as_deref() == Some(kind)
                    && row.fields.get("declared_name").and_then(Value::as_str) == Some(name)),
                    "{source}: missing {kind} {name}: {model:?}");
            }
            assert!(rows.iter().flat_map(|row| row.links.values().flatten())
                .any(|link| link.spelling == "Thing" && link.target_type.ends_with("#//Type")),
                "The construction tree must retain the imported typed reference: {model:?}");
        }
        for kerml in [true, false] {
            let empty = candidate_document("// empty document\n", kerml).unwrap();
            assert_eq!(empty.object_kind.as_deref(), Some("Namespace"));
        }
    }

    #[test]
    fn candidate_documents_reject_incomplete_and_trailing_source() {
        for kerml in [true, false] {
            for source in ["package P {", "package P; }", "package P { package Q;", "package P { package Q } "] {
                let result = candidate_document(source, kerml);
                assert!(matches!(result, Err(CandidateDocumentError::Syntax(_))), "{source}: {result:?}");
            }
        }
        // Confirmed against the pinned Pilot frontend: a plain KerML Type
        // requires specialization/conjugation; SysML reserves bare `item`.
        for (kerml, source) in [
            (true, "package Outer { type Thing; feature item : Thing; }"),
            (false, "package Outer { part def Thing; part item : Thing; }"),
        ] {
            let result = candidate_document(source, kerml);
            assert!(matches!(result, Err(CandidateDocumentError::Syntax(_))), "{source}: {result:?}");
        }
    }

    #[test]
    fn candidate_execution_failures_do_not_count_as_invalid_syntax() {
        for failure in ["Unresolved graph ambiguity at rule token 0", "Caller FOLLOW context required",
            "Unimplemented prediction at rule: missing", "Changed Ecore identity for node"] {
            assert!(matches!(CandidateDocumentError::execution(failure.into()), CandidateDocumentError::Unsupported(_)));
        }
        for failure in ["Graph work budget exceeded at rule token 0", "Graph call depth exceeded at rule token 0",
            "Xtext execution step budget exceeded", "Xtext model rule frame budget exceeded (256)"] {
            assert!(matches!(CandidateDocumentError::execution(failure.into()), CandidateDocumentError::ResourceLimit(_)));
        }
        assert!(matches!(candidate_document(&" ".repeat(1_048_577), false), Err(CandidateDocumentError::ResourceLimit(_))));
    }

    #[test]
    fn finite_graph_executes_complete_definition_body_items() {
        let program = finite_graph_programs();
        let evidence: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction-nfa.experimental.json"
        ))
        .unwrap();
        fn snapshot(model: &Match, tree: &mut Vec<Value>) {
            let mut row = json!({"kind":model.object_kind});
            if let Some(name) = model.fields.get("declared_name") {
                row["name"] = name.clone();
            }
            tree.push(row);
            for child in model.children.values().flatten() {
                snapshot(child, tree);
            }
        }
        for case in evidence["complete_model_controls"].as_array().unwrap() {
            let source = case["source"].as_str().unwrap();
            let kerml = case["context"].as_str().unwrap().ends_with("KerML");
            let entry = format!(
                "{}::{}",
                case["context"].as_str().unwrap(),
                case["entry"].as_str().unwrap()
            );
            let input = tokens(source);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program.language(kerml).unwrap(),
                tokens: &input,
                kind: program.language(kerml).unwrap().rules[&entry]
                    .owner
                    .rsplit("//")
                    .next()
                    .unwrap(),
                kerml,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let result = executor.rule(&entry, &Match::default());
            let complete = matches!(&result, Ok(Some(model)) if model.consumed == input.len()-1);
            assert_eq!(
                complete,
                case["accepted"].as_bool().unwrap(),
                "{source}: {result:?}"
            );
            if let Err(error) = &result {
                assert!(
                    (error.starts_with("Committed ") && error.ends_with(" failed"))
                        || error == "Incomplete Xtext group",
                    "An unsupported/runtime failure is not a syntax rejection: {source}: {error}"
                );
            }
            if complete {
                let model = result.unwrap().unwrap();
                let mut tree = Vec::new();
                snapshot(&model, &mut tree);
                assert_eq!(json!(tree), case["tree"], "{source}");
            }
            assert!(stack.borrow().is_empty());
        }
    }

    #[test]
    fn experimental_expression_paths() {
        let program: Programs = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-partial-programs.json"
        ))
        .unwrap();
        for kerml in [true, false] {
            for source in ["1", "true", "x", "1 + 2", "1 + 2 * 3", "(1)"] {
                let input = tokens(source);
                let stack = RefCell::new(Vec::new());
                let executor = Executor {
                    programs: program.language(kerml).unwrap(),
                    tokens: &input,
                    kind: "Expression",
                    kerml,
                    speculative: false,
                    call_stack: &stack,
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                let result = executor.rule(
                    "org.omg.kerml.expressions.xtext.KerMLExpressions::OwnedExpression",
                    &Match::default(),
                );
                let matched = result.unwrap().unwrap();
                assert_eq!(matched.consumed, input.len() - 1, "{kerml} {source}");
                match source {
                    "1" | "(1)" => assert_eq!(matched.fields["value"], 1),
                    "true" => assert_eq!(matched.fields["value"], true),
                    "x" => {
                        assert_eq!(
                            matched.object_kind.as_deref(),
                            Some("FeatureReferenceExpression")
                        );
                        assert_eq!(
                            matched.children["owned_relationship"][0].links["member_element"][0]
                                .spelling,
                            "x"
                        );
                    }
                    _ => {
                        assert_eq!(matched.fields["operator"], "+");
                        assert_eq!(matched.captured_operands.len(), 2);
                        assert_eq!(matched.captured_operands[0].fields["value"], 1);
                        if source == "1 + 2 * 3" {
                            let right = &matched.captured_operands[1];
                            assert_eq!(right.fields["operator"], "*");
                            assert_eq!(right.captured_operands[0].fields["value"], 2);
                            assert_eq!(right.captured_operands[1].fields["value"], 3);
                        } else {
                            assert_eq!(matched.captured_operands[1].fields["value"], 2);
                        }
                    }
                }
                assert!(stack.borrow().is_empty());
            }
        }
    }

    #[test]
    fn unresolved_prediction_is_an_error_even_in_optional_and_predicate_paths() {
        let input = tokens("x");
        for speculative in [true, false] {
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: programs().unwrap(),
                tokens: &input,
                kind: "Element",
                kerml: true,
                speculative,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            for cardinality in ["", "?", "*", "+"] {
                let node: Node = serde_json::from_value(json!({"id":"unsupported/site",
                    "kind":"keyword", "value":"never", "cardinality":cardinality,
                    "unsupported":"unresolved decision", "prediction":[["keyword:never"]]}))
                .unwrap();
                let state = Match::default();
                assert!(
                    executor
                        .node(&node, &state)
                        .unwrap_err()
                        .contains("unsupported/site")
                );
                let mut probe = SyntaxProbe {
                    executor: &executor,
                    active: Vec::new(),
                    memo: BTreeMap::new(),
                };
                assert!(
                    probe
                        .node(&node, 0)
                        .unwrap_err()
                        .contains("unsupported/site")
                );
                assert!(
                    probe
                        .first(&node, 0)
                        .unwrap_err()
                        .contains("unsupported/site")
                );
                assert_eq!(state.consumed, 0);
                assert!(stack.borrow().is_empty());
            }
        }
    }

    #[test]
    fn capture_execution_preserves_left_association_and_transactional_state() {
        // Controlled acyclic context around the exact pinned additive action.
        // This tests the executor, not the still-unadmitted enclosing grammar.
        let mut raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        let action_id = raw["operator_captures"]["+"].as_str().unwrap().to_owned();
        let action = raw["capture_actions"][&action_id].clone();
        raw["rules"]["controlled"] = json!({
            "owner":"https://www.omg.org/spec/SysML/20250201#//Expression",
            "scalar":false, "construct":true,
            "body":{"id":"controlled/body","cardinality":"","kind":"sequence","elements":[
                {"id":"left","cardinality":"","kind":"call","rule":"org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralExpression"},
                {"id":"tail","cardinality":"*","kind":"sequence","elements":[
                    action,
                    {"id":"operator","cardinality":"","kind":"assign","feature":"operator",
                     "feature_id":"https://www.omg.org/spec/SysML/20250201#//OperatorExpression/operator",
                     "operator":"=","terminal":{"id":"plus","cardinality":"","kind":"keyword","value":"+"}},
                    {"id":"right","cardinality":"","kind":"append_operand",
                     "feature_id":"https://www.omg.org/spec/SysML/20250201#//InvocationExpression/operand",
                     "terminal":{"id":"literal","cardinality":"","kind":"call","rule":"org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralExpression"}}
                ]}
            ]}
        });
        let program: Programs = serde_json::from_value(raw).unwrap();
        let input = tokens("1 + 2 + 3");
        let executor = Executor {
            programs: &program,
            tokens: &input,
            kind: "Expression",
            kerml: true,
            speculative: false,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let result = executor
            .rule("controlled", &Match::default())
            .unwrap()
            .unwrap();
        assert_eq!(result.consumed, input.len() - 1);
        assert_eq!(result.object_kind.as_deref(), Some("OperatorExpression"));
        assert_eq!(result.fields["operator"], "+");
        assert_eq!(result.captured_operands.len(), 2);
        assert_eq!(result.captured_operands[1].fields["value"], 3);
        let left = &result.captured_operands[0];
        assert_eq!(left.object_kind.as_deref(), Some("OperatorExpression"));
        assert_eq!(left.captured_operands[0].fields["value"], 1);
        assert_eq!(left.captured_operands[1].fields["value"], 2);
        assert!(!result.children.contains_key("operand"));
        assert!(!result.fields.contains_key("operand"));
        let saved = result.clone();
        let incomplete = tokens("+");
        let speculative = Executor {
            programs: &program,
            tokens: &incomplete,
            kind: "Expression",
            kerml: true,
            speculative: true,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let mut prior = result;
        prior.consumed = 0;
        let before = prior.clone();
        let Operation::Sequence { elements } = &program.rules["controlled"].body.operation else {
            panic!()
        };
        let tail = &elements[1];
        // Repetition returns unchanged state when its tentative iteration fails.
        assert_eq!(speculative.node(tail, &prior).unwrap().unwrap(), before);
        assert_eq!(prior, before);
        assert_eq!(saved.fields["operator"], "+");
        assert!(
            executor
                .body(&program.capture_actions[&action_id], &Match::default())
                .is_err()
        );
        let mut contained = prior.clone();
        contained.container_feature = Some("owning_relationship".into());
        assert!(
            executor
                .body(&program.capture_actions[&action_id], &contained)
                .is_err()
        );
    }

    #[test]
    fn upstream_expression_decisions_and_logical_tokens_match_pilot() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/expression-decision-pilot-controls.json"
        ))
        .unwrap();
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        for case in evidence["cases"].as_array().unwrap() {
            let kernel = case["language"] == "kerml";
            let context = if kernel {
                "org.omg.kerml.xtext.KerML"
            } else {
                "org.omg.sysml.xtext.SysML"
            };
            let mut raw = imported["contexts"][context][case["rule"].as_str().unwrap()].clone();
            raw["keywords"] = imported["contexts"][context]["keywords"].clone();
            let decision: Decision = serde_json::from_value(raw).unwrap();
            let input = tokens(case["source"].as_str().unwrap());
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: programs().unwrap().language(kernel).unwrap(),
                tokens: &input,
                kind: "Expression",
                kerml: kernel,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let mut cursor = 0;
            let mut inner = 0;
            let mut logical = Vec::new();
            while let Some(symbol) = executor
                .decision_token(&decision, &mut cursor, &mut inner)
                .unwrap()
            {
                logical.push(symbol);
            }
            assert_eq!(json!(logical), case["tokens"], "{case}");
            assert_eq!(
                executor.decide(&decision, 0).unwrap(),
                Some(case["alternative"].as_u64().unwrap() as usize),
                "{case}"
            );
        }
    }

    #[test]
    fn prediction_predicates_are_zero_width_ordered_and_fail_closed() {
        let probe = json!({"id":"guard", "kind":"sequence", "cardinality":"", "elements":[
            {"id":"a", "kind":"keyword", "cardinality":"", "value":"a"},
            {"id":"b", "kind":"keyword", "cardinality":"", "value":"b"}]});
        let condition = json!({"kind":"syntax", "source_id":"controlled/guard", "probe":probe});
        let raw = json!({"start":0,"keywords":[],"states":[
            {"edges":{"terminal:ID":1}},
            {"edges":{},"predicate_edges":[{"condition":condition,"target":2},
                {"condition":{"kind":"always"},"target":3}]},
            {"edges":{},"gate":condition,"accept":0},
            {"edges":{},"accept":1}]});
        for (source, offset, expected) in [
            ("a b", 0, Some(0)),
            ("a c", 0, Some(1)),
            ("prefix a b", 1, Some(0)),
            ("a", 0, Some(1)),
        ] {
            let input = tokens(source);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: programs().unwrap().language(false).unwrap(),
                tokens: &input,
                kind: "Element",
                kerml: false,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let decision: Decision = serde_json::from_value(raw.clone()).unwrap();
            assert_eq!(
                executor.decide(&decision, offset).unwrap(),
                expected,
                "{source}"
            );
            assert!(stack.borrow().is_empty());
            let mut first_set = raw.clone();
            first_set["states"][1]["predicate_edges"][0]["condition"]["first_set"] = json!(true);
            first_set["states"][2]["gate"]["first_set"] = json!(true);
            let first_set: Decision = serde_json::from_value(first_set).unwrap();
            assert_eq!(
                executor.decide(&first_set, offset).unwrap(),
                Some(0),
                "first-set predicate must inspect only the first token: {source}"
            );
            assert!(stack.borrow().is_empty());
            if source == "a c" {
                let gated: Decision = serde_json::from_value(json!({"start":0,"keywords":[],
                    "states":[{"edges":{},"accept":0,"gate":condition}]}))
                .unwrap();
                assert_eq!(executor.decide(&gated, 0).unwrap(), None);
            }
            let mut unbound = raw.clone();
            unbound["states"][1]["predicate_edges"][0]["condition"]
                .as_object_mut()
                .unwrap()
                .remove("probe");
            let unbound: Decision = serde_json::from_value(unbound).unwrap();
            assert!(
                executor
                    .decide(&unbound, offset)
                    .unwrap_err()
                    .contains("Unbound")
            );
            assert!(stack.borrow().is_empty());
        }
        let input = tokens("a");
        let stack = RefCell::new(Vec::new());
        let executor = Executor {
            programs: programs().unwrap().language(false).unwrap(),
            tokens: &input,
            kind: "Element",
            kerml: false,
            speculative: false,
            call_stack: &stack,
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let cycle: Decision = serde_json::from_value(json!({"start":0,"keywords":[],
            "states":[{"edges":{},"predicate_edges":[{"condition":{"kind":"always"},"target":0}]}]})).unwrap();
        assert!(executor.decide(&cycle, 0).unwrap_err().contains("cycle"));
        let unknown: DecisionCondition = serde_json::from_value(json!({"kind":"syntax",
            "source_id":"controlled/unknown","probe":{"id":"unknown","cardinality":"","kind":"call","rule":"unknown"}})).unwrap();
        assert!(executor.decision_condition(&unknown, 0).is_err());
        assert!(
            stack.borrow().is_empty(),
            "failed probes must unwind their recursion guard"
        );
        stack
            .borrow_mut()
            .push(("predicate::controlled/unknown".into(), 0));
        assert!(
            executor
                .decision_condition(&unknown, 0)
                .unwrap_err()
                .contains("Recursive")
        );
        assert_eq!(
            stack.borrow().len(),
            1,
            "recursion failure must preserve its caller frame"
        );
    }

    #[test]
    fn pinned_succession_member_constructs_multiplicity_models_matching_pilot() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/succession-entry-pilot-controls.json"
        ))
        .unwrap();
        fn visit(model: &Match, kinds: &mut Vec<String>, integers: &mut Vec<Value>) {
            for child in model.children.values().flatten() {
                let kind = child.object_kind.clone().unwrap();
                kinds.push(kind.clone());
                if kind == "LiteralInteger" {
                    integers.push(child.fields["value"].clone());
                }
                assert!(
                    child.container_feature.is_some(),
                    "missing Ecore inverse for {kind}"
                );
                visit(child, kinds, integers);
            }
        }
        let program = programs().unwrap().language(false).unwrap();
        let root = "org.omg.sysml.xtext.SysML::EmptySuccessionMember";
        for case in evidence["cases"].as_array().unwrap() {
            let input = tokens(case["source"].as_str().unwrap());
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program,
                tokens: &input,
                kind: "FeatureMembership",
                kerml: false,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let original = Match::default();
            let result = executor.rule(root, &original);
            let complete = matches!(&result, Ok(Some(model)) if model.consumed == input.len() - 1);
            assert_eq!(
                complete,
                case["accepted"].as_bool().unwrap(),
                "{case}: {result:?}"
            );
            if complete {
                let model = result.unwrap().unwrap();
                assert_eq!(json!(model.object_kind), case["kind"]);
                let (mut kinds, mut integers) = (Vec::new(), Vec::new());
                visit(&model, &mut kinds, &mut integers);
                assert_eq!(json!(kinds), case["contained_kinds"], "{case}");
                assert_eq!(json!(integers), case["integer_values"], "{case}");
            }
            assert_eq!(original, Match::default());
            assert!(stack.borrow().is_empty());
        }
    }

    #[test]
    fn contextual_relationship_decisions_match_independent_pilot_branches() {
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/relationship-decision-pilot-controls.json"
        ))
        .unwrap();
        let context = &imported["contexts"]["org.omg.kerml.xtext.KerML"];
        assert_eq!(evidence["cases"].as_array().unwrap().len(), 72);
        for case in evidence["cases"].as_array().unwrap() {
            let mut raw = context[case["decision"].as_str().unwrap()].clone();
            assert_eq!(raw["contextual_binding"]["occurrences"], 2);
            raw["keywords"] = context["keywords"].clone();
            let decision: Decision = serde_json::from_value(raw).unwrap();
            let input = tokens(case["input"].as_str().unwrap());
            let executor = Executor {
                programs: programs().unwrap().language(true).unwrap(),
                tokens: &input,
                kind: "Relationship",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert_eq!(
                executor.decide(&decision, 0).unwrap(),
                Some(case["alternative"].as_u64().unwrap() as usize),
                "{case}"
            );
        }
    }

    #[test]
    fn expanded_decisions_match_independent_pilot_branches() {
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/expanded-decision-pilot-controls.json"
        ))
        .unwrap();
        assert_eq!(evidence["cases"].as_array().unwrap().len(), 37);
        let partial: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-partial-programs.json"
        ))
        .unwrap();
        let partial_programs: Programs = serde_json::from_value(partial.clone()).unwrap();
        fn bound_decision<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
            if value.get("id").and_then(Value::as_str) == Some(id) {
                return value.get("decision");
            }
            match value {
                Value::Object(fields) => fields.values().find_map(|v| bound_decision(v, id)),
                Value::Array(items) => items.iter().find_map(|v| bound_decision(v, id)),
                _ => None,
            }
        }
        for case in evidence["cases"].as_array().unwrap() {
            let context_name = case["context"].as_str().unwrap();
            let context = &imported["contexts"][context_name];
            let mut raw = context[case["decision"].as_str().unwrap()].clone();
            if let Some(bound) = bound_decision(
                &partial["language_programs"][if context_name == "org.omg.kerml.xtext.KerML" {
                    "kerml"
                } else {
                    "sysml"
                }]["rules"],
                raw["source_id"].as_str().unwrap(),
            ) {
                raw = bound.clone();
            }
            raw["keywords"] = context["keywords"].clone();
            let decision: Decision = serde_json::from_value(raw).unwrap();
            let input = tokens(case["input"].as_str().unwrap());
            let kerml = context_name == "org.omg.kerml.xtext.KerML";
            let executor = Executor {
                programs: partial_programs.language(kerml).unwrap(),
                tokens: &input,
                kind: "Element",
                kerml,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert_eq!(
                executor.decide(&decision, 0).unwrap(),
                Some(case["alternative"].as_u64().unwrap() as usize),
                "{case}"
            );
        }
    }

    #[test]
    fn experimental_finite_graph_reports_matched_and_unsupported_pilot_controls() {
        let finite: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction-nfa.experimental.json"
        ))
        .unwrap();
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/expanded-decision-pilot-controls.json"
        ))
        .unwrap();
        let partial: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-partial-programs.json"
        ))
        .unwrap();
        let partial_programs: Programs = serde_json::from_value(partial.clone()).unwrap();
        fn guards(value: &Value, result: &mut BTreeMap<String, Value>) {
            if value.get("predicated").and_then(Value::as_bool) == Some(true)
                || value.get("first_set_predicated").and_then(Value::as_bool) == Some(true)
            {
                result.insert(value["id"].as_str().unwrap().into(), value.clone());
            }
            match value {
                Value::Object(fields) => {
                    for child in fields.values() {
                        guards(child, result);
                    }
                }
                Value::Array(items) => {
                    for child in items {
                        guards(child, result);
                    }
                }
                _ => {}
            }
        }
        let mut bound_matched = 0;
        let mut negative_matched = 0;
        let mut bound_rejected = BTreeMap::<String, usize>::new();
        let mut matched = 0;
        let mut blocked_matched = 0;
        let mut rejected = BTreeMap::<String, usize>::new();
        for case in evidence["cases"]
            .as_array()
            .unwrap()
            .iter()
            .chain(finite["blocked_decision_controls"].as_array().unwrap())
            .chain(finite["negative_controls"].as_array().unwrap())
        {
            let context_name = case["context"].as_str().unwrap();
            let context = &imported["contexts"][context_name];
            let raw = &context[case["decision"].as_str().unwrap()];
            let graph: crate::xtext_nfa_experiment::Graph =
                serde_json::from_value(finite["contexts"][context_name].clone()).unwrap();
            let decision: Decision = serde_json::from_value(
                json!({"start":0,"states":[],"keywords":context["keywords"]}),
            )
            .unwrap();
            let input = tokens(case["input"].as_str().unwrap());
            let kerml = context_name == "org.omg.kerml.xtext.KerML";
            let executor = Executor {
                programs: partial_programs.language(kerml).unwrap(),
                tokens: &input,
                kind: "Element",
                kerml,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let (mut cursor, mut number) = (0, 0);
            let mut symbols = Vec::new();
            while let Some(symbol) = executor
                .decision_token(&decision, &mut cursor, &mut number)
                .unwrap()
            {
                symbols.push(symbol);
            }
            let fallback = format!(
                "{context_name}::{}{}",
                case["decision"].as_str().unwrap(),
                "/alternatives"
            );
            let mut bound_guards = BTreeMap::new();
            guards(
                &partial["language_programs"][if kerml { "kerml" } else { "sysml" }],
                &mut bound_guards,
            );
            let source_id = raw["source_id"].as_str().unwrap_or(&fallback);
            let bound_result =
                graph.predict_with_predicates(source_id, &symbols, 100_000, |condition| {
                    let source_id = condition["source_id"]
                        .as_str()
                        .ok_or("Unbound source guard identity")?;
                    let mut probe = bound_guards
                        .get(source_id)
                        .ok_or("Missing compiled source guard")?
                        .clone();
                    let first_set = condition["first_set"]
                        .as_bool()
                        .ok_or("Missing guard kind")?;
                    if probe["first_set_predicated"].as_bool().unwrap_or(false) != first_set {
                        return Err("Changed source guard kind".into());
                    }
                    probe["predicated"] = json!(false);
                    probe["first_set_predicated"] = json!(false);
                    probe["cardinality"] = json!("");
                    let condition = DecisionCondition::Syntax {
                        source_id: source_id.into(),
                        first_set,
                        probe: Some(Box::new(
                            serde_json::from_value(probe).map_err(|e| e.to_string())?,
                        )),
                    };
                    executor.decision_condition(&condition, 0)
                });
            match bound_result {
                Ok(selected) => {
                    assert_eq!(
                        selected,
                        Some(case["alternative"].as_u64().unwrap() as usize),
                        "bound {case}"
                    );
                    bound_matched += 1;
                    if case["accepted"] == json!(false) {
                        negative_matched += 1;
                    }
                }
                Err(reason) => {
                    *bound_rejected.entry(reason).or_default() += 1;
                }
            }
            assert!(executor.call_stack.borrow().is_empty());
            if case["accepted"] == json!(false) {
                continue;
            }
            match graph.predict(
                raw["source_id"].as_str().unwrap_or(&fallback),
                &symbols,
                100_000,
            ) {
                Ok(selected) => {
                    assert_eq!(
                        selected,
                        Some(case["alternative"].as_u64().unwrap() as usize),
                        "{case}"
                    );
                    matched += 1;
                    if raw.is_null() {
                        blocked_matched += 1;
                    }
                }
                Err(reason) => {
                    let category = reason.split(" at ").next().unwrap().to_string();
                    *rejected.entry(category).or_default() += 1;
                }
            }
        }
        println!(
            "Finite graph control assessment: {matched} matched; explicit exclusions {rejected:?}"
        );
        println!("Bound graph assessment: {bound_matched} matched; exclusions {bound_rejected:?}");
        assert_eq!(bound_matched, 52);
        assert_eq!(negative_matched, 7);
        assert!(bound_rejected.is_empty());
        assert_eq!(matched, 32);
        assert_eq!(blocked_matched, 7);
        assert_eq!(
            rejected,
            BTreeMap::from([("Unbound graph predicate".into(), 13)])
        );
    }

    #[test]
    fn upstream_repeated_type_body_decision_distinguishes_members_and_exit() {
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        let context = &imported["contexts"]["org.omg.kerml.xtext.KerML"];
        let mut raw = context["TypeBody#/elements/1/elements/1"].clone();
        assert_eq!(raw["exit_cardinality"], "*");
        raw["keywords"] = context["keywords"].clone();
        let decision: Decision = serde_json::from_value(raw).unwrap();
        assert_eq!(decision.exit_alternative, Some(4));
        for (source, expected) in [
            ("}", None),
            ("package P;", Some(0)),
            ("private import P::*;", Some(3)),
        ] {
            let input = tokens(source);
            let executor = Executor {
                programs: programs().unwrap().language(true).unwrap(),
                tokens: &input,
                kind: "Type",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert_eq!(executor.decide(&decision, 0).unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn upstream_optional_argument_decision_retains_exit_and_named_positional_choices() {
        let imported: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction.extract.json"
        ))
        .unwrap();
        for kernel in [true, false] {
            let context = if kernel {
                "org.omg.kerml.xtext.KerML"
            } else {
                "org.omg.sysml.xtext.SysML"
            };
            let mut raw = imported["contexts"][context]["ArgumentList"].clone();
            raw["keywords"] = imported["contexts"][context]["keywords"].clone();
            let decision: Decision = serde_json::from_value(raw).unwrap();
            assert_eq!(decision.exit_alternative, Some(2));
            for (source, expected) in [
                (")", None),
                ("1)", Some(0)),
                ("x)", Some(0)),
                ("x=1)", Some(1)),
                ("q::x=1)", Some(1)),
                ("1.25e-2)", Some(0)),
            ] {
                let input = tokens(source);
                let executor = Executor {
                    programs: programs().unwrap().language(kernel).unwrap(),
                    tokens: &input,
                    kind: "Expression",
                    kerml: kernel,
                    speculative: false,
                    call_stack: &RefCell::new(Vec::new()),
                    prediction_cache: &RefCell::new(BTreeMap::new()),
                };
                assert_eq!(executor.decide(&decision, 0).unwrap(), expected, "{source}");
            }
        }
    }

    #[test]
    fn feature_chain_adapter_matches_pilot_acceptance_and_preserves_groups() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/feature-chain-pilot-controls.json"
        )).unwrap();
        let mut checked = 0;
        let mut global_unsupported = 0;
        for case in evidence["cases"].as_array().unwrap().iter().filter(|c| c["rule"] == "FeatureChainMember") {
            let source = case["source"].as_str().unwrap();
            let input = tokens(source);
            let result = feature_chain_names(&input, case["language"] == "kerml");
            if source.starts_with("$::") {
                assert!(case["accepted"].as_bool().unwrap());
                assert!(result.unwrap_err().message.contains("global scope service"));
                global_unsupported += 1;
                continue;
            }
            let complete = result.as_ref().is_ok_and(|(consumed, _)| *consumed == input.len()-1);
            assert_eq!(complete, case["accepted"].as_bool().unwrap(), "{case}: {result:?}");
            checked += 1;
        }
        assert_eq!((checked, global_unsupported), (22, 2));
        for kerml in [true, false] {
            let input = tokens("a::b.c::d to next");
            let (consumed, names) = feature_chain_names(&input, kerml).unwrap();
            assert_eq!(names.iter().map(|n| n.segments.clone()).collect::<Vec<_>>(),
                vec![vec!["a", "b"], vec!["c", "d"]]);
            assert!(matches!(&input[consumed].kind, TokenKind::Identifier(s) if s == "to"));
            assert_eq!((names[0].span.start_col, names[0].span.end_col), (1,4));
            assert_eq!((names[1].span.start_col, names[1].span.end_col), (6,9));
            let (_, names) = feature_chain_names(&tokens("'a.b'.'c::d';"), kerml).unwrap();
            assert_eq!(names[0].segments, ["a.b"]);
            assert_eq!(names[1].segments, ["c::d"]);
            assert!(feature_chain_names(&tokens("a. to b"), kerml).is_err());
        }
    }

    #[test]
    fn upstream_prediction_constructs_feature_chains_matching_pilot() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/feature-chain-pilot-controls.json"
        ))
        .unwrap();
        fn contained(model: &Match, output: &mut Vec<String>) {
            for child in model.children.values().flatten() {
                output.push(child.object_kind.clone().unwrap());
                contained(child, output);
            }
        }
        for case in evidence["cases"].as_array().unwrap() {
            let kernel = case["language"] == "kerml";
            let input = tokens(case["source"].as_str().unwrap());
            let program = programs().unwrap().language(kernel).unwrap();
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program,
                tokens: &input,
                kind: "Membership",
                kerml: kernel,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let original = Match::default();
            let result = executor.rule(
                &format!(
                    "org.omg.kerml.expressions.xtext.KerMLExpressions::{}",
                    case["rule"].as_str().unwrap()
                ),
                &original,
            );
            let complete = matches!(&result, Ok(Some(model)) if model.consumed == input.len() - 1);
            assert_eq!(
                complete,
                case["accepted"].as_bool().unwrap(),
                "{case}: {result:?}"
            );
            if complete {
                let model = result.unwrap().unwrap();
                assert_eq!(
                    model.object_kind.as_deref(),
                    case["kind"].as_str(),
                    "{case}"
                );
                let mut children = Vec::new();
                contained(&model, &mut children);
                assert_eq!(json!(children), case["contained_kinds"], "{case}");
            }
            assert_eq!(
                original,
                Match::default(),
                "prediction/model staging mutated its caller"
            );
            assert!(stack.borrow().is_empty());
        }
    }

    #[test]
    fn optional_metadata_entry_compares_pilot_with_explicit_name_boundary() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/metadata-entry-pilot-controls.json"
        ))
        .unwrap();
        let root = "org.omg.kerml.xtext.KerML::MetadataFeatureDeclaration";
        let program = programs().unwrap().language(true).unwrap();
        let mut quoted_name_disagreements = 0;
        for case in evidence["cases"].as_array().unwrap() {
            // The fragment's real caller supplies the MetadataFeature current
            // object and MetadataBody follower; the standalone fragment is not
            // an independent entry rule in Pilot.
            let input = tokens(&format!("{};", case["source"].as_str().unwrap()));
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program,
                tokens: &input,
                kind: "MetadataFeature",
                kerml: true,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let original = Match {
                object_kind: Some("MetadataFeature".into()),
                ..Match::default()
            };
            let result = executor.rule(root, &original);
            let complete = matches!(&result, Ok(Some(model)) if model.consumed == input.len() - 2);
            assert_eq!(
                complete,
                case["accepted"].as_bool().unwrap(),
                "{case}: {result:?}"
            );
            let mut probe = SyntaxProbe {
                executor: &executor,
                active: Vec::new(),
                memo: BTreeMap::new(),
            };
            let ends = probe.body(&program.rules[root].body, 0).unwrap();
            assert_eq!(ends.contains(&(input.len() - 2)), complete, "probe {case}");
            if complete {
                let model = result.unwrap().unwrap();
                let fields: BTreeMap<_, _> = model
                    .fields
                    .iter()
                    .map(|(key, value)| {
                        let upstream = match key.as_str() {
                            "declared_name" => "declaredName",
                            "declared_short_name" => "declaredShortName",
                            other => panic!("Unexpected metadata field {other}"),
                        };
                        (upstream, value)
                    })
                    .collect();
                if case["source"] == "'m' : 'T'" {
                    // Raw Pilot declaredName stores lexical quotes. Native Name
                    // follows KerML 8.2.2.3 represented-name semantics; the
                    // normalized-name corpus checks Pilot unescapeString separately.
                    assert_eq!(case["fields"], json!({"declaredName": "'m'"}));
                    assert_eq!(json!(fields), json!({"declaredName": "m"}));
                    quoted_name_disagreements += 1;
                } else {
                    assert_eq!(json!(fields), case["fields"], "{case}");
                }
                assert_eq!(json!(model.object_kind), case["kind"], "{case}");
                let typing = &model.children["owned_relationship"];
                assert_eq!(typing.len(), 1);
                assert_eq!(json!([typing[0].object_kind]), case["contained_kinds"]);
                assert_eq!(
                    typing[0].links["type"][0].target_type.rsplit('/').next(),
                    Some("Metaclass")
                );
                assert_eq!(
                    json!(typing[0].links["type"][0].spelling),
                    case["type_text"],
                    "{case}"
                );
            }
            assert_eq!(
                original,
                Match {
                    object_kind: Some("MetadataFeature".into()),
                    ..Match::default()
                }
            );
            assert!(stack.borrow().is_empty());
        }
        assert_eq!(quoted_name_disagreements, 1);
    }

    #[test]
    fn generated_two_symbol_decisions_construct_models_and_commit_failures() {
        let all: Programs = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/xtext-context-controls.json"
        ))
        .unwrap();
        let program = &all.language_programs["prediction"];
        let root = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralExpression";
        for (source, value, kind) in [
            (". 7;", json!(7), "LiteralInteger"),
            (". (true)", json!(true), "LiteralBoolean"),
            (". [9] . (false)", json!(false), "LiteralBoolean"),
        ] {
            let input = tokens(source);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program,
                tokens: &input,
                kind: "LiteralExpression",
                kerml: true,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let before = Match::default();
            let result = executor.rule(root, &before).unwrap().unwrap();
            assert_eq!(result.fields["value"], value, "{source}");
            assert_eq!(result.object_kind.as_deref(), Some(kind));
            assert_eq!(result.consumed, input.len() - 1);
            assert!(before.fields.is_empty());
            assert!(stack.borrow().is_empty());
        }
        for source in [". 7)", ". (true;", ". [9) . 7;", ". [9] . ("] {
            let input = tokens(source);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: program,
                tokens: &input,
                kind: "LiteralExpression",
                kerml: true,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert!(executor.rule(root, &Match::default()).is_err(), "{source}");
            assert!(stack.borrow().is_empty());
        }
    }

    #[test]
    fn syntactic_predicates_probe_without_model_mutation_and_commit_on_success() {
        let mut raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        // Both branches start with a number; only the first includes a suffix.
        let integer = json!({"id":"integer","cardinality":"","kind":"call",
            "rule":"org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralInteger"});
        let guarded = json!({"id":"guarded","cardinality":"","predicated":true,
            "kind":"sequence","elements":[integer,
                {"id":"suffix","cardinality":"","kind":"keyword","value":";"}]});
        raw["rules"]["predicate"] = json!({"owner":"https://www.omg.org/spec/SysML/20250201#//Expression",
            "scalar":false,"construct":true,"body":{"id":"choice","cardinality":"",
            "kind":"choice","elements":[guarded, integer]}});
        let program: Programs = serde_json::from_value(raw.clone()).unwrap();
        for (text, consumed) in [("7;", 2), ("7", 1)] {
            let input = tokens(text);
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let result = executor
                .rule("predicate", &Match::default())
                .unwrap()
                .unwrap();
            assert_eq!(result.fields["value"], 7);
            assert_eq!(result.consumed, consumed);
            assert!(executor.call_stack.borrow().is_empty());
        }
        let mut first_set = raw.clone();
        first_set["rules"]["predicate"]["body"]["elements"][0]["predicated"] = json!(false);
        first_set["rules"]["predicate"]["body"]["elements"][0]["first_set_predicated"] =
            json!(true);
        let first_set: Programs = serde_json::from_value(first_set).unwrap();
        let input = tokens("7");
        let executor = Executor {
            programs: &first_set,
            tokens: &input,
            kind: "Expression",
            kerml: true,
            speculative: false,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        // First-set lookahead commits on the number, even without the suffix.
        // Full syntactic lookahead above instead selects the fallback.
        assert!(
            executor
                .rule("predicate", &Match::default())
                .unwrap_err()
                .contains("Incomplete Xtext group")
        );
        assert!(executor.call_stack.borrow().is_empty());
        let mut nested = raw.clone();
        nested["rules"]["predicate"]["body"]["elements"][0]["elements"][0]["cardinality"] =
            json!("?");
        nested["rules"]["predicate"]["body"]["elements"][0]["elements"][0]["first_set_predicated"] =
            json!(true);
        nested["rules"]["predicate"]["body"]["elements"][0]["elements"][1] = integer.clone();
        let nested: Programs = serde_json::from_value(nested).unwrap();
        let executor = Executor {
            programs: &nested,
            tokens: &input,
            kind: "Expression",
            kerml: true,
            speculative: false,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        // The nested optional first-set predicate commits to the sole integer.
        // Lookahead must not backtrack it to epsilon to satisfy the second integer.
        assert_eq!(
            executor
                .rule("predicate", &Match::default())
                .unwrap()
                .unwrap()
                .fields["value"],
            7
        );
        // Failed lookahead must not execute an invalid semantic action.
        // Successful lookahead must execute it and report its real error.
        raw["rules"]["predicate"]["body"]["elements"][0]["elements"][0] = json!({
        "id":"invalid-action","kind":"sequence","cardinality":"","elements":[
            {"id":"action","kind":"capture","cardinality":"",
             "classifier":"bad","feature_id":"bad","captured_types":[]}, integer
        ]});
        let program: Programs = serde_json::from_value(raw).unwrap();
        for (text, error) in [("7", false), ("7;", true)] {
            let input = tokens(text);
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let result = executor.rule("predicate", &Match::default());
            assert_eq!(result.is_err(), error);
            if !error {
                assert_eq!(result.unwrap().unwrap().fields["value"], 7);
            }
            assert!(executor.call_stack.borrow().is_empty());
        }
    }

    #[test]
    fn repeated_predicates_require_a_complete_occurrence_and_bound_recursion() {
        let mut raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        let keyword = |id: &str, value: &str| json!({"id":id,"kind":"keyword","cardinality":"","value":value});
        raw["rules"]["guarded"] = json!({"owner":"https://www.omg.org/spec/SysML/20250201#//Expression",
            "scalar":false, "construct":true,"body":{"id":"repeated","kind":"sequence","cardinality":"",
                "elements":[{"id":"item","kind":"sequence","cardinality":"*","predicated":true,
                    "elements":[keyword("a","a"),keyword("b","b")]},
                    keyword("last-a","a"), keyword("last-c","c")]}});
        let program: Programs = serde_json::from_value(raw.clone()).unwrap();
        for source in ["a c", "a b a c", "a b a b a c"] {
            let input = tokens(source);
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert_eq!(
                executor
                    .rule("guarded", &Match::default())
                    .unwrap()
                    .unwrap()
                    .consumed,
                input.len() - 1
            );
        }
        raw["rules"]["guarded"]["body"] = json!({"id":"recursive","kind":"choice","cardinality":"",
            "elements":[{"id":"group","kind":"sequence","cardinality":"","elements":[
                keyword("open","("), {"id":"call","kind":"call","cardinality":"","rule":"guarded"},
                keyword("close",")")]}, keyword("leaf","a")]});
        let program: Programs = serde_json::from_value(raw).unwrap();
        for (source, error) in [
            ("((a))".to_string(), false),
            (format!("{}a{}", "(".repeat(80), ")".repeat(80)), false),
            (format!("{}a{}", "(".repeat(257), ")".repeat(257)), true),
        ] {
            let input = tokens(&source);
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let mut probe = SyntaxProbe {
                executor: &executor,
                active: Vec::new(),
                memo: BTreeMap::new(),
            };
            let result = probe.node(&program.rules["guarded"].body, 0);
            assert_eq!(result.is_err(), error);
            if !error {
                assert!(result.unwrap().contains(&(input.len() - 1)));
            }
            assert!(probe.active.is_empty());
        }
    }

    #[test]
    fn textual_representation_rules_match_pinned_raw_parser_values() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/comment-values-pilot-controls.json"
        )).unwrap();
        let mut checked = 0;
        for case in evidence["cases"].as_array().unwrap().iter()
            .filter(|c| c["kind"] == "TextualRepresentation") {
            let kerml = case["language"] == "kerml";
            let grammar = if kerml { "org.omg.kerml.xtext.KerML" } else { "org.omg.sysml.xtext.SysML" };
            let rule = format!("{grammar}::TextualRepresentation");
            let input = tokens(case["source"].as_str().unwrap());
            let result = execute_in_context(&rule, &input, "TextualRepresentation", kerml).unwrap().unwrap();
            assert_eq!(result.consumed, input.len() - 1);
            assert_eq!(result.object_kind.as_deref(), Some("TextualRepresentation"));
            for (native, upstream) in [("body", "body"), ("language", "languageValue"),
                ("declared_name", "declaredName"), ("declared_short_name", "declaredShortName")] {
                assert_eq!(result.fields.get(native).unwrap_or(&Value::Null), &case[upstream]);
            }
            checked += 1;
        }
        assert_eq!(checked, 10);
    }

    #[test]
    fn qualified_name_adapter_preserves_boundaries_values_and_context() {
        for kerml in [true, false] {
            for (source, expected) in [
                ("P::A;", vec!["P", "A"]),
                ("'A::B'::'C.D';", vec!["A::B", "C.D"]),
                ("P :: 'B\\tC';", vec!["P", "B\tC"]),
                ("'package'::'doc';", vec!["package", "doc"]),
            ] {
                let input = tokens(source);
                let (consumed, name) = qualified_name(&input, kerml).unwrap();
                assert_eq!(name.segments, expected);
                assert!(matches!(input[consumed].kind, TokenKind::Semicolon));
                assert_eq!(name.span.start_col, 1);
                assert_eq!(name.span.end_col, source.len()-1);
            }
            // A fragment may leave a separator for its enclosing rule (e.g. import wildcards).
            let input = tokens("P::;");
            let (consumed, _) = qualified_name(&input, kerml).unwrap();
            assert!(matches!(input[consumed].kind, TokenKind::ScopeSep));
            assert!(qualified_name(&tokens("package;"), kerml).is_err());
            assert_eq!(qualified_name(&tokens("part;"), kerml).is_ok(), kerml);
            // Dot is an enclosing FeatureChain operation, not a QualifiedName separator.
            let input = tokens("P.member");
            let (consumed, name) = qualified_name(&input, kerml).unwrap();
            assert_eq!(name.segments, ["P"]);
            assert!(matches!(input[consumed].kind, TokenKind::Dot));
        }
    }

    #[test]
    fn comment_rules_match_pinned_raw_parser_values_and_typed_targets() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/comment-values-pilot-controls.json"
        )).unwrap();
        let mut checked = 0;
        for case in evidence["cases"].as_array().unwrap().iter().filter(|c| c["kind"] == "Comment") {
            let kerml = case["language"] == "kerml";
            let grammar = if kerml { "org.omg.kerml.xtext.KerML" } else { "org.omg.sysml.xtext.SysML" };
            let input = tokens(case["source"].as_str().unwrap());
            let result = execute_in_context(&format!("{grammar}::Comment"), &input, "Comment", kerml).unwrap().unwrap();
            assert_eq!(result.consumed, input.len()-1);
            assert_eq!(result.object_kind.as_deref(), Some("Comment"));
            for (native, upstream) in [("body", "body"), ("locale", "locale"),
                ("declared_name", "declaredName"), ("declared_short_name", "declaredShortName")] {
                assert_eq!(result.fields.get(native).unwrap_or(&Value::Null), &case[upstream]);
            }
            let children = result.children.get("owned_relationship").cloned().unwrap_or_default();
            let expected = case["references"].as_array().unwrap();
            assert_eq!(children.len(), expected.len());
            for (child, reference) in children.iter().zip(expected) {
                assert_eq!(child.object_kind.as_deref(), reference["kind"].as_str());
                let links = &child.links["annotated_element"];
                assert_eq!(links.len(), 1);
                assert_eq!(links[0].spelling, reference["spelling"]);
                assert!(links[0].target_type.ends_with(reference["targetType"].as_str().unwrap()));
                assert!(links[0].span.is_some());
                let input = tokens(&format!("{};", reference["spelling"].as_str().unwrap()));
                let (consumed, name) = qualified_name(&input, kerml).unwrap();
                let expected = match reference["spelling"].as_str().unwrap() {
                    "P::A" => vec!["P", "A"], "'B.C'" => vec!["B.C"], "A" => vec!["A"],
                    other => panic!("unassessed reference carrier: {other}"),
                };
                assert_eq!(name.segments, expected);
                assert!(matches!(input[consumed].kind, TokenKind::Semicolon));

            }
            checked += 1;
        }
        assert_eq!(checked, 12);
    }

    #[test]
    fn documentation_rules_match_pinned_raw_parser_values() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/comment-values-pilot-controls.json"
        ))
        .unwrap();
        for case in evidence["cases"].as_array().unwrap() {
            if case["kind"] == "TextualRepresentation" || case["kind"] == "Comment" || case["kind"] == "RootNamespace" { continue; }
            if case["kind"] == "LiteralString" {
                let input = tokens(case["source"].as_str().unwrap());
                let result = execute(
                    "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralString",
                    &input,
                    "LiteralString",
                )
                .unwrap()
                .unwrap();
                assert_eq!(result.fields["value"], case["value"]);
                assert_eq!(result.consumed, input.len() - 1);
                continue;
            }
            let rule = if case["language"] == "kerml" {
                "org.omg.kerml.xtext.KerML::Documentation"
            } else {
                "org.omg.sysml.xtext.SysML::Documentation"
            };
            let input = tokens(case["source"].as_str().unwrap());
            let result = execute(rule, &input, "Documentation").unwrap().unwrap();
            assert_eq!(result.consumed, input.len() - 1);
            assert_eq!(result.object_kind.as_deref(), Some("Documentation"));
            for (native, upstream) in [
                ("body", "body"),
                ("locale", "locale"),
                ("declared_name", "declaredName"),
            ] {
                assert_eq!(
                    result.fields.get(native).unwrap_or(&Value::Null),
                    &case[upstream]
                );
            }
            for invalid in ["doc", "doc D locale /* body */", "doc D \"body\""] {
                assert!(execute(rule, &tokens(invalid), "Documentation").is_err());
            }
        }
        assert!(crate::xtext_terminal::lex("doc /* open").is_err());
        // Generated terminal recognition still rejects a forged multi-comment carrier.
        let mut input = tokens("doc /* valid */");
        input[1].kind = TokenKind::BlockDoc("a */ /* b".into());
        assert!(
            execute(
                "org.omg.kerml.xtext.KerML::Documentation",
                &input,
                "Documentation"
            )
            .is_err()
        );
    }

    #[test]
    fn resolved_language_contexts_construct_distinct_inherited_objects() {
        // The source override/call/containment identities are pinned; only the
        // ExpressionBody bodies are controlled replacements. This does not
        // qualify the complete Pilot ExpressionBody grammar.
        let document: Programs = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/xtext-context-controls.json"
        ))
        .unwrap();
        let member = "org.omg.kerml.expressions.xtext.KerMLExpressions::ExpressionBodyMember";
        let body = "org.omg.kerml.expressions.xtext.KerMLExpressions::ExpressionBody";
        for (language, source, kind, value) in [
            ("base", "7", "LiteralInteger", json!(7)),
            ("kerml", "true", "LiteralBoolean", json!(true)),
            ("sysml", r#""text""#, "LiteralString", json!(r#""text""#)),
        ] {
            let programs = &document.language_programs[language];
            let input = tokens(source);
            let executor = Executor {
                programs,
                tokens: &input,
                kind: "Element",
                kerml: language != "sysml",
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let result = executor.rule(member, &Match::default()).unwrap().unwrap();
            assert_eq!(result.consumed, input.len() - 1);
            assert_eq!(result.object_kind.as_deref(), Some("FeatureMembership"));
            let children = &result.children["owned_related_element"];
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].object_kind.as_deref(), Some(kind));
            assert_eq!(children[0].fields["value"], value);
            assert_eq!(
                children[0].container_feature.as_deref(),
                Some("owning_relationship")
            );
            let direct = executor.rule(body, &Match::default()).unwrap().unwrap();
            assert_eq!(direct.object_kind.as_deref(), Some(kind));
            assert_eq!(direct.fields["value"], value);
            assert!(executor.call_stack.borrow().is_empty());
            for rejected in ["7", "true", r#""text""#]
                .into_iter()
                .filter(|s| *s != source)
            {
                let other = tokens(rejected);
                let attempt = Executor {
                    tokens: &other,
                    ..executor
                };
                assert!(attempt.rule(member, &Match::default()).unwrap().is_none());
                assert!(attempt.call_stack.borrow().is_empty());
            }
        }
        // Missing language selection cannot silently fall back to neutral rules.
        assert!(document.language(true).is_ok());
        let neutral: Programs = serde_json::from_value(json!({"rules":{}})).unwrap();
        assert!(neutral.language(true).is_err());
        assert!(
            !programs()
                .unwrap()
                .language(true)
                .unwrap()
                .rules
                .contains_key("org.omg.sysml.xtext.SysML::Identification")
        );
    }

    #[test]
    fn definition_enum_family_matches_all_pilot_rule_controls() {
        let artifact: Value = serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/enum-family-pilot-controls.json")).unwrap();
        let cases = artifact["controls"].as_array().unwrap();
        assert_eq!(cases.len(), 72);
        let mut declarations = BTreeSet::new();
        for case in cases {
            let language = case["language"].as_str().unwrap();
            let kerml = language == "org.omg.kerml.xtext.KerML";
            let rule = case["rule"].as_str().unwrap();
            declarations.insert((language, rule, case["token"].as_str().unwrap()));
            let input = tokens(case["source"].as_str().unwrap());
            let candidate = candidate_programs().unwrap().language(kerml).unwrap();
            let found = Executor { programs: candidate, tokens: &input, kind: "Element", kerml,
                speculative: false, call_stack: &RefCell::new(Vec::new()), prediction_cache: &RefCell::new(BTreeMap::new())
            }.rule(&format!("{language}::{rule}"), &Match::default()).unwrap();
            let complete = found.filter(|m| input.get(m.consumed).is_some_and(|token| matches!(token.kind, TokenKind::Eof)));
            assert_eq!(complete.is_some(), case["accepted"] == true, "{case}");
            if let Some(found) = complete {
                assert_eq!(found.value, Some(case["literal"].clone()), "{case}");
                let mapping = crate::enum_grammar::lookup(kerml, rule, case["token"].as_str().unwrap()).unwrap();
                assert_eq!(mapping.literal, case["literal"].as_str().unwrap());
                assert_eq!(json!(mapping.value), case["value"]);
            }
        }
        assert_eq!(declarations.len(), 24);
    }

    #[test]
    fn definition_enum_family_matches_all_pilot_caller_values() {
        let artifact: Value = serde_json::from_str(include_str!("../../../docs/conformance/2026-08-support/enum-family-pilot-controls.json")).unwrap();
        let cases = artifact["model_controls"].as_array().unwrap();
        assert_eq!(cases.len(), 34);
        for case in cases {
            let language = case["language"].as_str().unwrap();
            let field = if case["field"] == "portionKind" { "portion_kind" } else { case["field"].as_str().unwrap() }.to_owned();
            let tree = experimental_model(case["source"].as_str().unwrap(), &format!("{language}::{}", case["carrier_rule"].as_str().unwrap()), language == "org.omg.kerml.xtext.KerML").unwrap();
            assert_eq!(tree.object_kind.as_deref(), case["kind"].as_str(), "{case}");
            assert_eq!(tree.fields[&field], case["literal"], "{case}");
        }
    }

    #[test]
    fn enum_returning_datatype_uses_the_pinned_keyword_subset() {
        let rule = "org.omg.sysml.xtext.SysML::TimeTriggerKind";
        for spelling in ["at", "after"] {
            let input = tokens(spelling);
            let result = execute(rule, &input, "Element").unwrap().unwrap();
            assert_eq!(result.value, Some(json!(spelling)));
            assert_eq!(result.consumed, 1);
            assert!(result.object_kind.is_none());
        }
        // 'when' is a valid TriggerKind literal but is outside this rule.
        for spelling in ["when", "before", "'at'"] {
            assert!(
                execute(rule, &tokens(spelling), "Element")
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn recursive_fragments_augment_the_existing_subtype_and_fields() {
        let mut raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        raw["rules"]["fragment"] = json!({"owner":"https://www.omg.org/spec/SysML/20250201#//Element",
            "scalar":false,"construct":false,"preserves_current":true,
            "body":{"id":"choice","kind":"choice","cardinality":"","elements":[
                {"id":"nested","kind":"sequence","cardinality":"","elements":[
                    {"id":"open","kind":"keyword","cardinality":"","value":"("},
                    {"id":"call","kind":"call","cardinality":"","rule":"fragment"},
                    {"id":"close","kind":"keyword","cardinality":"","value":")"}]},
                {"id":"name","kind":"assign","cardinality":"","operator":"=","feature":"declared_name",
                 "feature_id":"https://www.omg.org/spec/SysML/20250201#//Element/declaredName",
                 "terminal":{"id":"Name","kind":"name","cardinality":""}}]}});
        let program: Programs = serde_json::from_value(raw).unwrap();
        let input = tokens("(((Vehicle)))");
        let executor = Executor {
            programs: &program,
            tokens: &input,
            kind: "PartUsage",
            kerml: false,
            speculative: false,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let mut original = Match {
            object_kind: Some("PartUsage".into()),
            ..Match::default()
        };
        original.fields.insert("is_ordered".into(), json!(true));
        let result = executor.rule("fragment", &original).unwrap().unwrap();
        assert_eq!(result.object_kind.as_deref(), Some("PartUsage"));
        assert_eq!(result.fields["declared_name"], "Vehicle");
        assert_eq!(result.fields["is_ordered"], true);
        assert_eq!(result.consumed, input.len() - 1);
        assert!(!original.fields.contains_key("declared_name"));
        assert!(executor.call_stack.borrow().is_empty());
    }

    #[test]
    fn recursive_rule_execution_requires_progress_and_bounds_nesting() {
        let literal = json!({"id":"integer","cardinality":"","kind":"call",
            "rule":"org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralInteger"});
        let mut raw: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        raw["rules"]["recursive"] = json!({
            "owner":"https://www.omg.org/spec/SysML/20250201#//Expression",
            "scalar":false, "construct":true,
            "body":{"id":"body","cardinality":"","kind":"choice","elements":[
                {"id":"paren","cardinality":"","kind":"sequence","elements":[
                    {"id":"open","cardinality":"","kind":"keyword","value":"("},
                    {"id":"recur","cardinality":"","kind":"call","rule":"recursive"},
                    {"id":"close","cardinality":"","kind":"keyword","value":")"}
                ]}, literal
            ]}
        });
        let program: Programs = serde_json::from_value(raw.clone()).unwrap();
        for text in [
            "7",
            "(7)",
            "(((7)))",
            &format!("{}7{}", "(".repeat(80), ")".repeat(80)),
        ] {
            let input = tokens(text);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let result = executor
                .rule("recursive", &Match::default())
                .unwrap()
                .unwrap();
            assert_eq!(result.fields["value"], 7);
            assert_eq!(result.object_kind.as_deref(), Some("LiteralInteger"));
            assert_eq!(result.consumed, input.len() - 1);
            assert!(stack.borrow().is_empty());
        }
        for text in ["((7)", &format!("{}7{}", "(".repeat(300), ")".repeat(300))] {
            let input = tokens(text);
            let stack = RefCell::new(Vec::new());
            let executor = Executor {
                programs: &program,
                tokens: &input,
                kind: "Expression",
                kerml: true,
                speculative: false,
                call_stack: &stack,
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            assert!(executor.rule("recursive", &Match::default()).is_err());
            assert!(
                stack.borrow().is_empty(),
                "failure must unwind the call guard"
            );
        }
        raw["rules"]["recursive"]["body"] = json!({
            "id":"cycle","cardinality":"","kind":"call","rule":"recursive"
        });
        let invalid: Programs = serde_json::from_value(raw).unwrap();
        let input = tokens("7");
        let stack = RefCell::new(Vec::new());
        let executor = Executor {
            programs: &invalid,
            tokens: &input,
            kind: "Expression",
            kerml: true,
            speculative: false,
            call_stack: &stack,
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        assert!(
            executor
                .rule("recursive", &Match::default())
                .unwrap_err()
                .contains("Non-consuming")
        );
        assert!(stack.borrow().is_empty());
    }

    #[test]
    fn fragment_identification_executes_choices_groups_and_assignments() {
        for language in ["org.omg.kerml.xtext.KerML", "org.omg.sysml.xtext.SysML"] {
            let rule = format!("{language}::Identification");
            let result = execute(&rule, &tokens("<short> 'Long Name';"), "PartDefinition")
                .unwrap()
                .unwrap();
            assert_eq!(result.consumed, 4);
            assert_eq!(result.fields["declared_short_name"], "short");
            assert_eq!(result.fields["declared_name"], "Long Name");
            let short_only = execute(&rule, &tokens("<short>;"), "Element")
                .unwrap()
                .unwrap();
            assert_eq!(short_only.consumed, 3);
            assert!(!short_only.fields.contains_key("declared_name"));
            assert!(execute(&rule, &tokens(";"), "Element").unwrap().is_none());
            for malformed in ["<>", "<short", "<a b> Name"] {
                assert!(
                    execute(&rule, &tokens(malformed), "Element").is_err(),
                    "{malformed}"
                );
            }
            assert!(execute(&rule, &tokens("Name"), "UnknownClass").is_err());
        }
    }

    #[test]
    fn represented_names_decode_escapes_without_changing_reference_spelling() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/name-values-pilot-controls.json"
        ))
        .unwrap();
        assert_eq!(evidence["cases"].as_array().unwrap().len(), 26);
        for case in evidence["cases"].as_array().unwrap() {
            let kernel = case["kerml"].as_bool().unwrap();
            let grammar = if kernel {
                "org.omg.kerml.xtext.KerML"
            } else {
                "org.omg.sysml.xtext.SysML"
            };
            let spelling = case["spelling"].as_str().unwrap();
            let input = tokens(spelling);
            let before = input.clone();
            let model = execute_in_context(
                &format!("{grammar}::Identification"),
                &input,
                "Element",
                kernel,
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                model.fields["declared_name"], case["represented_name"],
                "{case}"
            );
            assert_eq!(input, before);
            let reference = execute_in_context(
                &format!("{grammar}::OwnedSubsetting"),
                &input,
                "Subsetting",
                kernel,
            )
            .unwrap()
            .unwrap();
            assert_eq!(reference.links["subsetted_feature"][0].spelling, spelling);
        }
        for invalid in [r"'a\qb'", r"'a\ub'", "'unterminated"] {
            assert!(crate::xtext_terminal::lex(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn fragment_name_adapter_separates_keywords_and_quoted_identifiers() {
        let rule = "org.omg.sysml.xtext.SysML::Identification";
        for (input, expected) in [
            ("'<'", "<"),
            ("'action'", "action"),
            ("'two words'", "two words"),
            ("'ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©'", "ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©"),
        ] {
            let result = execute(rule, &tokens(input), "Element").unwrap().unwrap();
            assert_eq!(result.fields["declared_name"], expected);
        }
        assert!(
            execute(rule, &tokens("action"), "Element")
                .unwrap()
                .is_none()
        );
        assert!(crate::xtext_terminal::lex("ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©").is_err());
        let member = execute(
            "org.omg.sysml.xtext.SysML::MemberPrefix",
            &tokens("'private'"),
            "Membership",
        )
        .unwrap()
        .unwrap();
        assert_eq!(member.consumed, 0);
        assert!(member.fields.is_empty());
    }

    #[test]
    fn fragment_prefixes_apply_enum_and_boolean_values_in_grammar_order() {
        let rule = "org.omg.kerml.xtext.KerML::BasicFeaturePrefix";
        let result = execute(
            rule,
            &tokens("inout derived abstract composite var tail"),
            "Feature",
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.consumed, 5);
        assert_eq!(result.fields["direction"], "inout");
        for flag in ["is_derived", "is_abstract", "is_composite", "is_variable"] {
            assert_eq!(result.fields[flag], true);
        }
        let reordered = execute(rule, &tokens("var derived"), "Feature")
            .unwrap()
            .unwrap();
        assert_eq!(reordered.consumed, 1);
        assert!(!reordered.fields.contains_key("is_derived"));
        for language in ["org.omg.kerml.xtext.KerML", "org.omg.sysml.xtext.SysML"] {
            let member = execute(
                &format!("{language}::MemberPrefix"),
                &tokens("protected tail"),
                "OwningMembership",
            )
            .unwrap()
            .unwrap();
            assert_eq!(member.fields["visibility"], "protected");
            assert_eq!(member.consumed, 1);
            assert!(
                execute(
                    &format!("{language}::MemberPrefix"),
                    &tokens(";"),
                    "Membership"
                )
                .unwrap()
                .unwrap()
                .fields
                .is_empty()
            );
        }
    }

    #[test]
    fn fragment_repetition_requires_progress_and_preserves_prefix_boundaries() {
        for (cardinality, input, count, present) in [
            ("*", "", 0, true),
            ("+", "", 0, false),
            ("+", "x x tail", 2, true),
        ] {
            let program: Programs = serde_json::from_value(json!({"rules":{"test":{
                "owner":"https://www.omg.org/spec/SysML/20250201#//Element", "scalar":false,
                "body":{"id":"test", "cardinality":cardinality, "kind":"keyword", "value":"x"}
            }}}))
            .unwrap();
            let input = tokens(input);
            let result = Executor {
                programs: &program,
                tokens: &input,
                kind: "Element",
                kerml: false,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            }
            .rule("test", &Match::default())
            .unwrap();
            assert_eq!(result.is_some(), present);
            if let Some(result) = result {
                assert_eq!(result.consumed, count);
            }
        }
    }

    #[test]
    fn constructed_literal_roots_use_ecore_types_and_transactional_assignments() {
        let controls = [
            ("LiteralBoolean", "true tail", json!(true)),
            ("LiteralBoolean", "false", json!(false)),
            ("LiteralInteger", "00042 tail", json!(42)),
            ("LiteralInteger", "2147483647", json!(2147483647)),
            (
                "LiteralString",
                "\"hello ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©\"",
                json!(r#""hello ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â©""#),
            ),
            // Escape conversion remains a named, unqualified boundary.
            ("LiteralString", r#""a\n\"b""#, json!(r#""a\n\"b""#)),
        ];
        for (kind, source, value) in controls {
            let input = tokens(source);
            let root = format!("org.omg.kerml.expressions.xtext.KerMLExpressions::{kind}");
            let prior = Match {
                fields: BTreeMap::from([("declared_name".into(), json!("old"))]),
                ..Match::default()
            };
            let executor = Executor {
                programs: programs().unwrap(),
                tokens: &input,
                kind: "Element",
                kerml: true,
                speculative: false,
                call_stack: &RefCell::new(Vec::new()),
                prediction_cache: &RefCell::new(BTreeMap::new()),
            };
            let matched = executor.rule(&root, &prior).unwrap().unwrap();
            assert_eq!(matched.consumed, 1);
            assert_eq!(matched.object_kind.as_deref(), Some(kind));
            assert_eq!(
                matched.fields,
                BTreeMap::from([("value".into(), value.clone())])
            );
            ecore_model::validate_value(kind, "value", &value).unwrap();
            assert_eq!(
                prior.fields["declared_name"], "old",
                "input state was mutated"
            );
        }
        let integer = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralInteger";
        assert!(
            execute(integer, &tokens("2147483648"), "Element")
                .unwrap_err()
                .contains("EInt")
        );
        for input in ["false", "1.5", "1e3", "'123'", "-1", ";"] {
            assert!(
                execute(integer, &tokens(input), "Element")
                    .unwrap()
                    .is_none(),
                "{input}"
            );
        }
        let boolean = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralBoolean";
        assert!(
            execute(boolean, &tokens("'true'"), "Element")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn simple_actions_create_fieldless_objects_and_rollback_failed_branches() {
        for (name, source, count) in [
            ("LiteralInfinity", "* tail", 1),
            ("NullExpression", "null tail", 1),
            ("NullExpression", "( ) tail", 2),
        ] {
            let result = execute(
                &format!("org.omg.kerml.expressions.xtext.KerMLExpressions::{name}"),
                &tokens(source),
                "Element",
            )
            .unwrap()
            .unwrap();
            assert_eq!(result.object_kind.as_deref(), Some(name));
            assert!(result.fields.is_empty());
            assert_eq!(result.consumed, count);
        }
        let input = tokens("(42)");
        let prior = Match {
            object_kind: Some("LiteralInteger".into()),
            fields: BTreeMap::from([("value".into(), json!(99))]),
            ..Match::default()
        };
        let saved = prior.clone();
        let executor = Executor {
            programs: programs().unwrap(),
            tokens: &input,
            kind: "Element",
            kerml: true,
            speculative: true,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        assert!(
            executor
                .rule(
                    "org.omg.kerml.expressions.xtext.KerMLExpressions::NullExpression",
                    &prior
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(prior, saved);
        for input in ["(42)", "'null'", "'*'", "(", "(,)", ";"] {
            assert!(
                literal_expression(&tokens(input)).unwrap().is_none(),
                "{input}"
            );
        }
    }

    #[test]
    fn unassigned_object_call_returns_subtype_and_following_assignment_uses_it() {
        // Executor control only; this wrapper is not an imported Pilot rule.
        let mut source: Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-fragment-programs.json"
        ))
        .unwrap();
        source["rules"]["wrapper"] = json!({
            "owner":"https://www.omg.org/spec/SysML/20250201#//Expression",
            "scalar":false,"construct":true,
            "body":{"id":"wrapper","cardinality":"","kind":"sequence","elements":[
                {"id":"open","cardinality":"","kind":"keyword","value":"["},
                {"id":"call","cardinality":"","kind":"call","rule":"org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralBoolean"},
                {"id":"name","cardinality":"","kind":"assign","feature":"declared_name",
                 "feature_id":"https://www.omg.org/spec/SysML/20250201#//Element/declaredName","operator":"=",
                 "terminal":{"id":"name.value","cardinality":"","kind":"name"}},
                {"id":"close","cardinality":"","kind":"keyword","value":"]"}
            ]}
        });
        let programs: Programs = serde_json::from_value(source).unwrap();
        let input = tokens("[false named] tail");
        let executor = Executor {
            programs: &programs,
            tokens: &input,
            kind: "Element",
            kerml: true,
            speculative: false,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        let result = executor
            .rule("wrapper", &Match::default())
            .unwrap()
            .unwrap();
        assert_eq!(result.consumed, 4);
        assert_eq!(result.object_kind.as_deref(), Some("LiteralBoolean"));
        assert_eq!(
            result.fields,
            BTreeMap::from([
                ("value".into(), json!(false)),
                ("declared_name".into(), json!("named"))
            ])
        );
    }

    #[test]
    fn action_expression_bridge_preserves_parentheses_operators_and_spans() {
        use mercurio_foundation::language_contracts::ast::{Expr, LiteralExpr};
        for source in ["null", "()"] {
            let (expression, consumed) =
                crate::parser::parse_expression_prefix(&tokens(source)).unwrap();
            assert!(consumed > 0);
            assert!(matches!(expression, Expr::Tuple { items, .. } if items.is_empty()));
        }
        let (expression, consumed) = crate::parser::parse_expression_prefix(&tokens("*")).unwrap();
        assert_eq!(consumed, 1);
        assert!(
            matches!(expression, Expr::Operation { operator, operands, .. } if operator == "infinity" && operands.is_empty())
        );
        let (expression, _) = crate::parser::parse_expression_prefix(&tokens("(42)")).unwrap();
        assert!(matches!(
            expression,
            Expr::Literal(LiteralExpr::Integer(42))
        ));
        let (expression, _) = crate::parser::parse_expression_prefix(&tokens("2 * 3")).unwrap();
        assert!(matches!(expression, Expr::Binary { .. }));
        let (expression, _) = crate::parser::parse_expression_prefix(&tokens("'null'")).unwrap();
        assert!(matches!(expression, Expr::Name(_)));
        let (_, expression) = literal_expression(&tokens("(\n)")).unwrap().unwrap();
        let Expr::Tuple { span, .. } = expression else {
            panic!("NullExpression");
        };
        assert_eq!(
            (span.start_line, span.start_col, span.end_line, span.end_col),
            (1, 1, 2, 1)
        );
        for kernel in [false, true] {
            for expression in ["null", "()", "*", "(42)", "(1, 2)", "2 * 3"] {
                let construct = if kernel { "feature" } else { "attribute" };
                let source = format!("package P {{ {construct} f = {expression}; }}");
                let result = if kernel {
                    crate::kerml::parse_kerml(&source)
                } else {
                    crate::parse_sysml(&source)
                };
                assert!(result.is_ok(), "{source}: {result:?}");
            }
        }
    }

    #[test]
    fn action_roots_reach_native_expression_models_in_both_languages() {
        let library = crate::load_sysml_baseline().unwrap();
        for kernel in [false, true] {
            let kind = if kernel { "feature" } else { "attribute" };
            let source = format!(
                "package P {{ {kind} empty = null; {kind} parens = (); {kind} unbounded = *; }}"
            );
            let parsed = if kernel {
                crate::kerml::parse_kerml(&source)
            } else {
                crate::parse_sysml(&source)
            }
            .unwrap();
            let document = if kernel {
                crate::kerml::compile_kerml_module(&parsed, "actions.kerml", &library)
            } else {
                crate::compile_sysml_module(&parsed, "actions.sysml", &library)
            }
            .unwrap();
            for (name, expression) in [
                ("empty", json!({"kind":"tuple", "items":[]})),
                ("parens", json!({"kind":"tuple", "items":[]})),
                (
                    "unbounded",
                    json!({"kind":"operation", "operator":"infinity", "operands":[]}),
                ),
            ] {
                let element = document
                    .elements
                    .iter()
                    .find(|e| e.properties.get("declared_name") == Some(&json!(name)))
                    .unwrap();
                assert_eq!(
                    element.properties["expression_ir"], expression,
                    "{name}, KerML={kernel}"
                );
            }
        }
    }

    #[test]
    fn full_literal_dispatch_executes_real_syntax_and_preserves_source_spelling() {
        use mercurio_foundation::language_contracts::ast::{Expr, LiteralExpr};
        let root = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralExpression";
        for (source, kind, expected) in [
            ("true", "LiteralBoolean", Some(json!(true))),
            ("42", "LiteralInteger", Some(json!(42))),
            (r#""word""#, "LiteralString", Some(json!(r#""word""#))),
            ("*", "LiteralInfinity", None),
            ("1.25e2", "LiteralRational", Some(json!(125.0))),
            (".5", "LiteralRational", Some(json!(0.5))),
            ("2E-3", "LiteralRational", Some(json!(0.002))),
            // Numeric components in a datatype rule are text, not EInt values.
            ("2147483648.5", "LiteralRational", Some(json!(2147483648.5))),
        ] {
            let result = execute(root, &tokens(source), "Element").unwrap().unwrap();
            assert_eq!(result.object_kind.as_deref(), Some(kind));
            assert_eq!(result.fields.get("value"), expected.as_ref());
            assert_eq!(result.consumed, 1);
            if kind == "LiteralRational" {
                let (_, expression) = literal_expression(&tokens(source)).unwrap().unwrap();
                assert!(
                    matches!(expression, Expr::Literal(LiteralExpr::Real(value)) if value == source)
                );
            }
        }
        for value in ["1e", "1e+", "1.", ".", "1.2.3", "x"] {
            let mut input = tokens("1");
            input[0].kind = TokenKind::Number(value.into());
            assert!(
                execute(root, &input, "Element").unwrap().is_none(),
                "{value}"
            );
        }
        assert!(
            execute(root, &tokens("1e999"), "Element")
                .unwrap_err()
                .contains("finite EDouble")
        );
        assert!(execute(root, &tokens("null"), "Element").unwrap().is_none());
    }

    #[test]
    fn containment_assignments_preserve_order_instances_and_ecore_inverses() {
        let mut tree = multiplicity_tree("2..2").unwrap();
        assert_eq!(tree.object_kind.as_deref(), Some("MultiplicityRange"));
        let members = tree.children.get_mut("owned_relationship").unwrap();
        assert_eq!(members.len(), 2);
        for member in members.iter() {
            assert_eq!(member.object_kind.as_deref(), Some("OwningMembership"));
            assert_eq!(
                member.container_feature.as_deref(),
                Some("owning_related_element")
            );
            let child = &member.children["owned_related_element"][0];
            assert_eq!(child.object_kind.as_deref(), Some("LiteralInteger"));
            assert_eq!(
                child.container_feature.as_deref(),
                Some("owning_relationship")
            );
            assert_eq!(child.fields["value"], 2);
        }
        let first_member = Arc::make_mut(&mut members[0]);
        let first_child = Arc::make_mut(&mut first_member.children
            .get_mut("owned_related_element").unwrap()[0]);
        first_child.fields.insert("value".into(), json!(3));
        assert_eq!(
            members[1].children["owned_related_element"][0].fields["value"],
            2
        );
        assert!(
            ecore_model::containment_inverse("Element", "owned_relationship", "LiteralInteger")
                .is_err()
        );
        assert!(
            ecore_model::containment_inverse("Relationship", "related_element", "LiteralInteger")
                .is_err()
        );
        for name in ["org.omg.kerml.xtext.KerML", "org.omg.sysml.xtext.SysML"] {
            let member = execute(
                &format!("{name}::MultiplicityExpressionMember"),
                &tokens("*"),
                "Element",
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                member.children["owned_related_element"][0]
                    .object_kind
                    .as_deref(),
                Some("LiteralInfinity")
            );
        }
    }

    #[test]
    fn cross_reference_syntax_is_typed_preserved_and_explicitly_unresolved() {
        for spelling in [
            "count",
            "P::count",
            "$::P::count",
            "P::'limit::name'",
            "'true'",
        ] {
            let tree = multiplicity_tree(&format!("1..{spelling}"))
                .unwrap_or_else(|e| panic!("{spelling}: {e}"));
            let expression =
                &tree.children["owned_relationship"][1].children["owned_related_element"][0];
            assert_eq!(
                expression.object_kind.as_deref(),
                Some("FeatureReferenceExpression")
            );
            let member = &expression.children["owned_relationship"][0];
            assert_eq!(member.object_kind.as_deref(), Some("Membership"));
            let reference = &member.links["member_element"][0];
            assert_eq!(reference.spelling, spelling);
            assert_eq!(
                reference.target_type,
                "https://www.omg.org/spec/SysML/20250201#//Feature"
            );
            assert!(
                !member.fields.contains_key("member_element"),
                "unlinked spelling is not a resolved identity"
            );
        }
        let rule = "org.omg.kerml.xtext.KerML::OwnedMultiplicityRange";
        for text in [
            "[]",
            "[null]",
            "[1..]",
            "[1..2..3]",
            "[P::]",
            "[1+2]",
            "[1. .2]",
            "[;1]",
            "[1..;count]",
            "[1../* gap */.2]",
        ] {
            assert!(
                execute(rule, &tokens(text), "Element").map_or(true, |m| m.is_none()),
                "{text}"
            );
        }
        let input = tokens("[1..]");
        let prior = Match {
            fields: BTreeMap::from([("declared_name".into(), json!("unchanged"))]),
            ..Match::default()
        };
        let saved = prior.clone();
        let executor = Executor {
            programs: programs().unwrap(),
            tokens: &input,
            kind: "MultiplicityRange",
            kerml: true,
            speculative: true,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        assert!(executor.rule(rule, &prior).unwrap().is_none());
        assert_eq!(prior, saved);
    }

    #[test]
    fn generated_containment_tree_reaches_linked_native_kir() {
        let source = "package P { feature count; multiplicity m [1..P::count]; multiplicity repeated [2..2]; }";
        let parsed = crate::kerml::parse_kerml(source).unwrap();
        let library = crate::load_sysml_baseline().unwrap();
        let document = crate::kerml::compile_kerml_module_strict_with_context(
            &parsed,
            "generated-bounds.kerml",
            std::slice::from_ref(&parsed),
            &library,
        )
        .unwrap();
        let count = document
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name") == Some(&json!("count")))
            .unwrap();
        for name in ["m", "repeated"] {
            let range = document
                .elements
                .iter()
                .find(|e| e.properties.get("declared_name") == Some(&json!(name)))
                .unwrap();
            let members: Vec<_> = range.properties["owned_relationship"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| {
                    document
                        .elements
                        .iter()
                        .find(|e| e.id == id.as_str().unwrap())
                        .unwrap()
                })
                .filter(|e| e.kind == "SysML::OwningMembership")
                .collect();
            assert_eq!(members.len(), 2);
            let mut ids = BTreeSet::new();
            for (index, member) in members.into_iter().enumerate() {
                assert_eq!(member.properties["owning_related_element"], range.id);
                let id = member.properties["owned_related_element"][0]
                    .as_str()
                    .unwrap();
                assert!(
                    ids.insert(id),
                    "equal-valued bounds require distinct object identities"
                );
                let expression = document.elements.iter().find(|e| e.id == id).unwrap();
                assert_eq!(expression.properties["owning_relationship"], member.id);
                if name == "m" && index == 1 {
                    assert_eq!(expression.kind, "SysML::FeatureReferenceExpression");
                    let id = expression.properties["owned_relationship"][0]
                        .as_str()
                        .unwrap();
                    let reference = document.elements.iter().find(|e| e.id == id).unwrap();
                    assert_eq!(reference.properties["member_element"], count.id);
                    assert_eq!(
                        reference.properties["owning_related_element"],
                        expression.id
                    );
                } else {
                    assert_eq!(expression.kind, "SysML::LiteralInteger");
                    assert_eq!(
                        expression.properties["value"],
                        if name == "m" { 1 } else { 2 }
                    );
                }
            }
        }
        let invalid = crate::kerml::parse_kerml(&source.replace("P::count", "P::missing")).unwrap();
        assert!(
            crate::kerml::compile_kerml_module_strict_with_context(
                &invalid,
                "missing-bound.kerml",
                std::slice::from_ref(&invalid),
                &library
            )
            .is_err()
        );
    }

    #[test]
    fn literal_terminal_binding_rechecks_the_generated_language() {
        let mut input = tokens("12");
        input[0].kind = TokenKind::Number("12x".into());
        let integer = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralInteger";
        assert!(execute(integer, &input, "Element").unwrap().is_none());
        input[0].kind = TokenKind::String(r"bad\x".into());
        let string = "org.omg.kerml.expressions.xtext.KerMLExpressions::LiteralString";
        assert!(execute(string, &input, "Element").unwrap().is_none());
    }

    #[test]
    fn literal_rules_reach_native_models_through_both_expression_frontends() {
        let sysml = r#"package P { attribute b = false; attribute i = 42; attribute s = "word"; attribute r = 1.25e2; }"#;
        let kerml =
            r#"package P { flow b = false; flow i = 42; flow s = "word"; flow r = 1.25e2; }"#;
        let library = crate::load_sysml_baseline().unwrap();
        for kernel in [false, true] {
            let source = if kernel { kerml } else { sysml };
            let parsed = if kernel {
                crate::kerml::parse_kerml(source)
            } else {
                crate::parse_sysml(source)
            }
            .unwrap();
            let document = if kernel {
                crate::kerml::compile_kerml_module(&parsed, "literal.kerml", &library)
            } else {
                crate::compile_sysml_module(&parsed, "literal.sysml", &library)
            }
            .unwrap();
            for (name, kind, value) in [
                ("b", "LiteralBoolean", json!(false)),
                ("i", "LiteralInteger", json!(42)),
                ("s", "LiteralString", json!("word")),
                ("r", "LiteralRational", json!(125.0)),
            ] {
                let usage = document
                    .elements
                    .iter()
                    .find(|e| e.properties.get("declared_name") == Some(&json!(name)))
                    .unwrap();
                assert_eq!(
                    usage.properties["expression_ir"],
                    json!({"kind":"literal","value":value}),
                    "{name}, KerML={kernel}"
                );
                if kernel {
                    let element = document
                        .elements
                        .iter()
                        .find(|e| {
                            e.kind == format!("SysML::{kind}")
                                && e.properties.get("value") == Some(&value)
                        })
                        .unwrap();
                    ecore_model::validate_value(kind, "value", &element.properties["value"])
                        .unwrap();
                }
            }
        }
        for source in [
            "package P { attribute i = 2147483648; }",
            r#"package P { attribute s = "bad\x"; }"#,
        ] {
            assert!(crate::parse_sysml(source).is_err());
            assert!(crate::kerml::parse_kerml(&source.replace("attribute", "feature")).is_err());
        }
    }

    #[test]
    fn represented_names_reach_production_compiled_models() {
        let library = crate::load_sysml_baseline().unwrap();
        for (spelling, represented) in [
            (r"'vehicle\nname'", "vehicle\nname"),
            (r"'driver\'s'", "driver's"),
        ] {
            let source = format!("package P {{ part def <'short\\tname'> {spelling}; }}");
            let module = crate::parse_sysml(&source).unwrap();
            let document = crate::compile_sysml_module(&module, "name.sysml", &library).unwrap();
            let element = document
                .elements
                .iter()
                .find(|e| e.kind.ends_with("PartDefinition"))
                .unwrap();
            assert_eq!(element.properties["declared_name"], represented);
            assert_eq!(element.properties["declared_short_name"], "short\tname");
            let source = format!("package P {{ class <'short\\tname'> {spelling}; }}");
            let module = crate::kerml::parse_kerml(&source).unwrap();
            let document =
                crate::kerml::compile_kerml_module(&module, "name.kerml", &library).unwrap();
            let element = document
                .elements
                .iter()
                .find(|e| e.kind.ends_with("Class"))
                .unwrap();
            assert_eq!(element.properties["declared_name"], represented);
            assert_eq!(element.properties["declared_short_name"], "short\tname");
        }
    }

    #[test]
    fn fragment_definition_adapter_reaches_native_models_in_both_languages() {
        let sysml = crate::parse_sysml("package P { part def <S> Vehicle; }").unwrap();
        let library = crate::load_sysml_baseline().unwrap();
        let document =
            crate::compile_sysml_module(&sysml, "identification.sysml", &library).unwrap();
        let element = document
            .elements
            .iter()
            .find(|e| {
                e.kind.ends_with("PartDefinition")
                    && e.properties.get("declared_name") == Some(&json!("Vehicle"))
            })
            .unwrap();
        assert_eq!(element.properties["declared_short_name"], "S");
        let kerml = crate::kerml::parse_kerml("package P { class <C> Vehicle; }").unwrap();
        let document =
            crate::kerml::compile_kerml_module(&kerml, "identification.kerml", &library).unwrap();
        let element = document
            .elements
            .iter()
            .find(|e| {
                e.kind.ends_with("Class")
                    && e.properties.get("declared_name") == Some(&json!("Vehicle"))
            })
            .unwrap();
        assert_eq!(element.properties["declared_short_name"], "C");
        for input in ["part def <a b> Vehicle;", "part def <> Vehicle;"] {
            assert!(crate::parse_sysml(input).is_err());
        }
    }
}

/// Preserve QualifiedName groups separated by FeatureChain dots. The returned
/// names are syntax only; endpoint typing and member lookup remain native services.
pub(crate) fn feature_chain_names(
    tokens: &[Token], kerml: bool,
) -> Result<(usize, Vec<mercurio_foundation::language_contracts::ast::QualifiedName>), Diagnostic> {
    let fail = |message: String| Diagnostic::new(message, tokens.first().map(|t| t.span.clone()));
    // Fail on upstream shape drift: this boundary adapter is valid only for
    // OwnedFeatureChaining ('.' OwnedFeatureChaining)+, as imported in the pin.
    let program = programs().map_err(&fail)?.language(kerml).map_err(&fail)?;
    let chain = program.rules.get("org.omg.kerml.expressions.xtext.KerMLExpressions::FeatureChain")
        .ok_or_else(|| fail("missing imported FeatureChain".into()))?;
    let step = |node: &Node| node.cardinality.is_empty() && matches!(&node.operation,
        Operation::Contain { feature, operator, terminal, .. }
            if feature == "owned_relationship" && operator == "+=" && terminal.cardinality.is_empty()
            && matches!(&terminal.operation, Operation::Call { rule } if rule.ends_with("::OwnedFeatureChaining")));
    let supported = matches!(&chain.body.operation, Operation::Sequence { elements }
        if chain.body.cardinality.is_empty() && elements.len() == 2 && step(&elements[0])
        && elements[1].cardinality == "+"
        && matches!(&elements[1].operation, Operation::Sequence { elements: repeated }
            if repeated.len() == 2 && repeated[0].cardinality.is_empty() && step(&repeated[1])
            && matches!(&repeated[0].operation, Operation::Keyword { value } if value == ".")));
    if !supported { return Err(fail("unsupported imported feature-chain boundary shape".into())); }
    // The imported reference-member DFA carries expression-caller FOLLOW sets.
    // Determine this operand's extent using imported QualifiedName recognition,
    // then execute the standalone member rule with an explicit EOF boundary.
    // This is a source-boundary adapter, not a fallback on prediction failure.
    let mut extent = 0;
    loop {
        let name = execute_in_context(
            "org.omg.kerml.expressions.xtext.KerMLExpressions::QualifiedName",
            &tokens[extent..], "Element", kerml,
        ).map_err(&fail)?.ok_or_else(|| fail("expected feature chain name".into()))?;
        if name.consumed == 0 { return Err(fail("non-progressing feature chain name".into())); }
        extent += name.consumed;
        if !matches!(tokens.get(extent).map(|t| &t.kind), Some(TokenKind::Dot)) { break; }
        extent += 1;
    }
    let mut bounded = tokens[..extent].to_vec();
    let mut eof = tokens.get(extent).cloned()
        .ok_or_else(|| fail("missing feature chain boundary".into()))?;
    eof.kind = TokenKind::Eof;
    bounded.push(eof);
    let model = execute_in_context(
        "org.omg.kerml.expressions.xtext.KerMLExpressions::FeatureChainMember", &bounded, "Membership", kerml,
    ).map_err(&fail)?.ok_or_else(|| fail("expected feature reference or chain".into()))?;
    if model.consumed != extent { return Err(fail("incomplete generated feature chain".into())); }
    let references: Vec<&PendingReference> = match model.object_kind.as_deref() {
        Some("Membership") => model.links.get("member_element").into_iter().flatten().collect(),
        Some("OwningMembership") => {
            let owned = model.children.get("owned_related_element")
                .ok_or_else(|| fail("missing generated feature chain".into()))?;
            if owned.len() != 1 || owned[0].object_kind.as_deref() != Some("Feature") {
                return Err(fail("invalid generated feature chain container".into()));
            }
            let mut references = Vec::new();
            for child in owned[0].children.get("owned_relationship").into_iter().flatten() {
                if child.object_kind.as_deref() != Some("FeatureChaining") {
                    return Err(fail("invalid generated feature chain step".into()));
                }
                let links = child.links.get("chaining_feature")
                    .ok_or_else(|| fail("missing feature chain reference".into()))?;
                if links.len() != 1 { return Err(fail("invalid feature chain reference count".into())); }
                references.push(&links[0]);
            }
            if references.len() < 2 { return Err(fail("feature chain needs two steps".into())); }
            references
        }
        _ => return Err(fail("unexpected feature reference model".into())),
    };
    if references.is_empty() { return Err(fail("missing feature reference".into())); }
    let names = references.into_iter().map(|reference| {
        if reference.spelling.starts_with("$::") {
            return Err(fail("global-qualified feature references require a global scope service".into()));
        }
        Ok(mercurio_foundation::language_contracts::ast::QualifiedName {
            segments: reference_name_segments(&reference.spelling, kerml).map_err(&fail)?,
            span: reference.span.as_deref().cloned().ok_or_else(|| fail("missing feature reference span".into()))?,
        })
    }).collect::<Result<Vec<_>, Diagnostic>>()?;
    Ok((model.consumed, names))
}

/// Production declaration-name adapter driven by the imported contextual Name
/// rule. Keyword acceptance and represented-name conversion belong to that rule.
pub(crate) fn declared_name(tokens: &[Token], kerml: bool) -> Result<(usize, String), Diagnostic> {
    let fail = |message: String| Diagnostic::new(message, tokens.first().map(|t| t.span.clone()));
    let matched = execute_in_context(
        "org.omg.kerml.expressions.xtext.KerMLExpressions::Name", tokens, "Element", kerml,
    ).map_err(&fail)?.ok_or_else(|| fail("expected declaration name".into()))?;
    let name = matched.value.as_ref().and_then(Value::as_str)
        .ok_or_else(|| fail("missing generated declaration name".into()))?;
    if matched.consumed == 0 { return Err(fail("declaration name consumed no source".into())); }
    Ok((matched.consumed, name.to_owned()))
}

/// Production reference-syntax adapter. Recognition follows the imported
/// QualifiedName rule; represented-name conversion and AST layout are explicit.
pub(crate) fn qualified_name(
    tokens: &[Token], kerml: bool,
) -> Result<(usize, mercurio_foundation::language_contracts::ast::QualifiedName), Diagnostic> {
    let fail = |message: String| Diagnostic::new(message, tokens.first().map(|t| t.span.clone()));
    let matched = execute_in_context(
        "org.omg.kerml.expressions.xtext.KerMLExpressions::QualifiedName", tokens, "Element", kerml,
    ).map_err(&fail)?.ok_or_else(|| fail("expected qualified name".into()))?;
    let spelling = matched.value.as_ref().and_then(Value::as_str)
        .ok_or_else(|| fail("missing generated qualified name".into()))?;
    let segments = reference_name_segments(spelling, kerml).map_err(&fail)?;
    let first = tokens.first().ok_or_else(|| fail("missing reference source".into()))?;
    let last = matched.consumed.checked_sub(1).and_then(|i| tokens.get(i))
        .ok_or_else(|| fail("qualified name consumed no source".into()))?;
    Ok((matched.consumed, mercurio_foundation::language_contracts::ast::QualifiedName {
        segments,
        span: mercurio_foundation::language_contracts::ast::SourceSpan {
            end_line: last.span.end_line, end_col: last.span.end_col, ..first.span.clone()
        },
    }))
}

/// Shared represented-name decoding for native linking. Name recognition uses
/// the imported contextual rule; separators retain the QualifiedName contract.
pub(crate) fn reference_name_segments(source: &str, kerml: bool) -> Result<Vec<String>, String> {
    // Named handwritten SysMLQualifiedNameConverter dependency: a conjugated
    // qualification appends the conjugated final member beneath its port.
    // Name recognition/decoding still executes the imported Name rule.
    if !kerml {
        if let Some(original) = source.strip_prefix('~') {
            if original.trim_start().starts_with('~') { return Err("Repeated conjugation prefix".into()); }
            let mut names = reference_name_segments(original.trim(), false)?;
            let last = names.last().ok_or("Missing conjugated reference name")?;
            names.push(format!("~{last}"));
            return Ok(names);
        }
    }
    let input = crate::xtext_terminal::lex(source).map_err(|e| format!("{e:?}"))?;
    let mut cursor = 0;
    let mut segments = Vec::new();
    loop {
        let name = execute_in_context(
            "org.omg.kerml.expressions.xtext.KerMLExpressions::Name",
            &input[cursor..],
            "Element",
            kerml,
        )?
        .ok_or("Expected reference name")?;
        if name.consumed == 0 {
            return Err("Non-progressing reference name".into());
        }
        segments.push(
            name.value
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or("Missing represented name")?,
        );
        cursor += name.consumed;
        match input.get(cursor).map(|t| &t.kind) {
            Some(TokenKind::Eof) => return Ok(segments),
            Some(TokenKind::ScopeSep) => cursor += 1,
            _ => return Err("Expected qualified-name separator".into()),
        }
    }
}

pub(crate) fn execute(rule: &str, tokens: &[Token], kind: &str) -> Result<Option<Match>, String> {
    let kerml = rule.starts_with("org.omg.kerml.");
    execute_in_context(rule, tokens, kind, kerml)
}

pub(crate) fn execute_in_context(
    rule: &str,
    tokens: &[Token],
    kind: &str,
    kerml: bool,
) -> Result<Option<Match>, String> {
    let programs = programs()?.language(kerml)?;
    let identity = programs
        .entry_rules
        .get(rule)
        .map(String::as_str)
        .unwrap_or(rule);
    let contract = programs
        .rules
        .get(identity)
        .ok_or_else(|| format!("Missing rule {rule}"))?;
    let mut initial = Match::default();
    if !contract.scalar && !contract.construct {
        // Fragment entry executes inside an already-created caller object.
        // Preserve that classifier even when the entire fragment is optional.
        let owner = contract.owner.rsplit("#//").next().unwrap();
        if !metaclass_conforms(kind, owner) {
            return Err(format!(
                "Fragment caller {kind} does not conform to {owner}"
            ));
        }
        initial.object_kind = Some(kind.to_owned());
    }
    Executor {
        programs,
        tokens,
        kind,
        kerml,
        speculative: false,
        call_stack: &RefCell::new(Vec::new()),
        prediction_cache: &RefCell::new(BTreeMap::new()),
    }
    .rule(rule, &initial)
}

/// Bridge generated Identification assignments into the existing authoring AST.
/// Anonymous/short-name-only definitions still need a different AST identity
/// representation; the executor itself preserves absence of declaredName.
pub(crate) fn definition_identification(
    kerml: bool,
    tokens: &[Token],
) -> Result<(usize, String, Option<String>), Diagnostic> {
    let rule = if kerml {
        "org.omg.kerml.xtext.KerML::Identification"
    } else {
        "org.omg.sysml.xtext.SysML::Identification"
    };
    let matched = execute(rule, tokens, "Element")
        .map_err(|message| Diagnostic::new(message, tokens.first().map(|t| t.span.clone())))?
        .ok_or_else(|| {
            Diagnostic::new(
                "expected definition identification",
                tokens.first().map(|t| t.span.clone()),
            )
        })?;
    let name = matched
        .fields
        .get("declared_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Diagnostic::new(
                "definition requires a declared name in the current authoring AST",
                tokens.first().map(|t| t.span.clone()),
            )
        })?;
    Ok((
        matched.consumed,
        name.into(),
        matched
            .fields
            .get("declared_short_name")
            .and_then(Value::as_str)
            .map(str::to_owned),
    ))
}

/// Explicit bridge from constructed Ecore literals to the existing authoring
/// AST. This is shared by the SysML and KerML expression parsing entry points.
pub(crate) fn literal_expression(
    tokens: &[Token],
) -> Result<Option<(usize, mercurio_foundation::language_contracts::ast::Expr)>, Diagnostic> {
    use mercurio_foundation::language_contracts::ast::{Expr, LiteralExpr};
    let run = || -> Result<Option<(usize, Expr)>, String> {
        let executor = Executor {
            programs: programs()?,
            tokens,
            kind: "Element",
            kerml: true,
            speculative: true,
            call_stack: &RefCell::new(Vec::new()),
            prediction_cache: &RefCell::new(BTreeMap::new()),
        };
        for root in &executor.programs.literal_roots {
            // Root attempts are transactional. In particular, '(' belongs to
            // NullExpression only when its complete '()' alternative matches;
            // '(expression)' remains available to the outer expression parser.
            let Some(mut matched) = executor.rule(root, &Match::default())? else {
                continue;
            };
            if matched.consumed == 0 {
                return Err("Expression root matched without consuming a token".into());
            }
            let mut span = tokens[0].span.clone();
            span.end_line = tokens[matched.consumed - 1].span.end_line;
            span.end_col = tokens[matched.consumed - 1].span.end_col;
            let value = matched.fields.remove("value");
            let expression = match (matched.object_kind.as_deref(), value) {
                (Some("LiteralBoolean"), Some(Value::Bool(value))) => {
                    Expr::Literal(LiteralExpr::Boolean(value))
                }
                (Some("LiteralInteger"), Some(Value::Number(value))) => Expr::Literal(
                    LiteralExpr::Integer(value.as_i64().ok_or("Invalid EInt AST value")?),
                ),
                (Some("LiteralString"), Some(Value::String(value))) => {
                    // The pinned parser stores the terminal spelling, including
                    // quotes. The authoring AST uses its interior; escape
                    // decoding remains a separately unqualified converter.
                    let interior = value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .ok_or("Invalid raw STRING_VALUE spelling")?;
                    Expr::Literal(LiteralExpr::String(interior.into()))
                }
                (Some("LiteralRational"), Some(Value::Number(_))) => {
                    Expr::Literal(LiteralExpr::Real(
                        matched
                            .lexeme
                            .take()
                            .ok_or("Missing real source spelling")?,
                    ))
                }
                (Some("LiteralInfinity"), None) => Expr::Operation {
                    operator: "infinity".into(),
                    operands: Vec::new(),
                    span,
                },
                (Some("NullExpression"), None) => Expr::Tuple {
                    items: Vec::new(),
                    span,
                },
                _ => return Err("Unsupported constructed literal AST bridge".into()),
            };
            return Ok(Some((matched.consumed, expression)));
        }
        Ok(None)
    };
    run().map_err(|message| Diagnostic::new(message, tokens.first().map(|t| t.span.clone())))
}

/// Construct the complete pinned multiplicity syntax tree. The caller performs
/// Natural-value validation and resolves deferred references before KIR emission.
pub(crate) fn multiplicity_tree(raw: &str) -> Result<Match, String> {
    let input = crate::xtext_terminal::lex(&format!("[{raw}]")).map_err(|e| format!("{e:?}"))?;
    let matched = execute(
        "org.omg.kerml.xtext.KerML::OwnedMultiplicityRange",
        &input,
        "MultiplicityRange",
    )?
    .ok_or("Expected MultiplicityBounds")?;
    if matched.consumed + 1 != input.len() {
        return Err("Trailing tokens after MultiplicityBounds".into());
    }
    Ok(matched)
}

#[cfg(test)]
#[path = "xtext_fragment_sharing_tests.rs"]
mod sharing_tests;
