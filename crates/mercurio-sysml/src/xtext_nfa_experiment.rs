//! Bounded native graph prediction for the opt-in definition-driven candidate.
//! This is not an LL(*) equivalence claim or a production fallback. Unbound
//! predicates and caller FOLLOW requirements are errors, never guessed branches.
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::OnceLock,
};

#[derive(Deserialize)]
pub(crate) struct Graph {
    states: BTreeMap<usize, State>,
    decisions: BTreeMap<String, Decision>,
    #[serde(skip)]
    reachability: OnceLock<Reachability>,
}
#[derive(Default)]
struct Reachability {
    predecessors: BTreeMap<usize, Vec<usize>>,
    token_sources: BTreeMap<String, BTreeSet<usize>>,
    unresolved: BTreeSet<usize>,
    viable: BTreeMap<String, BTreeSet<usize>>,
    fallback: BTreeSet<usize>,
}

#[derive(Deserialize)]
struct State {
    stop: bool,
    edges: Vec<Edge>,
    #[serde(default)]
    follow_edges: Vec<Edge>,
}
#[derive(Deserialize)]
struct Decision {
    entries: Vec<usize>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Edge {
    Epsilon {
        target: usize,
    },
    Action {
        target: usize,
    },
    Call {
        target: usize,
        follow: usize,
    },
    Tokens {
        target: usize,
        symbols: Vec<String>,
    },
    Predicate {
        target: usize,
        condition: serde_json::Value,
    },
}
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Configuration {
    state: usize,
    returns: Vec<usize>,
    alternative: usize,
    predicates_visible: bool,
    guarded: bool,
}

impl Graph {
    // Conservative FIRST reachability over the imported finite graph. Predicate
    // truth and precise return stacks only remove paths from this union. Every
    // rule stop is conservatively nullable: a precise return may differ from
    // global FOLLOW. Filtering must neither erase that return nor hide a
    // required-context error.
    fn viable_states(&self, token: &str) -> &BTreeSet<usize> {
        let index = self.reachability.get_or_init(|| {
            let mut index = Reachability::default();
            for (&id, state) in &self.states {
                if state.stop {
                    index.unresolved.insert(id);
                }
                let edges = if state.stop {
                    &state.follow_edges
                } else {
                    &state.edges
                };
                for edge in edges {
                    match edge {
                        Edge::Tokens { symbols, .. } => {
                            for symbol in symbols {
                                index
                                    .token_sources
                                    .entry(symbol.clone())
                                    .or_default()
                                    .insert(id);
                            }
                        }
                        Edge::Epsilon { target }
                        | Edge::Action { target }
                        | Edge::Call { target, .. }
                        | Edge::Predicate { target, .. } => {
                            if !self.states.contains_key(target) {
                                index.unresolved.insert(id);
                            }
                            index.predecessors.entry(*target).or_default().push(id);
                        }
                    }
                }
            }
            // This projection depends only on the immutable imported graph.
            // Cache its finite alphabet; user input cannot grow the dictionary.
            index.fallback = Self::predecessor_closure(&index, std::iter::empty());
            index.viable = index.token_sources.iter().map(|(symbol, sources)| {
                (symbol.clone(), Self::predecessor_closure(&index, sources.iter().copied()))
            }).collect();
            index
        });
        index.viable.get(token).unwrap_or(&index.fallback)
    }

    fn predecessor_closure(
        index: &Reachability, sources: impl IntoIterator<Item = usize>,
    ) -> BTreeSet<usize> {
        let mut reached = index.unresolved.clone();
        reached.extend(sources);
        let mut pending: Vec<_> = reached.iter().copied().collect();
        while let Some(state) = pending.pop() {
            for &previous in index.predecessors.get(&state).into_iter().flatten() {
                if reached.insert(previous) {
                    pending.push(previous);
                }
            }
        }
        reached
    }

    #[cfg(test)]
    pub(crate) fn predict(
        &self,
        identity: &str,
        tokens: &[String],
        budget: usize,
    ) -> Result<Option<usize>, String> {
        self.predict_with_predicates(identity, tokens, budget, |_| {
            Err("Unbound graph predicate".into())
        })
    }

    pub(crate) fn predict_with_predicates(
        &self,
        identity: &str,
        tokens: &[String],
        budget: usize,
        probe: impl FnMut(&serde_json::Value) -> Result<bool, String>,
    ) -> Result<Option<usize>, String> {
        let mut input = tokens.iter();
        self.predict_with_input(identity, || Ok(input.next().cloned()), budget, probe)
    }

    // Ask the carrier adapter only for symbols required by this decision.
    // Reaching one alternative never scans or validates an unrelated suffix.
    pub(crate) fn predict_with_input(
        &self,
        identity: &str,
        mut input: impl FnMut() -> Result<Option<String>, String>,
        budget: usize,
        mut probe: impl FnMut(&serde_json::Value) -> Result<bool, String>,
    ) -> Result<Option<usize>, String> {
        let decision = self
            .decisions
            .get(identity)
            .ok_or("Unknown graph decision")?;
        let mut pending: VecDeque<_> = decision
            .entries
            .iter()
            .enumerate()
            .map(|(alternative, state)| Configuration {
                state: *state,
                returns: Vec::new(),
                alternative,
                predicates_visible: true,
                guarded: false,
            })
            .collect();
        let mut work = 0;
        loop {
            if pending.is_empty() {
                return Ok(None);
            }
            if pending
                .iter()
                .map(|c| c.alternative)
                .collect::<BTreeSet<_>>()
                .len()
                == 1
            {
                return Ok(Some(pending[0].alternative));
            }
            // A bounded predicate-conflict case: all remaining configurations
            // have the same continuation and exact return context. Every earlier
            // alternative must carry a successful visible guard; only the final
            // alternative may be an unconditional fallback. Other conflicts stay
            // unsupported rather than becoming general ordered backtracking.
            let first = &pending[0];
            let last_alternative = pending.iter().map(|c| c.alternative).max()
                .ok_or("Missing pending prediction alternative")?;
            if pending
                .iter()
                .all(|c| c.state == first.state && c.returns == first.returns)
                && pending
                    .iter()
                    .all(|c| c.alternative == last_alternative || c.guarded)
            {
                return Ok(pending.iter().map(|c| c.alternative).min());
            }
            let symbol = input()?;
            let token = symbol.as_ref();
            let viable = token.map(|token| self.viable_states(token));
            let mut visited = BTreeSet::new();
            let mut next = BTreeSet::new();
            while let Some(config) = pending.pop_front() {
                work += 1;
                if work > budget {
                    return Err("Graph work budget exceeded".into());
                }
                if !visited.insert(config.clone()) {
                    continue;
                }
                let state = self
                    .states
                    .get(&config.state)
                    .ok_or("Dangling graph state")?;
                if viable
                    .as_ref()
                    .is_some_and(|states| !states.contains(&config.state))
                {
                    continue;
                }
                if state.stop && !config.returns.is_empty() {
                    let mut returning = config;
                    returning.state = returning
                        .returns
                        .pop()
                        .ok_or("Caller FOLLOW context required")?;
                    pending.push_back(returning);
                    continue;
                }
                // NFAToDFAConverter.closure follows the precise return stack
                // when present; at the decision boundary it follows the imported
                // context-insensitive FOLLOW union. It does not invent a caller.
                let edges = if state.stop {
                    if state.follow_edges.is_empty() {
                        return Err("Caller FOLLOW context required".into());
                    }
                    &state.follow_edges
                } else {
                    &state.edges
                };
                for edge in edges {
                    let mut advanced = config.clone();
                    match edge {
                        Edge::Epsilon { target } => advanced.state = *target,
                        Edge::Action { target } => {
                            advanced.state = *target;
                            advanced.predicates_visible = false;
                        }
                        Edge::Call { target, follow } => {
                            if advanced.returns.len() >= 128 {
                                return Err("Graph call depth exceeded".into());
                            }
                            advanced.returns.push(*follow);
                            advanced.state = *target;
                        }
                        Edge::Tokens { target, symbols } => {
                            if token.is_some_and(|token| symbols.contains(token)) {
                                advanced.state = *target;
                                advanced.predicates_visible = false;
                                next.insert(advanced);
                            }
                            continue;
                        }
                        Edge::Predicate { target, condition } => {
                            // Pinned NFAToDFAConverter.closure collects syntactic
                            // predicates only at this decision's alternative entry,
                            // before an action or consumed token blocks visibility.
                            match condition["kind"].as_str() {
                                Some("always") => {}
                                Some("syntax")
                                    if !config.predicates_visible
                                        || config.state != decision.entries[config.alternative] => {
                                }
                                Some("syntax") => {
                                    if !probe(condition)? {
                                        continue;
                                    }
                                    advanced.guarded = true;
                                }
                                _ => return Err("Unsupported graph predicate semantics".into()),
                            }
                            advanced.state = *target;
                        }
                    }
                    pending.push_back(advanced);
                }
            }
            pending = next.into_iter().collect();
            // A missing symbol gets the same final closure pass as the slice
            // interface, but cannot manufacture a second end-of-input event.
            if token.is_none() { break; }
        }
        Err("Unresolved graph ambiguity".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn first_reachability_prunes_impossible_recursive_paths_without_hiding_missing_follow() {
        let graph: Graph = serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
            "0":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["x"]}]},
            "1":{"stop":false,"edges":[{"kind":"call","target":1,"follow":2},{"kind":"tokens","target":2,"symbols":["y"]}]},
            "2":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["z"]}]},
            "4":{"stop":true,"edges":[]}
        }})).unwrap();
        assert_eq!(graph.predict("d", &["x".into()], 10).unwrap(), Some(0));
        // No matching token must remain a rejection, not ordered selection.
        assert_eq!(graph.predict("d", &["missing".into()], 10).unwrap(), None);
        // The recursive path is viable for y, so the existing bound still applies.
        assert!(
            graph
                .predict("d", &["y".into()], 10)
                .unwrap_err()
                .contains("budget")
        );
        let unknown: Graph =
            serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
                "0":{"stop":true,"edges":[]},
                "1":{"stop":false,"edges":[{"kind":"tokens","target":0,"symbols":["x"]}]}
            }}))
            .unwrap();
        assert!(
            unknown
                .predict("d", &["missing".into()], 10)
                .unwrap_err()
                .contains("FOLLOW")
        );
    }

    #[test]
    fn calls_return_to_their_own_continuation_and_limits_fail_closed() {
        let graph: Graph =
            serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
              "0":{"stop":false,"edges":[{"kind":"call","target":2,"follow":4}]},
              "1":{"stop":false,"edges":[{"kind":"call","target":2,"follow":5}]},
              "2":{"stop":false,"edges":[{"kind":"tokens","target":3,"symbols":["x"]}]},
              "3":{"stop":true,"edges":[]},
              "4":{"stop":false,"edges":[{"kind":"tokens","target":6,"symbols":["a"]}]},
              "5":{"stop":false,"edges":[{"kind":"tokens","target":6,"symbols":["b"]}]},
              "6":{"stop":true,"edges":[]}
            }}))
            .unwrap();
        assert_eq!(
            graph.predict("d", &["x".into(), "a".into()], 100).unwrap(),
            Some(0)
        );
        assert_eq!(
            graph.predict("d", &["x".into(), "b".into()], 100).unwrap(),
            Some(1)
        );
        assert_eq!(
            graph.predict("d", &["x".into(), "c".into()], 100).unwrap(),
            None
        );
        assert!(
            graph
                .predict("d", &["x".into()], 1)
                .unwrap_err()
                .contains("budget")
        );
        assert!(graph.predict("missing", &[], 100).is_err());
    }
    #[test]
    fn predicates_and_missing_caller_context_never_select_a_fallback() {
        let source = json!({"decisions":{"d":{"entries":[0,1]}},"states":{
          "0":{"stop":false,"edges":[{"kind":"predicate","target":2,"condition":{"kind":"syntax"}}]},
          "1":{"stop":false,"edges":[{"kind":"tokens","target":2,"symbols":["x"]}]},
          "2":{"stop":true,"edges":[]}
        }});
        let graph: Graph = serde_json::from_value(source.clone()).unwrap();
        assert!(
            graph
                .predict("d", &["x".into()], 100)
                .unwrap_err()
                .contains("Unbound graph predicate")
        );
        let mut needs_caller = source;
        needs_caller["states"]["0"] = json!({"stop":true,"edges":[]});
        let graph: Graph = serde_json::from_value(needs_caller).unwrap();
        assert_eq!(
            graph.predict("d", &["x".into()], 100).unwrap_err(),
            "Caller FOLLOW context required"
        );
    }
    #[test]
    fn nested_syntax_predicates_do_not_guard_the_enclosing_decision() {
        let graph: Graph = serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
          "0":{"stop":false,"edges":[{"kind":"call","target":2,"follow":4}]},
          "1":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["b"]}]},
          "2":{"stop":false,"edges":[{"kind":"predicate","target":3,"condition":{"kind":"syntax"}}]},
          "3":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["a"]}]},
          "4":{"stop":true,"edges":[]}
        }})).unwrap();
        assert_eq!(graph.predict("d", &["a".into()], 100).unwrap(), Some(0));
        assert_eq!(graph.predict("d", &["b".into()], 100).unwrap(), Some(1));
    }

    #[test]
    fn imported_follow_is_used_only_without_a_precise_return() {
        let graph: Graph = serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
          "0":{"stop":false,"edges":[{"kind":"call","target":2,"follow":3}]},
          "1":{"stop":true,"edges":[],"follow_edges":[{"kind":"tokens","target":4,"symbols":["b"]}]},
          "2":{"stop":true,"edges":[],"follow_edges":[{"kind":"tokens","target":4,"symbols":["b"]}]},
          "3":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["a"]}]},
          "4":{"stop":true,"edges":[]}
        }})).unwrap();
        assert_eq!(graph.predict("d", &["a".into()], 100).unwrap(), Some(0));
        assert_eq!(graph.predict("d", &["b".into()], 100).unwrap(), Some(1));
    }
    #[test]
    fn converged_guarded_alternatives_preserve_priority_and_binding_errors() {
        let graph: Graph = serde_json::from_value(json!({"decisions":{"d":{"entries":[0,1]}},"states":{
          "0":{"stop":false,"edges":[{"kind":"predicate","target":2,"condition":{"kind":"syntax"}}]},
          "1":{"stop":false,"edges":[{"kind":"tokens","target":3,"symbols":["x"]}]},
          "2":{"stop":false,"edges":[{"kind":"tokens","target":3,"symbols":["x"]}]},
          "3":{"stop":true,"edges":[]}
        }})).unwrap();
        let input = vec!["x".into()];
        assert_eq!(
            graph
                .predict_with_predicates("d", &input, 100, |_| Ok(false))
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            graph
                .predict_with_predicates("d", &input, 100, |_| Ok(true))
                .unwrap(),
            Some(0)
        );
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/xtext-prediction-nfa.experimental.json"
        ))
        .unwrap();
        let controls = evidence["predicate_conflict_controls"].as_array().unwrap();
        assert_eq!(controls.len(), 2);
        for case in controls {
            assert_eq!(
                graph
                    .predict_with_predicates("d", &input, 100, |_| Ok(case["guard"]
                        .as_bool()
                        .unwrap()))
                    .unwrap(),
                Some(case["alternative"].as_u64().unwrap() as usize)
            );
        }
        assert_eq!(
            graph
                .predict_with_predicates("d", &input, 100, |_| Err("binding failure".into()))
                .unwrap_err(),
            "binding failure"
        );
        let mut ambiguous = graph;
        ambiguous.states.get_mut(&0).unwrap().edges = vec![Edge::Epsilon { target: 2 }];
        assert!(
            ambiguous
                .predict_with_predicates("d", &input, 100, |_| Ok(true))
                .is_err()
        );
    }
    #[test]
    fn demand_driven_prediction_reads_only_required_symbols_and_propagates_required_errors() {
        let graph:Graph=serde_json::from_value(json!({"decisions":{
            "d":{"entries":[0,1]},"single":{"entries":[0]},"empty":{"entries":[]}},
          "states":{
            "0":{"stop":false,"edges":[{"kind":"tokens","target":2,"symbols":["x"]}]},
            "1":{"stop":false,"edges":[{"kind":"tokens","target":3,"symbols":["x"]}]},
            "2":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["b"]}]},
            "3":{"stop":false,"edges":[{"kind":"tokens","target":4,"symbols":["a"]}]},
            "4":{"stop":true,"edges":[]}
        }})).unwrap();
        for (second,alternative) in [("a",1),("b",0)] {
            let mut reads=0;
            let selected=graph.predict_with_input("d",||{
                reads+=1;
                match reads {
                    1=>Ok(Some("x".into())),
                    2=>Ok(Some(second.into())),
                    _=>Err("unused suffix must not be decoded".into())
                }
            },100,|_|Err("unexpected predicate".into())).unwrap();
            assert_eq!(selected,Some(alternative));
            assert_eq!(reads,2);
            assert_eq!(selected,graph.predict("d",&["x".into(),second.into(),"unused".into()],100).unwrap());
        }
        let mut reads=0;
        assert_eq!(graph.predict_with_input("d",||{
            reads+=1; if reads==1{Ok(Some("x".into()))}else{Err("required carrier error".into())}
        },100,|_|Ok(false)).unwrap_err(),"required carrier error");
        assert_eq!(reads,2);
        assert_eq!(graph.predict_with_input("single",||Err("unexpected carrier read".into()),0,|_|Ok(false)).unwrap(),Some(0));
        assert_eq!(graph.predict_with_input("empty",||Err("unexpected carrier read".into()),0,|_|Ok(false)).unwrap(),None);
        assert!(graph.predict_with_input("d",||Ok(None),100,|_|Ok(false)).is_err(),"missing input cannot invent a second EOF");
    }

    #[test]
    fn immutable_first_projection_reuses_finite_alphabet_and_preserves_unresolved_states() {
        let graph:Graph=serde_json::from_value(json!({"decisions":{},"states":{
            "0":{"stop":false,"edges":[{"kind":"epsilon","target":1}]},
            "1":{"stop":false,"edges":[{"kind":"action","target":2}]},
            "2":{"stop":false,"edges":[{"kind":"call","target":3,"follow":4}]},
            "3":{"stop":false,"edges":[{"kind":"tokens","target":5,"symbols":["x"]}]},
            "4":{"stop":false,"edges":[{"kind":"tokens","target":5,"symbols":["y"]}]},
            "5":{"stop":true,"edges":[]},
            "6":{"stop":false,"edges":[{"kind":"epsilon","target":99}]},
            "7":{"stop":false,"edges":[{"kind":"predicate","target":0,"condition":{"kind":"always"}}]}
        }})).unwrap();
        assert_eq!(graph.viable_states("x"),&BTreeSet::from([0,1,2,3,5,6,7]));
        assert_eq!(graph.viable_states("y"),&BTreeSet::from([4,5,6]));
        assert!(std::ptr::eq(graph.viable_states("x"),graph.viable_states("x")));
        assert_eq!(graph.reachability.get().unwrap().viable.len(),2);
        for n in 0..100 {
            assert_eq!(graph.viable_states(&format!("unknown-{n}")),&BTreeSet::from([5,6]));
            assert!(std::ptr::eq(graph.viable_states("missing"),graph.viable_states(&format!("unknown-{n}"))));
        }
        assert_eq!(graph.reachability.get().unwrap().viable.len(),2,"input cannot grow the immutable graph cache");
    }

}
