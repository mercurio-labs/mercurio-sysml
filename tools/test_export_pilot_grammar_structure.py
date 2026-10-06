"""Focused negative controls for the source grammar export contract."""
import unittest
import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from export_pilot_grammar_structure import HELPER, ROOT, SOURCE_PATHS, DEFAULT_OUTPUT, validate_structure, validate_contexts


def fixture():
    span = {"offset": 0, "length": 1, "start_line": 1, "end_line": 1}
    return {
        "schema_version": 1,
        "source_format": "raw-xtext",
        "parser": "org.eclipse.xtext.XtextStandaloneSetup",
        "span_encoding": "utf-16-code-units",
        "grammars": [{
            "kind": "Grammar", "id": "example.G", "name": "example.G", "origin": "source", "span": span,
            "fields": {"hiddenTokens": [{"$ref": "example.G::WS"}]},
            "rules": [{"kind": "TerminalRule", "id": "example.G::WS", "origin": "source", "span": span, "fields": {}}],
        }],
        "coverage": {"node_kinds": {"Grammar": 1, "TerminalRule": 1}, "nodes": 2, "references": 1, "inferred_nodes_without_source_span": 0, "unresolved_references": 0, "unsupported_nodes": 0},
    }


class GrammarExportContractTests(unittest.TestCase):
    def test_complete_internal_reference(self):
        validate_structure(fixture())

    def test_rejects_dangling_inherited_rule(self):
        document = fixture()
        document["grammars"][0]["fields"]["hiddenTokens"][0]["$ref"] = "other.G::WS"
        with self.assertRaisesRegex(ValueError, "Dangling"):
            validate_structure(document)

    def test_rejects_omitted_rule_even_with_updated_coverage(self):
        document = fixture()
        document["grammars"][0]["rules"] = []
        document["coverage"].update(node_kinds={"Grammar": 1}, nodes=1)
        with self.assertRaisesRegex(ValueError, "Dangling"):
            validate_structure(document)

    def test_rejects_duplicate_identity(self):
        document = fixture()
        document["grammars"][0]["rules"] *= 2
        with self.assertRaisesRegex(ValueError, "duplicate"):
            validate_structure(document)

    def test_rejects_false_coverage(self):
        document = fixture()
        document["coverage"]["unsupported_nodes"] = 1
        with self.assertRaisesRegex(ValueError, "coverage"):
            validate_structure(document)

    def test_rejects_fabricated_span_on_inferred_node(self):
        document = fixture()
        document["grammars"][0]["rules"][0]["origin"] = "xtext-inferred"
        with self.assertRaisesRegex(ValueError, "inferred"):
            validate_structure(document)

    def test_rejects_missing_structural_fields(self):
        document = fixture()
        del document["grammars"][0]["rules"][0]["fields"]
        with self.assertRaisesRegex(ValueError, "structural fields"):
            validate_structure(document)

    def test_rejects_unknown_offset_units(self):
        document = fixture()
        document["span_encoding"] = "utf-8-bytes"
        with self.assertRaisesRegex(ValueError, "UTF-16"):
            validate_structure(document)


class GrammarResolutionContextTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = json.loads(DEFAULT_OUTPUT.read_text(encoding='utf-8'))

    def test_hoisted_predicates_require_visible_guard_identities(self):
        document = copy.deepcopy(self.document)
        hoisted = document['resolution_contexts']['org.omg.sysml.xtext.SysML']['hoisted_predicates']
        self.assertEqual(len(hoisted), 1)
        source = next(iter(hoisted))
        hoisted[source] = 'missing::guard'
        with self.assertRaisesRegex(ValueError, 'Invalid contextual hoisted predicate identity'):
            validate_contexts(document)

    def test_contextual_first_sets_are_complete_and_terminal_bound(self):
        for mutation in ['missing', 'empty', 'nonterminal']:
            document = copy.deepcopy(self.document)
            sets = document['resolution_contexts']['org.omg.sysml.xtext.SysML']['first_set_predicates']
            identity = next(iter(sets))
            if mutation == 'missing': del sets[identity]
            elif mutation == 'empty': sets[identity] = []
            else: sets[identity] = ['call:OwnedExpression']
            with self.assertRaisesRegex(ValueError, 'first.set|First set'):
                validate_contexts(document)

    def test_pinned_contexts_preserve_independent_overrides(self):
        validate_structure(self.document)
        validate_contexts(self.document)
        base = 'org.omg.kerml.expressions.xtext.KerMLExpressions'
        call = base + '::ExpressionBodyMember/alternatives/terminal'
        for name, context in self.document['resolution_contexts'].items():
            self.assertEqual(context['effective_rules']['ExpressionBody'], name + '::ExpressionBody')
            self.assertEqual(context['rule_calls'][call], name + '::ExpressionBody')
        base_rule = next(r for g in self.document['grammars'] if g['id'] == base
                         for r in g['rules'] if r['id'] == base + '::ExpressionBodyMember')
        self.assertEqual(base_rule['fields']['alternatives']['fields']['terminal']['fields']['rule']['$ref'],
                         base + '::ExpressionBody')

    def test_missing_and_cross_language_bindings_fail(self):
        for mutation in ['missing', 'sibling', 'effective', 'source']:
            doc = copy.deepcopy(self.document)
            context = doc['resolution_contexts']['org.omg.kerml.xtext.KerML']
            call = 'org.omg.kerml.expressions.xtext.KerMLExpressions::ExpressionBodyMember/alternatives/terminal'
            if mutation == 'missing':
                del context['rule_calls'][call]
            elif mutation == 'sibling':
                context['rule_calls'][call] = 'org.omg.sysml.xtext.SysML::ExpressionBody'
            elif mutation == 'effective':
                context['effective_rules']['ExpressionBody'] = 'org.omg.sysml.xtext.SysML::ExpressionBody'
            else:
                base_rule = next(r for g in doc['grammars'] for r in g['rules']
                                 if r['id'] == call.split('/')[0])
                base_rule['fields']['alternatives']['fields']['terminal']['fields']['rule']['$ref'] = 'org.omg.sysml.xtext.SysML::ExpressionBody'
            with self.assertRaisesRegex(ValueError, 'contextual rule|effective rule|Contaminated source'):
                validate_contexts(doc)


@unittest.skipUnless(os.environ.get("MERCURIO_GRAMMAR_INTEGRATION_JAVA_BIN"), "Set MERCURIO_GRAMMAR_INTEGRATION_JAVA_BIN for raw Xtext integration controls")
class RawXtextIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.java_bin = Path(os.environ["MERCURIO_GRAMMAR_INTEGRATION_JAVA_BIN"])
        cls.support = ROOT.parent / "target/support-2026-08/grammar-structure"
        cls.source = ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation"
        cls.jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
        cls.suffix = ".exe" if os.name == "nt" else ""
        cls.support.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix="grammar integration ", dir=cls.support)
        cls.addClassCleanup(temporary.cleanup)
        cls.classes = Path(temporary.name) / "classes"
        cls.classes.mkdir()
        cls.baseline = Path(temporary.name) / "baseline.json"
        subprocess.run([
            str(cls.java_bin / ("javac" + cls.suffix)), "-encoding", "UTF-8", "-cp",
            str(cls.jar), "-d", str(cls.classes), str(HELPER),
        ], capture_output=True, text=True, check=True, timeout=60)
        subprocess.run([
            str(cls.java_bin / ("java" + cls.suffix)), "-Xmx2g", "-cp",
            str(cls.classes) + os.pathsep + str(cls.jar),
            "dev.mercurio.pilot.PilotGrammarExporter", str(cls.source), str(cls.baseline),
        ], capture_output=True, text=True, check=True, timeout=60)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="relocated grammar ", dir=self.support)
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        for relative in SOURCE_PATHS:
            target = self.directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(self.source / relative, target)

    def export(self):
        self.output = self.directory / "result.json"
        return subprocess.run([
            str(self.java_bin / ("java" + self.suffix)), "-Xmx2g", "-cp",
            str(self.classes) + os.pathsep + str(self.jar),
            "dev.mercurio.pilot.PilotGrammarExporter", str(self.directory), str(self.output),
        ], capture_output=True, text=True, timeout=60)

    def test_relocated_sources_have_identical_export(self):
        result = self.export()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.output.read_bytes(), self.baseline.read_bytes())
        validate_structure(json.loads(self.output.read_text(encoding="utf-8")))
        validate_contexts(json.loads(self.output.read_text(encoding="utf-8")))

    def test_unresolved_rule_fails_without_output(self):
        source = self.directory / SOURCE_PATHS[2]
        text = source.read_text(encoding="utf-8")
        self.assertIn("ownedRelationship += OwnedAnnotation", text)
        source.write_text(text.replace("ownedRelationship += OwnedAnnotation", "ownedRelationship += MissingGrammarRule", 1), encoding="utf-8")
        result = self.export()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Grammar link failed", result.stderr)
        self.assertFalse(self.output.exists())

    def test_malformed_grammar_fails_without_output(self):
        source = self.directory / SOURCE_PATHS[2]
        text = source.read_text(encoding="utf-8")
        self.assertIn("RelationshipBody returns SysML::Relationship :", text)
        source.write_text(text.replace("RelationshipBody returns SysML::Relationship :", "RelationshipBody returns SysML::Relationship :::", 1), encoding="utf-8")
        result = self.export()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Xtext parse failed", result.stderr)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
