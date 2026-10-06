package dev.mercurio.pilot;

import com.google.gson.GsonBuilder;
import com.google.gson.Gson;
import java.nio.file.*;
import java.util.*;
import org.antlr.analysis.*;
import org.antlr.tool.*;
import org.antlr.grammar.v2.ANTLRParser;

/** Build-time ANTLR frontend export. Never loads or executes generated parser actions. */
public final class PilotPredictionExporter {
    static List<String> signature(Grammar grammar, GrammarAST node) {
        var result = new ArrayList<String>();
        switch (node.getType()) {
            case ANTLRParser.ACTION:
            case ANTLRParser.EOA:
            case ANTLRParser.EOB: break;
            case ANTLRParser.SYN_SEMPRED:
                var predicate = grammar.getSyntacticPredicate(node.getText());
                if (predicate == null) throw new IllegalStateException("Unresolved syntactic predicate");
                result.add("predicate:begin");
                result.addAll(signature(grammar, predicate));
                result.add("predicate:end");
                break;
            case ANTLRParser.RULE_REF: result.add("call:" + node.getText().replaceFirst("^rule", "")); break;
            case ANTLRParser.TOKEN_REF: result.add("call:" + node.getText().replaceFirst("^RULE_", "")); break;
            case ANTLRParser.STRING_LITERAL:
            case ANTLRParser.CHAR_LITERAL:
                result.add("keyword:" + Grammar.getUnescapedStringFromGrammarStringLiteral(node.getText())); break;
            case ANTLRParser.ASSIGN:
                result.addAll(signature(grammar, node.getChild(1))); break;
            case ANTLRParser.OPTIONAL:
            case ANTLRParser.CLOSURE:
            case ANTLRParser.POSITIVE_CLOSURE:
                result.add("repeat:" + (node.getType() == ANTLRParser.OPTIONAL ? "?" :
                    node.getType() == ANTLRParser.CLOSURE ? "*" : "+"));
                for (var child : node.getChildrenAsArray()) result.addAll(signature(grammar, child));
                result.add("repeat:end");
                break;
            case ANTLRParser.ALT:
                for (var child : node.getChildrenAsArray()) result.addAll(signature(grammar, child));
                break;
            case ANTLRParser.BLOCK:
                long count = Arrays.stream(node.getChildrenAsArray())
                    .filter(child -> child.getType() == ANTLRParser.ALT).count();
                if (count > 1) result.add("choice:begin");
                int seen = 0;
                for (var child : node.getChildrenAsArray()) {
                    if (child.getType() == ANTLRParser.ALT && seen++ > 0) result.add("choice:or");
                    result.addAll(signature(grammar, child));
                }
                if (count > 1) result.add("choice:end");
                break;
            default: throw new IllegalStateException("Unsupported decision signature AST " + node.getType() + " " + node.getText());
        }
        return result;
    }
    static String symbol(Grammar g, int token) {
        if (token == Label.EOF) return "eof";
        if (token < Label.MIN_TOKEN_TYPE) throw new IllegalStateException("Unsupported token " + token);
        String name = g.getTokenDisplayName(token);
        if (name.startsWith("'")) return "keyword:" + Grammar.getUnescapedStringFromGrammarStringLiteral(name);
        if (name.startsWith("RULE_")) return "terminal:" + name.substring(5);
        throw new IllegalStateException("Unresolved token " + name);
    }
    static Map<String,Object> condition(Grammar grammar, SemanticContext context) {
        if (context instanceof SemanticContext.TruePredicate)
            return Map.of("kind", "always");
        if (context instanceof SemanticContext.Predicate predicate && predicate.isSyntacticPredicate()
                && predicate.predicateAST != null
                && predicate.predicateAST.getType() == ANTLRParser.SYN_SEMPRED) {
            var syntax = grammar.getSyntacticPredicate(predicate.predicateAST.getText());
            if (syntax == null) throw new IllegalStateException("Unresolved predicate syntax");
            return Map.of("kind", "syntax", "signature", signature(grammar, syntax));
        }
        throw new IllegalStateException("Unsupported predicate expression " + context.getClass().getSimpleName());
    }
    static Map<String,Object> export(Grammar g, DFA d, List<List<String>> alternatives, String cardinality) {
        int alternativeCount = g.getNumberOfAltsForDecisionNFA(d.decisionNFAStartState);
        boolean exit = alternativeCount == alternatives.size() + 1;
        if (exit && !List.of("?", "*", "+").contains(cardinality))
            throw new IllegalStateException("Unexpected implicit exit");
        if (alternativeCount != alternatives.size() && !exit)
            throw new IllegalStateException("Unexpected alternative count");
        if (g.getNumberOfAltsForDecisionNFA(d.decisionNFAStartState) != alternativeCount)
            throw new IllegalStateException("Decision alternative count mismatch");
        var states = new ArrayList<DFAState>();
        var indices = new IdentityHashMap<DFAState,Integer>();
        states.add(d.startState);
        indices.put(d.startState, 0);
        var rows = new ArrayList<Object>();
        for (int index = 0; index < states.size(); index++) {
            var state = states.get(index);
            if (state.abortedDueToRecursionOverflow)
                throw new IllegalStateException("Decision analysis recursion overflow");
            var predicateEdges = new ArrayList<Object>();
            var gate = state.getGatedPredicatesInNFAConfigurations();
            var edges = new TreeMap<String,Integer>();
            for (int i = 0; i < state.getNumberOfTransitions(); i++) {
                var transition = state.transition(i);
                var label = transition.label;
                if (!(label.isAtom() || label.isSet() || label.isSemanticPredicate()))
                    throw new IllegalStateException("Unsupported transition " + label);
                var target = (DFAState) transition.target;
                if (!indices.containsKey(target)) {
                    indices.put(target, states.size());
                    states.add(target);
                }
                if (label.isSemanticPredicate()) {
                    predicateEdges.add(Map.of("condition", condition(g, label.getSemanticContext()),
                        "target", indices.get(target)));
                    continue;
                }
                List<Integer> tokens = label.isAtom() ? List.of(label.getAtom()) : label.getSet().toList();
                for (int token : tokens) {
                    String name = symbol(g, token);
                    if (edges.put(name, indices.get(target)) != null)
                        throw new IllegalStateException("Overlapping DFA transition");
                }
            }
            var row = new TreeMap<String,Object>();
            row.put("edges", edges);
            if (gate != null) row.put("gate", condition(g, gate));
            if (!predicateEdges.isEmpty()) {
                if (!edges.isEmpty()) throw new IllegalStateException("Mixed token and predicate transitions");
                row.put("predicate_edges", predicateEdges);
            }
            if (state.isAcceptState()) {
                int alternative = state.getUniquelyPredictedAlt();
                if (alternative < 1 || alternative > alternativeCount)
                    throw new IllegalStateException("Invalid accepted alternative");
                row.put("accept", alternative - 1);
            }
            rows.add(row);
        }
        var result = new TreeMap<String,Object>();
        result.put("alternatives", alternatives);
        if (exit) {
            result.put("exit_alternative", alternatives.size());
            result.put("exit_cardinality", cardinality);
        }
        result.put("states", rows);
        result.put("start", 0);
        return result;
    }
    // Bound analysis work, not elapsed time or accepted lookahead length.
    // Exceeding either bound rejects the entire decision, never a partial DFA.
    static int stateLimit = 10000;
    static int configurationLimit = 10000000;
    static final class BoundedDFA extends DFA {
        int configurations;
        BoundedDFA(int decision, NFAState start) { super(decision, start); }
        void configuration() {
            if (++configurations > configurationLimit)
                throw new IllegalStateException("Deterministic analysis budget: " + configurationLimit + " configurations");
        }
        @Override public DFAState newState() {
            if (stateCounter >= stateLimit)
                throw new IllegalStateException("Deterministic analysis budget: " + stateLimit + " states");
            var state = new DFAState(this) {
                @Override public void addNFAConfiguration(NFAState nfa, NFAConfiguration config) {
                    configuration();
                    super.addNFAConfiguration(nfa, config);
                }
                @Override public NFAConfiguration addNFAConfiguration(NFAState nfa, int alternative,
                        NFAContext context, SemanticContext semantic) {
                    configuration();
                    return super.addNFAConfiguration(nfa, alternative, context, semantic);
                }
            };
            state.stateNumber = stateCounter++;
            states.setSize(state.stateNumber + 1);
            states.set(state.stateNumber, state);
            return state;
        }
    }
    static final class Diagnostics implements ANTLRErrorListener {
        final TreeSet<String> issues = new TreeSet<>();
        public void info(String message) {}
        public void error(Message message) { issues.add("error:" + message.msgID); }
        public void error(ToolMessage message) { issues.add("error:" + message.msgID); }
        public void warning(Message message) { issues.add("warning:" + message.msgID); }
    }
    static final class Request {
        String key, rule, source_id, cardinality, mode;
        List<List<String>> alternatives;
        List<String> rule_signature;
        int occurrence, occurrences;
    }
    static final class Candidate {
        int decision, line, column;
        List<List<String>> alternatives;
        Candidate(int decision, List<List<String>> alternatives, GrammarAST ast) {
            this.decision = decision; this.alternatives = alternatives;
            this.line = ast.getLine(); this.column = ast.getColumn();
        }
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 3 && args.length != 5)
            throw new IllegalArgumentException("Expected grammar, output, requests and optional state/configuration limits");
        if (args.length == 5) {
            stateLimit = Integer.parseInt(args[3]);
            configurationLimit = Integer.parseInt(args[4]);
        }
        if (stateLimit <= 0 || configurationLimit <= 0)
            throw new IllegalArgumentException("Analysis limits must be positive");
        // The frontend defaults to a one-second per-decision wall-clock
        // cutoff, which can change exports under CPU contention. The Python
        // driver bounds the whole process; an overrun fails without output.
        DFA.MAX_TIME_PER_DFA_CREATION = 0;
        var diagnostics = new Diagnostics();
        ErrorManager.setErrorListener(diagnostics);
        var grammar = new Grammar(Files.readString(Path.of(args[0])));
        grammar.buildNFA();
        if (ErrorManager.getNumErrors() != 0)
            throw new IllegalStateException("ANTLR grammar frontend reported errors");
        var requests = new Gson().fromJson(Files.readString(Path.of(args[2])), Request[].class);
        var byRule = new TreeMap<String,List<Candidate>>();
        for (int decision=1; decision<=grammar.getNumberOfDecisions(); decision++) {
            var ast = grammar.getDecisionBlockAST(decision);
            var alternatives = new ArrayList<List<String>>();
            try {
                for (var alt : ast.getChildrenAsArray())
                    if (alt.getType() == ANTLRParser.ALT) alternatives.add(signature(grammar, alt));
            } catch (IllegalStateException unsupported) { continue; }
            if (alternatives.isEmpty() || alternatives.stream().anyMatch(List::isEmpty)) continue;
            byRule.computeIfAbsent(ast.enclosingRuleName, ignored -> new ArrayList<>())
                .add(new Candidate(decision, alternatives, ast));
        }
        var decisions = new TreeMap<String,Object>();
        var unsupported = new TreeMap<String,String>();
        for (var request : requests) {
            var matches = byRule.getOrDefault("rule" + request.rule, List.of()).stream()
                .filter(candidate -> candidate.alternatives.equals(request.alternatives)).toList();
            Map<String,Object> contextual = null;
            if (matches.size() > 1 && request.rule_signature != null) {
                try {
                    var block = grammar.getRule("rule" + request.rule).tree.getFirstChildWithType(ANTLRParser.BLOCK);
                    if (!signature(grammar, block).equals(request.rule_signature)
                            || matches.size() != request.occurrences)
                        throw new IllegalStateException("Changed enclosing rule or occurrence count");
                    var ordered = matches.stream().sorted(Comparator.comparingInt((Candidate c) -> c.line)
                        .thenComparingInt(c -> c.column)).toList();
                    for (int i = 1; i < ordered.size(); i++)
                        if (ordered.get(i).line == ordered.get(i-1).line && ordered.get(i).column == ordered.get(i-1).column)
                            throw new IllegalStateException("Multiple decisions at one source location");
                    if (request.occurrence < 0 || request.occurrence >= ordered.size())
                        throw new IllegalStateException("Invalid source occurrence");
                    contextual = new TreeMap<>();
                    contextual.put("rule_signature", request.rule_signature);
                    contextual.put("occurrence", request.occurrence);
                    contextual.put("occurrences", request.occurrences);
                    matches = List.of(ordered.get(request.occurrence));
                } catch (RuntimeException error) {
                    unsupported.put(request.source_id, "Unresolved contextual decision: " + error.getMessage());
                    continue;
                }
            }
            if (matches.size() != 1) {
                unsupported.put(request.source_id, "Expected one structured ANTLR decision, found " + matches.size());
                continue;
            }
            int errorsBefore = ErrorManager.getNumErrors();
            diagnostics.issues.clear();
            try {
                var candidate = matches.get(0);
                // Use the frontend's exact one-token analysis first. It can
                // prove disjoint alternatives without expanding recursive
                // caller contexts; a null result still requires full analysis.
                DFA dfa = grammar.createLL_1_LookaheadDFA(candidate.decision);
                if (dfa == null)
                    dfa = new BoundedDFA(candidate.decision, grammar.getDecisionNFAStartState(candidate.decision));
                boolean predicateRetry = false;
                // Match Grammar.createLookaheadDFA's semantic retry, excluding
                // its wall-clock timeout branch. Budget exhaustion still throws.
                if (!dfa.analysisTimedOut()
                        && (dfa.probe.isNonLLStarDecision() || dfa.probe.analysisOverflowed())
                        && dfa.okToRetryDFAWithK1()) {
                    var block = grammar.getDecisionBlockAST(candidate.decision);
                    var previous = block.getBlockOptions();
                    previous = previous == null ? null : new HashMap<>(previous);
                    grammar.decisionsWhoseDFAsUsesSynPreds.remove(dfa);
                    try {
                        block.setBlockOption(grammar, "k", Integer.valueOf(1));
                        dfa = new BoundedDFA(candidate.decision, grammar.getDecisionNFAStartState(candidate.decision));
                        predicateRetry = true;
                    } finally {
                        block.setBlockOptions(previous);
                    }
                }
                dfa.probe.issueWarnings();
                if (dfa.analysisTimedOut())
                    throw new IllegalStateException("Incomplete timed-out decision analysis");
                var decision = export(grammar, dfa, candidate.alternatives, request.cardinality);
                if (ErrorManager.getNumErrors() != errorsBefore || !diagnostics.issues.isEmpty())
                    throw new IllegalStateException("ANTLR decision diagnostics " + diagnostics.issues);
                if (predicateRetry) decision.put("analysis", "bounded_predicate_retry_k1");
                if ("entry".equals(request.mode)) {
                    if (!decision.containsKey("exit_alternative"))
                        throw new IllegalStateException("Entry gate lacks an explicit exit");
                    decision.put("mode", "entry");
                }
                if (contextual != null) decision.put("contextual_binding", contextual);
                decisions.put(request.key, decision);
            } catch (RuntimeException error) {
                unsupported.put(request.source_id, error.getClass().getSimpleName() + ": " + error.getMessage());
            }
        }
        var vocabulary = new TreeSet<String>();
        for (String literal : grammar.getStringLiterals())
            vocabulary.add(Grammar.getUnescapedStringFromGrammarStringLiteral(literal).toString());
        decisions.put("keywords", vocabulary);
        var result = new TreeMap<String,Object>();
        result.put("decisions", decisions);
        result.put("unsupported", unsupported);
        Files.writeString(Path.of(args[1]), new GsonBuilder().setPrettyPrinting().create().toJson(result) + "\n");
    }
}
