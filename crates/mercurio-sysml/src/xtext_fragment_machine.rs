//! Heap-backed model evaluation. Syntax-only prediction retains its independent
//! bounded frame machine; no model rule invokes another model rule on the native stack.
use super::*;

pub(super) const MAX_RULE_FRAMES: usize = 256;
const BASE_STEPS: usize = 100_000;
const STEPS_PER_TOKEN: usize = 128;
pub(super) const MAX_STEPS: usize = 4_000_000;

// Credit only new input progress. Backtracking cannot repeatedly earn work;
// input padding cannot increase the allowance before it is actually consumed.
pub(super) struct WorkBudget {
    steps: usize,
    furthest: usize,
    start: usize,
    ceiling: usize,
}

impl WorkBudget {
    pub(super) fn new(start: usize, ceiling: usize) -> Self {
        Self { steps: 0, furthest: start, start, ceiling }
    }

    pub(super) fn charge(&mut self, consumed: usize) -> Result<(), String> {
        self.furthest = self.furthest.max(consumed);
        self.steps += 1;
        let limit = BASE_STEPS.saturating_add(
            self.furthest.saturating_sub(self.start).saturating_mul(STEPS_PER_TOKEN)
        ).min(self.ceiling);
        if self.steps > limit {
            return Err(format!(
                "Xtext execution step budget exceeded: {} steps, {} new tokens, limit {}",
                self.steps, self.furthest - self.start, limit
            ));
        }
        Ok(())
    }
}

enum Frame<'n> {
    Rule(&'n str, Match, String),
    RuleDone(&'n Rule, &'n str),
    Node(&'n Node, Match, String),
    Body(&'n Node, Match, String),
    Operation(&'n Node, Match, String),
    Optional(Match),
    Repeat(&'n Node, Match, String, usize),
    Sequence(&'n [Node], usize, usize, String),
    Choice(&'n [Node], usize, Match, String),
    Unary(&'n Node, Match, String),
    Committed(&'static str),
}

pub(super) fn run(
    executor: &Executor<'_>,
    identity: &str,
    state: &Match,
) -> Result<Option<Match>, String> {
    run_bounded(executor, identity, state, MAX_STEPS)
}

fn run_bounded(
    executor: &Executor<'_>, identity: &str, state: &Match, ceiling: usize,
) -> Result<Option<Match>, String> {
    let initial_depth = executor.call_stack.borrow().len();
    let result = evaluate(executor, identity, state, ceiling);
    // Error exits must release every active rule, including committed failures.
    executor.call_stack.borrow_mut().truncate(initial_depth);
    result
}

fn evaluate(
    executor: &Executor<'_>,
    identity: &str,
    state: &Match,
    ceiling: usize,
) -> Result<Option<Match>, String> {
    let mut frames = vec![Frame::Rule(identity, state.clone(), executor.kind.into())];
    let mut result: Option<Match> = None;
    let mut budget = WorkBudget::new(state.consumed, ceiling);
    while let Some(frame) = frames.pop() {
        budget.charge(result.as_ref().map_or(state.consumed, |value| value.consumed))?;
        match frame {
            Frame::Rule(identity, state, kind) => {
                let identity = executor
                    .programs
                    .entry_rules
                    .get(identity)
                    .map(String::as_str)
                    .unwrap_or(identity);
                let rule = executor
                    .programs
                    .rules
                    .get(identity)
                    .ok_or_else(|| format!("Unsupported Xtext fragment {identity}"))?;
                {
                    let mut active = executor.call_stack.borrow_mut();
                    if active
                        .iter()
                        .any(|(name, offset)| name == identity && *offset == state.consumed)
                    {
                        return Err(format!("Non-consuming recursive Xtext call {identity}"));
                    }
                    if active.len() >= MAX_RULE_FRAMES {
                        return Err("Xtext model rule frame budget exceeded (256)".into());
                    }
                    active.push((identity.into(), state.consumed));
                }
                frames.push(Frame::RuleDone(rule, identity));
                if rule.construct {
                    frames.push(Frame::Node(
                        &rule.body,
                        Match {
                            consumed: state.consumed,
                            ..Match::default()
                        },
                        rule.owner.rsplit('/').next().unwrap().into(),
                    ));
                } else {
                    if !rule.scalar
                        && !metaclass_conforms(
                            state.object_kind.as_deref().unwrap_or(&kind),
                            rule.owner.rsplit('/').next().unwrap(),
                        )
                    {
                        return Err(format!("Xtext fragment {identity} cannot augment {kind}"));
                    }
                    frames.push(Frame::Node(&rule.body, state, kind));
                }
            }
            Frame::RuleDone(rule, identity) => {
                executor.call_stack.borrow_mut().pop();
                result = executor.finish_rule_value(rule, identity, result)?;
            }
            Frame::Node(node, state, kind) => {
                match node.cardinality.as_str() {
                    "" => (),
                    "?" => frames.push(Frame::Optional(state.clone())),
                    "*" | "+" => frames.push(Frame::Repeat(node, state.clone(), kind.clone(), 0)),
                    other => return Err(format!("Unsupported Xtext cardinality {other}")),
                }
                frames.push(Frame::Body(node, state, kind));
            }
            Frame::Optional(state) => {
                if result.is_none() {
                    result = Some(state);
                }
            }
            Frame::Repeat(node, current, kind, count) => {
                if let Some(next) = result.take() {
                    if next.consumed <= current.consumed {
                        return Err(format!("Non-progressing Xtext repetition {}", node.id));
                    }
                    frames.push(Frame::Repeat(node, next.clone(), kind.clone(), count + 1));
                    frames.push(Frame::Body(node, next, kind));
                } else {
                    result = (count > 0 || node.cardinality == "*").then_some(current);
                }
            }
            Frame::Body(node, state, kind) => {
                ensure_prediction_supported(node)?;
                if !matches!(node.operation, Operation::Choice { .. }) {
                    if let Some(decision) = &node.decision {
                        match executor.decide(decision, state.consumed)? {
                            None => {
                                result = None;
                                continue;
                            }
                            Some(0) => {
                                frames.push(Frame::Committed("Committed entry prediction failed"))
                            }
                            Some(_) => return Err("Invalid entry prediction alternative".into()),
                        }
                    }
                }
                if !node.prediction.is_empty()
                    && !executor.predicts(&node.prediction, state.consumed)
                {
                    result = None;
                    continue;
                }
                if node.predicated || node.first_set_predicated {
                    let mut probe = SyntaxProbe {
                        executor,
                        active: Vec::new(),
                        memo: BTreeMap::new(),
                    };
                    let matches = if node.first_set_predicated {
                        probe.first(node, state.consumed)?.0
                    } else {
                        !probe.body(node, state.consumed)?.is_empty()
                    };
                    if !matches {
                        result = None;
                        continue;
                    }
                }
                frames.push(Frame::Operation(node, state, kind));
            }
            Frame::Operation(node, mut state, kind) => {
                let local = Executor {
                    programs: executor.programs,
                    tokens: executor.tokens,
                    kind: &kind,
                    kerml: executor.kerml,
                    speculative: executor.speculative,
                    call_stack: executor.call_stack,
                    prediction_cache: executor.prediction_cache,
                };
                match &node.operation {
                    Operation::Call { rule } => {
                        let identity = executor.programs.entry_rules.get(rule).unwrap_or(rule);
                        let called = executor
                            .programs
                            .rules
                            .get(identity)
                            .ok_or("Unknown called rule")?;
                        if !called.construct && !called.scalar && state.object_kind.is_none() {
                            state.object_kind = Some(kind.clone());
                        }
                        frames.push(Frame::Rule(rule, state, kind));
                    }
                    Operation::Sequence { elements } => {
                        let start = state.consumed;
                        result = Some(state);
                        frames.push(Frame::Sequence(elements, 0, start, kind));
                    }
                    Operation::Choice { elements } => {
                        if let Some(decision) = &node.decision {
                            match executor.decide(decision, state.consumed)? {
                                None => result = None,
                                Some(index) => {
                                    let selected = elements
                                        .get(index)
                                        .ok_or("Invalid prediction alternative")?;
                                    frames.push(Frame::Committed(
                                        "Committed prediction branch failed",
                                    ));
                                    frames.push(Frame::Node(selected, state, kind));
                                }
                            }
                        } else {
                            result = None;
                            frames.push(Frame::Choice(elements, 0, state, kind));
                        }
                    }
                    Operation::Contain { terminal, .. }
                    | Operation::AppendOperand { terminal, .. } => {
                        let input = Match {
                            consumed: state.consumed,
                            ..Match::default()
                        };
                        frames.push(Frame::Unary(node, state, kind.clone()));
                        frames.push(Frame::Node(terminal, input, kind));
                    }
                    Operation::Assign { terminal, .. }
                    | Operation::Link { terminal, .. }
                    | Operation::CrossReference { terminal, .. } => {
                        let mut input = state.clone();
                        if matches!(node.operation, Operation::Assign { .. }) {
                            input.value = None;
                        }
                        if matches!(node.operation, Operation::Link { .. }) {
                            input.reference = None;
                        }
                        frames.push(Frame::Unary(node, state, kind.clone()));
                        frames.push(Frame::Node(terminal, input, kind));
                    }
                    // All remaining operations are atomic model/lexical services.
                    Operation::TokenDatatype { .. } => {
                        result = local.body_token_datatype(node, &state)?
                    }
                    Operation::Datatype { .. } => result = local.body_datatype(node, &state)?,
                    Operation::Capture { .. } => result = local.body_capture(node, &state)?,
                    Operation::Create { .. } => result = local.body_create(node, &state)?,
                    Operation::Keyword { .. } => result = local.body_keyword(node, &state)?,
                    Operation::Enum { .. } => result = local.body_enum(node, &state)?,
                    Operation::Name => result = local.body_name(node, &state)?,
                    Operation::Terminal { .. } => result = local.body_terminal(node, &state)?,
                }
            }
            Frame::Unary(node, state, kind) => {
                let local = Executor {
                    programs: executor.programs,
                    tokens: executor.tokens,
                    kind: &kind,
                    kerml: executor.kerml,
                    speculative: executor.speculative,
                    call_stack: executor.call_stack,
                    prediction_cache: executor.prediction_cache,
                };
                result = match node.operation {
                    Operation::Contain { .. } => local.finish_contain(node, &state, result)?,
                    Operation::AppendOperand { .. } => {
                        local.finish_append_operand(node, &state, result)?
                    }
                    Operation::Assign { .. } => local.finish_assign(node, &state, result)?,
                    Operation::Link { .. } => local.finish_link(node, &state, result)?,
                    Operation::CrossReference { .. } => {
                        local.finish_cross_reference(node, &state, result)?
                    }
                    _ => unreachable!(),
                };
            }
            Frame::Sequence(elements, index, start, kind) => {
                if let Some(current) = result.take() {
                    if let Some(child) = elements.get(index) {
                        frames.push(Frame::Sequence(elements, index + 1, start, kind.clone()));
                        // Preserve commitment based on progress before this child.
                        if !executor.speculative && current.consumed > start {
                            frames.push(Frame::Committed("Incomplete Xtext group"));
                        }
                        frames.push(Frame::Node(child, current, kind));
                    } else {
                        result = Some(current);
                    }
                }
            }
            Frame::Choice(elements, index, state, kind) => {
                if result.is_none() {
                    if let Some(child) = elements.get(index) {
                        frames.push(Frame::Choice(
                            elements,
                            index + 1,
                            state.clone(),
                            kind.clone(),
                        ));
                        frames.push(Frame::Node(child, state, kind));
                    }
                }
            }
            Frame::Committed(message) => {
                if result.is_none() {
                    return Err(message.into());
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod work_tests {
    use super::*;

    #[test]
    fn work_credit_requires_new_progress_and_has_an_absolute_ceiling() {
        let mut budget = WorkBudget::new(7, MAX_STEPS);
        for _ in 0..BASE_STEPS { budget.charge(7).unwrap(); }
        budget.charge(8).unwrap();
        for _ in 1..STEPS_PER_TOKEN { budget.charge(7).unwrap(); }
        assert!(budget.charge(8).unwrap_err().contains("1 new tokens"));
        let mut capped = WorkBudget::new(0, 10);
        for _ in 0..10 { capped.charge(usize::MAX).unwrap(); }
        assert!(capped.charge(usize::MAX).unwrap_err().contains("limit 10"));
    }

    #[test]
    fn work_limit_unwinds_without_publishing_partial_state() {
        let program: Programs = serde_json::from_value(serde_json::json!({"rules":{
            "wide":{"owner":"https://www.omg.org/spec/SysML/20250201#//Feature",
                "scalar":false,"construct":true,"body":{"id":"wide/body",
                "kind":"sequence","cardinality":"","elements":(0..200).map(|i|
                    serde_json::json!({"id":format!("wide/{i}"),"kind":"keyword",
                        "value":"x","cardinality":"?"})).collect::<Vec<_>>()}}
        }})).unwrap();
        let tokens = crate::xtext_terminal::lex("y").unwrap();
        let stack = RefCell::new(Vec::new());
        let cache = RefCell::new(BTreeMap::new());
        let executor = Executor { programs: &program, tokens: &tokens, kind: "Feature",
            kerml: true, speculative: false, call_stack: &stack, prediction_cache: &cache };
        let initial = Match::default();
        let error = run_bounded(&executor, "wide", &initial, 100).unwrap_err();
        assert!(error.contains("0 new tokens"));
        assert!(stack.borrow().is_empty());
        assert_eq!(initial, Match::default());
    }
}
