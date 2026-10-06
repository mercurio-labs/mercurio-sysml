package dev.mercurio.pilot;

import com.google.gson.*;
import java.nio.file.*;
import java.util.*;
import org.antlr.analysis.*;
import org.antlr.tool.*;
import org.antlr.grammar.v2.ANTLRParser;

/** Experimental finite frontend graph; never executes generated parser actions. */
public final class PilotNfaExporter {
    public static void main(String[] args) throws Exception {
        var diagnostics = new PilotPredictionExporter.Diagnostics();
        ErrorManager.setErrorListener(diagnostics);
        var grammar = new Grammar(Files.readString(Path.of(args[0])));
        grammar.buildNFA();
        if (ErrorManager.getNumErrors() != 0) throw new IllegalStateException("Invalid frontend grammar");
        var requests = new Gson().fromJson(Files.readString(Path.of(args[2])), PilotPredictionExporter.Request[].class);
        var decisions = new TreeMap<String,Object>();
        var unsupported = new TreeMap<String,String>();
        var states = new TreeMap<Integer,NFAState>();
        var pending = new ArrayDeque<NFAState>();
        for (Rule rule : grammar.getRules()) pending.add(rule.startState);
        for (var request : requests) {
            var matches = new ArrayList<Integer>();
            for (int index = 1; index <= grammar.getNumberOfDecisions(); index++) {
                var ast = grammar.getDecisionBlockAST(index);
                if (!("rule" + request.rule).equals(ast.enclosingRuleName)) continue;
                var alternatives = new ArrayList<List<String>>();
                try {
                    for (var child : ast.getChildrenAsArray()) if (child.getType() == ANTLRParser.ALT)
                        alternatives.add(PilotPredictionExporter.signature(grammar, child));
                } catch (IllegalStateException error) { continue; }
                if (alternatives.equals(request.alternatives)) matches.add(index);
            }
            if (matches.size() > 1 && request.rule_signature != null) {
                var block = grammar.getRule("rule" + request.rule).tree.getFirstChildWithType(ANTLRParser.BLOCK);
                matches.sort(Comparator.comparingInt((Integer i) -> grammar.getDecisionBlockAST(i).getLine())
                    .thenComparingInt(i -> grammar.getDecisionBlockAST(i).getColumn()));
                if (!PilotPredictionExporter.signature(grammar, block).equals(request.rule_signature)
                        || matches.size() != request.occurrences || request.occurrence < 0 || request.occurrence >= matches.size())
                    throw new IllegalStateException("Changed contextual source occurrence: " + request.source_id);
                matches = new ArrayList<>(List.of(matches.get(request.occurrence)));
            }
            if (matches.size() != 1) {
                unsupported.put(request.source_id, "Unresolved source decision: " + matches.size()); continue;
            }
            var start = grammar.getDecisionNFAStartState(matches.get(0));
            int count = grammar.getNumberOfAltsForDecisionNFA(start);
            if (count != request.alternatives.size() && count != request.alternatives.size() + 1)
                throw new IllegalStateException("Unexpected alternative count");
            var entries = new ArrayList<Integer>();
            for (int alternative = 1; alternative <= count; alternative++) {
                int walk = start.translateDisplayAltToWalkAlt(alternative);
                var entry = grammar.getNFAStateForAltOfDecision(start, walk).transition(0).target;
                entries.add(entry.stateNumber);
                pending.add((NFAState)entry);
            }
            var row = new TreeMap<String,Object>();
            row.put("entries", entries);
            row.put("alternatives", request.alternatives);
            row.put("rule", request.rule);
            row.put("cardinality", request.cardinality == null ? "" : request.cardinality);
            decisions.put(request.source_id, row);
        }
        var rows = new TreeMap<Integer,Object>();
        var ruleSignatures = new TreeMap<String,Object>();
        while (!pending.isEmpty()) {
            var state = pending.removeFirst();
            if (states.putIfAbsent(state.stateNumber, state) != null) continue;
            var edges = new ArrayList<Object>();
            boolean stop = state.enclosingRule != null && state == state.enclosingRule.stopState;
            // Preserve the frontend FOLLOW union separately from precise call returns.
            for (int index = 0; index < state.getNumberOfTransitions(); index++) {
                var transition = state.transition(index);
                var edge = new TreeMap<String,Object>();
                edge.put("target", transition.target.stateNumber);
                pending.add((NFAState)transition.target);
                if (transition instanceof RuleClosureTransition call) {
                    edge.put("kind", "call"); edge.put("follow", call.followState.stateNumber);
                    pending.add(call.followState);
                } else if (transition.label.isAction()) {
                    edge.put("kind", "action"); // retained explicitly; not evaluated or confused with predicates
                } else if (transition.label.isEpsilon()) edge.put("kind", "epsilon");
                else if (transition.label.isSemanticPredicate()) {
                    edge.put("kind", "predicate");
                    var semantic = transition.label.getSemanticContext();
                    var condition = new TreeMap<String,Object>(PilotPredictionExporter.condition(grammar, semantic));
                    if (!(semantic instanceof SemanticContext.Predicate predicate) || predicate.predicateAST == null)
                        throw new IllegalStateException("Predicate source position unavailable");
                    condition.put("source_line", predicate.predicateAST.getLine());
                    condition.put("source_column", predicate.predicateAST.getColumn());
                    edge.put("condition", condition);
                    var owner = state.enclosingRule;
                    var body = owner.tree.getFirstChildWithType(ANTLRParser.BLOCK);
                    ruleSignatures.put(owner.name, PilotPredictionExporter.signature(grammar, body));
                } else if (transition.label.isAtom() || transition.label.isSet()) {
                    edge.put("kind", "tokens");
                    var symbols = new TreeSet<String>();
                    List<Integer> tokens = transition.label.isAtom() ? List.of(transition.label.getAtom()) : transition.label.getSet().toList();
                    for (int token : tokens) symbols.add(PilotPredictionExporter.symbol(grammar, token));
                    edge.put("symbols", symbols);
                } else throw new IllegalStateException("Unsupported NFA edge " + transition.label);
                edges.add(edge);
            }
            rows.put(state.stateNumber, Map.of("stop", stop, "edges", stop ? List.of() : edges,
                "follow_edges", stop ? edges : List.of(), "rule", state.enclosingRule == null ? "" : state.enclosingRule.name));
        }
        var result = new TreeMap<String,Object>();
        result.put("scope", "Experimental graph only: ambiguity policy, predicates, native execution and parser commitment remain unqualified.");
        result.put("rule_signatures", ruleSignatures);
        result.put("states", rows); result.put("decisions", decisions); result.put("unsupported", unsupported);
        Files.writeString(Path.of(args[1]), new GsonBuilder().setPrettyPrinting().create().toJson(result) + "\n");
    }
}
