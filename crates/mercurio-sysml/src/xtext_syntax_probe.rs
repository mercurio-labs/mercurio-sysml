//! Syntax-only Xtext evaluation on heap frames. No Ecore objects are constructed.
//! Imported choices, commitment and repetition retain their original semantics.
//! Rule/frame and progress-based work budgets bound input-controlled recursion.
use super::*;

enum Frame<'n> {
    Node(&'n Node, usize),
    Body(&'n Node, usize),
    RuleDone((String, usize)),
    Optional(&'n Node, usize),
    Repeat(&'n Node, usize, usize),
    MissingFirst(&'n Node, usize, Option<usize>),
    Sequence(&'n [Node], usize),
    Union(&'n [Node], usize, std::collections::btree_set::IntoIter<usize>, BTreeSet<usize>),
    Choice(&'n [Node], usize, usize),
    MissingFirstChoice(&'n [Node], usize, usize),
    First(&'n Node, usize),
    FirstNullable(bool),
    FirstRuleDone,
    FirstSequence(&'n [Node], usize, usize, bool, bool),
    FirstChoice(&'n [Node], usize, usize, bool, bool),
    FirstFromBody(usize),
}

impl SyntaxProbe<'_, '_> {
    pub(super) fn node(&mut self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        self.run(Frame::Node(node, offset), offset, machine::MAX_STEPS).map(|v| v.0)
    }

    pub(super) fn body(&mut self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        self.run(Frame::Body(node, offset), offset, machine::MAX_STEPS).map(|v| v.0)
    }

    pub(super) fn first(&mut self, node: &Node, offset: usize) -> Result<(bool, bool), String> {
        self.run(Frame::First(node, offset), offset, machine::MAX_STEPS).map(|v| v.1)
    }

    fn run(
        &mut self, frame: Frame<'_>, start: usize, ceiling: usize,
    ) -> Result<(BTreeSet<usize>, (bool, bool)), String> {
        let depth = self.active.len();
        let result = self.evaluate(frame, start, ceiling);
        // A failed probe must release guards but cannot publish any model state.
        self.active.truncate(depth);
        result
    }

    fn enter(&mut self, key: (String, usize)) -> Result<(), String> {
        if self.active.contains(&key) {
            return Err("Non-consuming predicate recursion".into());
        }
        if self.active.len() >= machine::MAX_RULE_FRAMES {
            return Err("Xtext predicate rule frame budget exceeded (256)".into());
        }
        self.active.push(key);
        Ok(())
    }

    fn missing(&self, node: &Node, offset: usize, repetitions: Option<usize>) -> BTreeSet<usize> {
        if !node.prediction.is_empty() && self.executor.predicts(&node.prediction, offset) {
            return BTreeSet::new();
        }
        if repetitions.is_none() || repetitions.is_some_and(|count| count > 0) || node.cardinality == "*" {
            BTreeSet::from([offset])
        } else {
            BTreeSet::new()
        }
    }

    fn evaluate(
        &mut self, frame: Frame<'_>, start: usize, ceiling: usize,
    ) -> Result<(BTreeSet<usize>, (bool, bool)), String> {
        let mut frames = vec![frame];
        let mut result = BTreeSet::new();
        let mut first = (false, false);
        let mut budget = machine::WorkBudget::new(start, ceiling);
        while let Some(frame) = frames.pop() {
            budget.charge(result.last().copied().unwrap_or(start))?;
            match frame {
                Frame::Node(node, offset) => {
                    match node.cardinality.as_str() {
                        "" => (),
                        "?" => frames.push(Frame::Optional(node, offset)),
                        "*" | "+" => frames.push(Frame::Repeat(node, offset, 0)),
                        _ => return Err("Unsupported predicate cardinality".into()),
                    }
                    frames.push(Frame::Body(node, offset));
                }
                Frame::Optional(node, offset) if result.is_empty() => {
                    if node.decision.as_ref().map(|d| self.executor.decide(d, offset)).transpose()?.flatten().is_some() {
                        continue; // Committed syntax failed.
                    }
                    if node.first_set_predicated {
                        frames.push(Frame::MissingFirst(node, offset, None));
                        frames.push(Frame::First(node, offset));
                    } else {
                        result = self.missing(node, offset, None);
                    }
                }
                Frame::Optional(_, _) => (),
                Frame::Repeat(node, offset, count) => {
                    if let Some(end) = result.last().copied() {
                        if end <= offset {
                            return Err("Non-progressing predicate repetition".into());
                        }
                        frames.push(Frame::Repeat(node, end, count + 1));
                        frames.push(Frame::Body(node, end));
                    } else if node.decision.as_ref().map(|d| self.executor.decide(d, offset)).transpose()?.flatten().is_none() {
                        if node.first_set_predicated {
                            frames.push(Frame::MissingFirst(node, offset, Some(count)));
                            frames.push(Frame::First(node, offset));
                        } else {
                            result = self.missing(node, offset, Some(count));
                        }
                    }
                }
                Frame::MissingFirst(node, offset, count) => {
                    result = if first.0 { BTreeSet::new() } else { self.missing(node, offset, count) };
                }
                Frame::Body(node, offset) => {
                    ensure_prediction_supported(node)?;
                    result.clear();
                    if !matches!(node.operation, Operation::Choice { .. }) {
                        if let Some(decision) = &node.decision {
                            match self.executor.decide(decision, offset)? {
                                None => continue,
                                Some(0) => (),
                                Some(_) => return Err("Invalid entry prediction alternative".into()),
                            }
                        }
                    }
                    if !node.prediction.is_empty() && !self.executor.predicts(&node.prediction, offset) {
                        continue;
                    }
                    let input = Match { consumed: offset, ..Match::default() };
                    let matched = match &node.operation {
                        Operation::Keyword { value } => self.executor.literal(value, &input),
                        Operation::Enum { token, .. } => self.executor.literal(token, &input),
                        Operation::Name => self.executor.body_name(node, &input)?,
                        Operation::Terminal { .. } => self.executor.body_terminal(node, &input)?,
                        Operation::Datatype { .. } => self.executor.body_datatype(node, &input)?,
                        Operation::TokenDatatype { syntax } => {
                            result = self.executor.token_ends(syntax, offset)?.into_iter().max().into_iter().collect();
                            continue;
                        }
                        Operation::Create { .. } | Operation::Capture { .. } => {
                            result.insert(offset);
                            continue;
                        }
                        Operation::Assign { terminal, .. }
                        | Operation::Contain { terminal, .. }
                        | Operation::AppendOperand { terminal, .. }
                        | Operation::Link { terminal, .. }
                        | Operation::CrossReference { terminal, .. } => {
                            frames.push(Frame::Node(terminal, offset));
                            continue;
                        }
                        Operation::Call { rule } => {
                            let key = (rule.clone(), offset);
                            if let Some(ends) = self.memo.get(&key) {
                                result = ends.clone();
                                continue;
                            }
                            let called = self.executor.programs.rules.get(rule).ok_or("Unknown predicate rule")?;
                            self.enter(key.clone())?;
                            frames.push(Frame::RuleDone(key));
                            frames.push(Frame::Node(&called.body, offset));
                            continue;
                        }
                        Operation::Sequence { elements } => {
                            result.insert(offset);
                            frames.push(Frame::Sequence(elements, 0));
                            continue;
                        }
                        Operation::Choice { elements } => {
                            if let Some(decision) = &node.decision {
                                let Some(index) = self.executor.decide(decision, offset)? else { continue; };
                                let selected = elements.get(index).ok_or("Invalid prediction alternative")?;
                                frames.push(Frame::Node(selected, offset));
                            } else {
                                frames.push(Frame::Choice(elements, 0, offset));
                            }
                            continue;
                        }
                    };
                    result.extend(matched.map(|m| m.consumed));
                }
                Frame::RuleDone(key) => {
                    self.active.pop();
                    self.memo.insert(key, result.clone());
                }
                Frame::Sequence(elements, index) => {
                    if let Some(child) = elements.get(index) {
                        let mut positions = std::mem::take(&mut result).into_iter();
                        if let Some(offset) = positions.next() {
                            frames.push(Frame::Union(elements, index, positions, BTreeSet::new()));
                            frames.push(Frame::Node(child, offset));
                        }
                    }
                }
                Frame::Union(elements, index, mut positions, mut next) => {
                    next.extend(std::mem::take(&mut result));
                    if let Some(offset) = positions.next() {
                        let child = elements.get(index).ok_or("Invalid predicate sequence continuation")?;
                        frames.push(Frame::Union(elements, index, positions, next));
                        frames.push(Frame::Node(child, offset));
                    } else {
                        result = next;
                        if !result.is_empty() {
                            frames.push(Frame::Sequence(elements, index + 1));
                        }
                    }
                }
                Frame::Choice(elements, index, offset) if result.is_empty() => {
                    if index > 0 {
                        let previous = elements.get(index - 1).ok_or("Invalid predicate choice continuation")?;
                        if previous.first_set_predicated {
                            frames.push(Frame::MissingFirstChoice(elements, index, offset));
                            frames.push(Frame::First(previous, offset));
                            continue;
                        }
                        if !previous.prediction.is_empty() && self.executor.predicts(&previous.prediction, offset) {
                            continue;
                        }
                    }
                    if let Some(child) = elements.get(index) {
                        frames.push(Frame::Choice(elements, index + 1, offset));
                        frames.push(Frame::Node(child, offset));
                    }
                }
                Frame::Choice(_, _, _) => (),
                Frame::MissingFirstChoice(elements, index, offset) => {
                    result.clear();
                    let previous = elements.get(index - 1).ok_or("Invalid predicate first-set continuation")?;
                    if !first.0 && (previous.prediction.is_empty() || !self.executor.predicts(&previous.prediction, offset)) {
                        if let Some(child) = elements.get(index) {
                            frames.push(Frame::Choice(elements, index + 1, offset));
                            frames.push(Frame::Node(child, offset));
                        }
                    }
                }
                Frame::First(node, offset) => {
                    ensure_prediction_supported(node)?;
                    frames.push(Frame::FirstNullable(matches!(node.cardinality.as_str(), "?" | "*")));
                    match &node.operation {
                        Operation::Create { .. } | Operation::Capture { .. } => first = (false, true),
                        Operation::Assign { terminal, .. }
                        | Operation::Contain { terminal, .. }
                        | Operation::AppendOperand { terminal, .. }
                        | Operation::Link { terminal, .. }
                        | Operation::CrossReference { terminal, .. } => frames.push(Frame::First(terminal, offset)),
                        Operation::TokenDatatype { syntax } => frames.push(Frame::First(syntax, offset)),
                        Operation::Call { rule } => {
                            let called = self.executor.programs.rules.get(rule).ok_or("Unknown first-set rule")?;
                            self.enter((rule.clone(), offset))?;
                            frames.push(Frame::FirstRuleDone);
                            frames.push(Frame::First(&called.body, offset));
                        }
                        Operation::Sequence { elements } => {
                            first = (false, true);
                            frames.push(Frame::FirstSequence(elements, 0, offset, false, true));
                        }
                        Operation::Choice { elements } => {
                            first = (false, false);
                            frames.push(Frame::FirstChoice(elements, 0, offset, false, false));
                        }
                        _ => {
                            frames.push(Frame::FirstFromBody(offset));
                            frames.push(Frame::Body(node, offset));
                        }
                    }
                }
                Frame::FirstNullable(nullable) => first.1 |= nullable,
                Frame::FirstRuleDone => { self.active.pop(); }
                Frame::FirstFromBody(offset) => first = (result.iter().any(|end| *end > offset), result.contains(&offset)),
                Frame::FirstSequence(elements, index, offset, found, nullable) => {
                    let combined = (found || first.0, nullable && first.1);
                    if combined.1 {
                        if let Some(child) = elements.get(index) {
                            frames.push(Frame::FirstSequence(elements, index + 1, offset, combined.0, combined.1));
                            frames.push(Frame::First(child, offset));
                            continue;
                        }
                    }
                    first = combined;
                }
                Frame::FirstChoice(elements, index, offset, found, nullable) => {
                    let combined = (found || first.0, nullable || first.1);
                    if let Some(child) = elements.get(index) {
                        frames.push(Frame::FirstChoice(elements, index + 1, offset, combined.0, combined.1));
                        frames.push(Frame::First(child, offset));
                    } else {
                        first = combined;
                    }
                }
            }
        }
        Ok((result, first))
    }
}

// Prior bounded evaluator is retained only for shallow differential controls.
#[cfg(test)]
impl SyntaxProbe<'_, '_> {
    fn legacy_node(&mut self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        match node.cardinality.as_str() {
            "" => self.legacy_body(node, offset),
            "?" => {
                let ends = self.legacy_body(node, offset)?;
                if !ends.is_empty() {
                    return Ok(ends);
                }
                if (node
                    .decision
                    .as_ref()
                    .map(|d| self.executor.decide(d, offset))
                    .transpose()?
                    .flatten()
                    .is_some())
                    || (node.first_set_predicated && self.legacy_first(node, offset)?.0)
                    || (!node.prediction.is_empty()
                        && self.executor.predicts(&node.prediction, offset))
                {
                    return Ok(BTreeSet::new()); // committed syntax failed
                }
                Ok(BTreeSet::from([offset]))
            }
            "*" | "+" => {
                let (mut current, mut count) = (offset, 0);
                loop {
                    let Some(end) = self.legacy_body(node, current)?.into_iter().max() else {
                        if (node
                            .decision
                            .as_ref()
                            .map(|d| self.executor.decide(d, current))
                            .transpose()?
                            .flatten()
                            .is_some())
                            || (node.first_set_predicated && self.legacy_first(node, current)?.0)
                            || (!node.prediction.is_empty()
                                && self.executor.predicts(&node.prediction, current))
                        {
                            return Ok(BTreeSet::new());
                        }
                        break;
                    };
                    if end <= current {
                        return Err("Non-progressing predicate repetition".into());
                    }
                    current = end;
                    count += 1;
                }
                Ok(if count > 0 || node.cardinality == "*" {
                    BTreeSet::from([current])
                } else {
                    BTreeSet::new()
                })
            }
            _ => Err("Unsupported predicate cardinality".into()),
        }
    }

    fn legacy_first(&mut self, node: &Node, offset: usize) -> Result<(bool, bool), String> {
        ensure_prediction_supported(node)?;
        let (matches, mut nullable) = match &node.operation {
            Operation::Create { .. } | Operation::Capture { .. } => (false, true),
            Operation::Assign { terminal, .. }
            | Operation::Contain { terminal, .. }
            | Operation::AppendOperand { terminal, .. }
            | Operation::Link { terminal, .. }
            | Operation::CrossReference { terminal, .. } => self.legacy_first(terminal, offset)?,
            Operation::TokenDatatype { syntax } => self.legacy_first(syntax, offset)?,
            Operation::Call { rule } => {
                let key = (rule.clone(), offset);
                if self.active.contains(&key) || self.active.len() >= 64 {
                    return Err("Non-consuming or excessive first-set recursion".into());
                }
                let called = self
                    .executor
                    .programs
                    .rules
                    .get(rule)
                    .ok_or("Unknown first-set rule")?;
                self.active.push(key);
                let result = self.legacy_first(&called.body, offset);
                self.active.pop();
                result?
            }
            Operation::Sequence { elements } => {
                let (mut matches, mut nullable) = (false, true);
                for child in elements {
                    let (found, empty) = self.legacy_first(child, offset)?;
                    matches |= found;
                    nullable &= empty;
                    if !empty {
                        break;
                    }
                }
                (matches, nullable)
            }
            Operation::Choice { elements } => {
                let (mut matches, mut nullable) = (false, false);
                for child in elements {
                    let (found, empty) = self.legacy_first(child, offset)?;
                    matches |= found;
                    nullable |= empty;
                }
                (matches, nullable)
            }
            _ => {
                let ends = self.legacy_body(node, offset)?;
                (ends.iter().any(|end| *end > offset), ends.contains(&offset))
            }
        };
        nullable |= matches!(node.cardinality.as_str(), "?" | "*");
        Ok((matches, nullable))
    }

    fn legacy_body(&mut self, node: &Node, offset: usize) -> Result<BTreeSet<usize>, String> {
        ensure_prediction_supported(node)?;
        if !matches!(node.operation, Operation::Choice { .. }) {
            if let Some(decision) = &node.decision {
                match self.executor.decide(decision, offset)? {
                    None => return Ok(BTreeSet::new()),
                    Some(0) => {}
                    Some(_) => return Err("Invalid entry prediction alternative".into()),
                }
            }
        }
        if !node.prediction.is_empty() && !self.executor.predicts(&node.prediction, offset) {
            return Ok(BTreeSet::new());
        }
        let input = Match {
            consumed: offset,
            ..Match::default()
        };
        let matched = match &node.operation {
            Operation::Keyword { value } => self.executor.literal(value, &input),
            Operation::Enum { token, .. } => self.executor.literal(token, &input),
            Operation::Name => self.executor.body_name(node, &input)?,
            Operation::Terminal { .. } => self.executor.body_terminal(node, &input)?,
            Operation::Datatype { .. } => self.executor.body_datatype(node, &input)?,
            Operation::TokenDatatype { syntax } => {
                return Ok(self
                    .executor
                    .token_ends(syntax, offset)?
                    .into_iter()
                    .max()
                    .into_iter()
                    .collect());
            }
            Operation::Create { .. } | Operation::Capture { .. } => {
                return Ok(BTreeSet::from([offset]));
            }
            Operation::Assign { terminal, .. }
            | Operation::Contain { terminal, .. }
            | Operation::AppendOperand { terminal, .. }
            | Operation::Link { terminal, .. }
            | Operation::CrossReference { terminal, .. } => return self.legacy_node(terminal, offset),
            Operation::Call { rule } => {
                let key = (rule.clone(), offset);
                if let Some(ends) = self.memo.get(&key) {
                    return Ok(ends.clone());
                }
                if self.active.contains(&key) {
                    return Err("Non-consuming predicate recursion".into());
                }
                if self.active.len() >= 64 {
                    return Err("Predicate nesting exceeds supported depth of 64".into());
                }
                let called = self
                    .executor
                    .programs
                    .rules
                    .get(rule)
                    .ok_or("Unknown predicate rule")?;
                self.active.push(key.clone());
                let result = self.legacy_node(&called.body, offset);
                self.active.pop();
                let ends = result?;
                self.memo.insert(key, ends.clone());
                return Ok(ends);
            }
            Operation::Sequence { elements } => {
                let mut positions = BTreeSet::from([offset]);
                for child in elements {
                    let mut next = BTreeSet::new();
                    for position in positions {
                        next.extend(self.legacy_node(child, position)?);
                    }
                    positions = next;
                    if positions.is_empty() {
                        break;
                    }
                }
                return Ok(positions);
            }
            Operation::Choice { elements } => {
                if let Some(decision) = &node.decision {
                    let Some(alternative) = self.executor.decide(decision, offset)? else {
                        return Ok(BTreeSet::new());
                    };
                    return self.legacy_node(
                        elements
                            .get(alternative)
                            .ok_or("Invalid prediction alternative")?,
                        offset,
                    );
                }
                for child in elements {
                    let ends = self.legacy_node(child, offset)?;
                    if !ends.is_empty() {
                        return Ok(ends);
                    }
                    if (child.first_set_predicated && self.legacy_first(child, offset)?.0)
                        || (!child.prediction.is_empty()
                            && self.executor.predicts(&child.prediction, offset))
                    {
                        return Ok(BTreeSet::new());
                    }
                }
                return Ok(BTreeSet::new());
            }
        };
        Ok(matched.map(|m| m.consumed).into_iter().collect())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn controls() -> Programs {
        let keyword = |id: &str, value: &str| json!({"id":id,"cardinality":"","kind":"keyword","value":value});
        let call = |id: &str, rule: &str, cardinality: &str| json!({"id":id,"cardinality":cardinality,"kind":"call","rule":rule});
        let rule = |body: Value| json!({"owner":"https://www.omg.org/spec/SysML/20250201#//Expression","scalar":false,"body":body});
        serde_json::from_value(json!({"rules":{
            "nested":rule(json!({"id":"nested/choice","kind":"choice","cardinality":"","elements":[
                {"id":"nested/group","kind":"sequence","cardinality":"","elements":[keyword("open","("),call("recur","nested",""),keyword("close",")")]},
                keyword("leaf","a")]})),
            "optional":rule(json!({"id":"optional/group","kind":"sequence","cardinality":"","elements":[call("opt","nested","?"),keyword("last","b")]})),
            "repeat":rule(json!({"id":"repeat/group","kind":"sequence","cardinality":"","elements":[call("items","nested","*"),keyword("last","b")]})),
            "committed":rule(json!({"id":"committed/choice","kind":"choice","cardinality":"","elements":[
                {"id":"committed/group","kind":"sequence","cardinality":"","first_set_predicated":true,"elements":[keyword("first","a"),keyword("second","b")]},
                keyword("fallback","a")]})),
            "cycle":rule(call("cycle/call","cycle","")),
            "empty_repeat":rule(json!({"id":"empty_repeat","kind":"create","classifier":"https://www.omg.org/spec/SysML/20250201#//Expression","cardinality":"*"})),
            "nullable_first":rule(json!({"id":"nullable/choice","kind":"choice","cardinality":"","elements":[
                {"id":"nullable/sequence","kind":"sequence","cardinality":"","elements":[
                    {"id":"nullable/action","kind":"create","classifier":"https://www.omg.org/spec/SysML/20250201#//Expression","cardinality":""},
                    call("nullable/call","nested","?"),keyword("last","b")]},
                keyword("other","c")]}))
        }})).unwrap()
    }

    #[test]
    fn heap_probe_preserves_shallow_recognition_first_sets_and_commitment() {
        let program = controls();
        for rule in ["nested","optional","repeat","committed","nullable_first"] {
            for source in ["", "a", "a b", "a c", "b", "c", "(a)", "((a)) b", "(a) a b", "(a", "a b a"] {
                let tokens=crate::xtext_terminal::lex(source).unwrap();
                let stack=RefCell::new(Vec::new());let cache=RefCell::new(BTreeMap::new());
                let executor=Executor{programs:&program,tokens:&tokens,kind:"Expression",kerml:true,speculative:false,call_stack:&stack,prediction_cache:&cache};
                let mut current=SyntaxProbe{executor:&executor,active:Vec::new(),memo:BTreeMap::new()};
                let mut prior=SyntaxProbe{executor:&executor,active:Vec::new(),memo:BTreeMap::new()};
                assert_eq!(current.node(&program.rules[rule].body,0),prior.legacy_node(&program.rules[rule].body,0),"{rule}: {source}");
                assert_eq!(current.first(&program.rules[rule].body,0),prior.legacy_first(&program.rules[rule].body,0),"FIRST {rule}: {source}");
                assert!(current.active.is_empty());
                assert!(stack.borrow().is_empty());
            }
        }
    }

    #[test]
    fn heap_probe_deep_rules_cycle_and_work_limits_unwind_without_semantics() {
        let program=controls();
        for (rule,source,error) in [
            ("nested",format!("{}a{}","(".repeat(128),")".repeat(128)),None),
            ("nested",format!("{}a{}","(".repeat(257),")".repeat(257)),Some("frame budget exceeded")),
            ("cycle","a".into(),Some("Non-consuming")),
            ("empty_repeat","a".into(),Some("Non-progressing")),
        ] {
            let tokens=crate::xtext_terminal::lex(&source).unwrap();
            let stack=RefCell::new(Vec::new());let cache=RefCell::new(BTreeMap::new());
            let executor=Executor{programs:&program,tokens:&tokens,kind:"Expression",kerml:true,speculative:false,call_stack:&stack,prediction_cache:&cache};
            let mut probe=SyntaxProbe{executor:&executor,active:Vec::new(),memo:BTreeMap::new()};
            let result=probe.node(&program.rules[rule].body,0);
            match error {
                Some(message)=>assert!(result.unwrap_err().contains(message),"{rule}"),
                None=>assert!(result.unwrap().contains(&(tokens.len()-1))),
            }
            assert!(probe.active.is_empty());
            assert!(stack.borrow().is_empty());
            assert!(cache.borrow().is_empty(),"model/decision state must remain untouched");
            let body=&program.rules["nested"].body;
            assert!(probe.run(Frame::Node(body,0),0,2).unwrap_err().contains("step budget exceeded"));
            assert!(probe.active.is_empty());
        }
        let tokens=crate::xtext_terminal::lex("a").unwrap();
        let stack=RefCell::new(Vec::new());let cache=RefCell::new(BTreeMap::new());
        let executor=Executor{programs:&program,tokens:&tokens,kind:"Expression",kerml:true,speculative:false,call_stack:&stack,prediction_cache:&cache};
        let mut probe=SyntaxProbe{executor:&executor,active:Vec::new(),memo:BTreeMap::new()};
        assert!(probe.first(&program.rules["cycle"].body,0).unwrap_err().contains("Non-consuming"));
        assert!(probe.active.is_empty());
    }

    #[test]
    fn definition_heap_probe_parses_unchanged_pinned_sampled_functions() {
        let source=include_str!("../tests/fixtures/definition-pipeline/SampledFunctions.sysml");
        let tree=candidate_document(source,false).unwrap();
        assert_eq!(tree.object_kind.as_deref(),Some("Namespace"));
        assert!(!tree.children.is_empty());
        for fragment in [
            "calc def C { return result = (1..size(samples)-1)->forAll { in i; (samples.domainValue#(i) < samples.domainValue#(i+1)) }; }",
            "calc def C { return result = new SampledFunction(samples = domainValues->collect { in x; new SamplePair(x, calculation(x)) }); }",
            "calc def C { return result = if index == null or index == size(domainValues)? null else Linear(fn.samples#(index), fn.samples#(index+1), value); }",
        ] {
            assert!(candidate_document(fragment,false).is_ok(),"{fragment}");
            let malformed=fragment.replace(" };"," ;").replace(" });"," ;").replace(", value);",", );");
            assert!(candidate_document(&malformed,false).is_err(),"{malformed}");
        }
    }
}
