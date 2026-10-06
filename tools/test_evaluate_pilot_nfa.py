"""Structural rejection controls for the experimental finite graph."""
import unittest
from evaluate_pilot_nfa import validate_graph, bind_predicates, GRAMMAR
import json
import copy

class FiniteGraphTests(unittest.TestCase):
    def fixture(self):
        return {"states":{"0":{"stop":False,"edges":[{"kind":"call","target":1,"follow":2}]},
                          "1":{"stop":True,"edges":[]},"2":{"stop":True,"edges":[]}},
                "decisions":{"d":{"entries":[0]}}}

    def test_call_graph_is_closed(self):
        validate_graph(self.fixture())
        for field in ["target", "follow"]:
            bad = self.fixture()
            bad["states"]["0"]["edges"][0][field] = 99
            with self.assertRaisesRegex(ValueError, "Dangling"):
                validate_graph(bad)

    def test_stop_cannot_use_global_follow(self):
        bad = self.fixture()
        bad["states"]["0"]["stop"] = True
        with self.assertRaisesRegex(ValueError, "call stack"):
            validate_graph(bad)

    def test_unknown_edges_and_missing_entries_fail(self):
        bad = self.fixture()
        bad["states"]["0"]["edges"][0]["kind"] = "guess"
        with self.assertRaisesRegex(ValueError, "Unknown edge"):
            validate_graph(bad)
        bad = self.fixture()
        bad["decisions"]["d"]["entries"] = [99]
        with self.assertRaisesRegex(ValueError, "Invalid decision"):
            validate_graph(bad)

    def test_follow_union_is_closed_and_separate(self):
        graph = self.fixture()
        graph["states"]["1"]["follow_edges"] = [{"kind":"epsilon", "target":2}]
        validate_graph(graph)
        graph["states"]["1"]["follow_edges"][0]["target"] = 99
        with self.assertRaisesRegex(ValueError, "Dangling"):
            validate_graph(graph)

    def test_changed_guard_signature_cannot_reuse_stale_binding(self):
        structured = json.loads(GRAMMAR.read_text(encoding="utf-8"))
        context = "org.omg.sysml.xtext.SysML"
        graph = {"states":{"0":{"rule":"ruleSendNode", "edges":[{"kind":"predicate", "condition":{
            "kind":"syntax", "signature":["call:DefinitelyNotTheGuard"],
            "source_id":"stale", "first_set":True}}]}}}
        bind_predicates(graph, structured, context)
        condition = graph["states"]["0"]["edges"][0]["condition"]
        self.assertIn("binding_error", condition)
        self.assertNotIn("source_id", condition)
        self.assertNotIn("first_set", condition)

    def test_repeated_guards_require_whole_rule_and_unique_occurrences(self):
        structured = json.loads(GRAMMAR.read_text(encoding="utf-8"))
        doc = json.loads(GRAMMAR.with_name("xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
        context = "org.omg.sysml.xtext.SysML"
        graph = doc["contexts"][context]
        def repeated(g):
            return [edge["condition"] for state in g["states"].values() if state["rule"] == "ruleActionBodyItem"
                    for edge in state["edges"] if edge["kind"] == "predicate"]
        bind_predicates(graph, structured, context)
        guards = repeated(graph)
        self.assertEqual(len(guards), 2)
        self.assertEqual(len({g["source_id"] for g in guards}), 2)
        self.assertEqual(sorted(g["occurrence_binding"]["index"] for g in guards), [0,1])
        bad = copy.deepcopy(graph)
        bad["rule_signatures"]["ruleActionBodyItem"].append("keyword:changed")
        bind_predicates(bad, structured, context)
        self.assertTrue(all("binding_error" in g and "source_id" not in g for g in repeated(bad)))
        bad = copy.deepcopy(graph)
        repeated(bad)[1]["source_line"] = repeated(bad)[0]["source_line"]
        repeated(bad)[1]["source_column"] = repeated(bad)[0]["source_column"]
        bind_predicates(bad, structured, context)
        self.assertTrue(all("binding_error" in g and "source_id" not in g for g in repeated(bad)))

if __name__ == "__main__":
    unittest.main()
