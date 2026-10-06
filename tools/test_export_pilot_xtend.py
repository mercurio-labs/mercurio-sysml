"""Structural guards and optional real-parser mutation tests for Xtend export.

Run real-parser tests with PILOT_XTEND_INTEGRATION=1 and JAVA_BIN pointing at a
JDK. Inputs are copied to a temporary directory; the pinned checkout is untouched.
"""
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from export_pilot_xtend import DEFAULT_OUTPUT, HELPER, ROOT, SOURCE, SYSML_SOURCE, SELECTION, METHODS, XTEND_VERSION, encode, validate_structure


class StructureTests(unittest.TestCase):
    def setUp(self):
        self.document = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))

    def test_complete_method_graph_resolves(self):
        validate_structure(self.document)
        self.assertEqual({method["name"] for method in self.document["methods"]}, METHODS)
        self.assertIn("org.omg.sysml.lang.sysml.Namespace", self.document["types"])
        self.assertIn("org.omg.sysml.lang.sysml.AnnotatingElement", self.document["types"])

    def test_selected_method_cannot_claim_other_source(self):
        self.document["methods"][0]["source"] = SYSML_SOURCE
        with self.assertRaisesRegex(ValueError, "source"):
            validate_structure(self.document)

    def test_constants_retain_both_source_origins(self):
        sources = {value["source"] for value in self.document["symbols"].values() if "initializer" in value}
        self.assertEqual(sources, set(SELECTION))

    def test_dangling_getter_rejected(self):
        symbol = "org.omg.sysml.lang.sysml.Import.getVisibility()"
        del self.document["symbols"][symbol]
        with self.assertRaisesRegex(ValueError, "Dangling"):
            validate_structure(self.document)

    def test_constant_must_match_source_initializer(self):
        symbol = next(value for value in self.document["symbols"].values() if "constant_value" in value)
        symbol["constant_value"] = "invented"
        with self.assertRaisesRegex(ValueError, "Constant differs"):
            validate_structure(self.document)

    def test_mutable_source_field_cannot_claim_constant(self):
        symbol = next(value for value in self.document["symbols"].values() if "constant_value" in value)
        symbol["source_immutable"] = False
        with self.assertRaisesRegex(ValueError, "mutable"):
            validate_structure(self.document)

    def test_truncation_rejected(self):
        self.document["methods"].pop()
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            validate_structure(self.document)

    def test_duplicate_identity_rejected(self):
        method = self.document["methods"][0]
        parameter = method["fields"]["parameters"][0]
        parameter["id"] = method["id"]
        with self.assertRaisesRegex(ValueError, "duplicated AST"):
            validate_structure(self.document)

    def test_source_span_required(self):
        self.document["methods"][0]["span"] = None
        with self.assertRaisesRegex(ValueError, "source span"):
            validate_structure(self.document)

    def test_serialization_ignores_map_iteration_order(self):
        shuffled = {key: self.document[key] for key in reversed(self.document)}
        self.assertEqual(encode(self.document), encode(shuffled))


@unittest.skipUnless(os.environ.get("PILOT_XTEND_INTEGRATION") == "1", "Set PILOT_XTEND_INTEGRATION=1 for real Xtend parser tests")
class RealParserTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="mercurio-xtend-")
        cls.root = Path(cls.temp.name)
        cls.classes = cls.root / "classes"
        cls.classes.mkdir()
        cls.java = Path(os.environ["JAVA_BIN"])
        cls.suffix = ".exe" if os.name == "nt" else ""
        jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
        xtend = Path.home() / f".m2/repository/org/eclipse/xtend/org.eclipse.xtend.core/{XTEND_VERSION}/org.eclipse.xtend.core-{XTEND_VERSION}.jar"
        cls.dependencies = os.pathsep.join(map(str, [jar, xtend]))
        subprocess.run([str(cls.java / ("javac" + cls.suffix)), "-encoding", "UTF-8", "-cp", cls.dependencies, "-d", str(cls.classes), str(HELPER)], check=True, capture_output=True, text=True)
        cls.source = (ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation" / SOURCE).read_text(encoding="utf-8")

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def export(self, source, success=True, sysml_mutation=None, selection=None):
        for other in SELECTION:
            if other != SOURCE:
                target = self.root / other
                target.parent.mkdir(parents=True, exist_ok=True)
                content = (ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation" / other).read_text(encoding="utf-8")
                if sysml_mutation:
                    content = sysml_mutation(content)
                target.write_text(content, encoding="utf-8", newline="")
        path = self.root / SOURCE
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8", newline="")
        output = self.root / "result.json"
        command = [str(self.java / ("java" + self.suffix)), "-Xmx2g", "-cp", str(self.classes) + os.pathsep + self.dependencies, "dev.mercurio.pilot.PilotXtendExporter", str(self.root), str(output)]
        if selection is not None:
            selected = self.root / "selection.json"
            selected.write_bytes(encode({source: sorted(names) for source, names in selection.items()}))
            command.append(str(selected))
        run = subprocess.run(command, capture_output=True, text=True)
        if success:
            self.assertEqual(run.returncode, 0, run.stderr)
            document = json.loads(output.read_text(encoding="utf-8"))
            validate_structure(document, selection)
            return document
        self.assertNotEqual(run.returncode, 0)
        return run.stderr

    def test_sysml_predicate_mutation_changes_resolved_operator(self):
        document = self.export(self.source, sysml_mutation=lambda text: text.replace("if (!defn.isVariation)", "if (defn.isVariation)"))
        original = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))
        def unary_count(value):
            if isinstance(value, dict):
                return int(value.get("kind") == "XUnaryOperation") + sum(unary_count(v) for v in value.values())
            if isinstance(value, list):
                return sum(map(unary_count, value))
            return 0
        select = lambda doc: next(m for m in doc["methods"] if m["name"] == "checkEnumerationDefinition")
        self.assertEqual(unary_count(select(original)), 1)
        self.assertEqual(unary_count(select(document)), 0)

    def test_helper_body_mutation_is_exported_without_predicate_reconstruction(self):
        from check_bounded_translation import SCOPE, AST, nodes
        scope = json.loads(SCOPE.read_text(encoding="utf-8"))
        selected = {source: set(names) for source, names in SELECTION.items()}
        for identity in scope["checks"] + scope["helpers"]:
            if identity.startswith("org.omg.sysml.xtext.validation.SysMLValidator#"):
                selected[SYSML_SOURCE].add(identity.split("#")[1])
        changed = self.export(self.source, selection=selected,
            sysml_mutation=lambda text: text.replace("types.length == 1", "types.length == 2"))
        original = json.loads(AST.read_text(encoding="utf-8"))
        def values(doc):
            helper = next(m for m in doc["methods"] if m["name"] == "checkOneType")
            return [n["fields"]["value"] for n in nodes(helper) if n["kind"] == "XNumberLiteral"]
        self.assertEqual(values(original), ["1"])
        self.assertEqual(values(changed), ["2"])
        self.assertEqual(len(changed["methods"]), 12)

    def test_identity_operator_is_resolved_from_source(self):
        old = "import_.visibility !== VisibilityKind.PRIVATE"
        self.assertIn(old, self.source)
        document = self.export(self.source.replace(old, "import_.visibility != VisibilityKind.PRIVATE"))
        self.assertIn("org.eclipse.xtext.xbase.lib.ObjectExtensions.operator_notEquals(java.lang.Object,java.lang.Object)", document["symbols"])

    def test_changed_constant_is_parsed_not_hardcoded(self):
        document = self.export(self.source.replace("Top level import must be private", "Changed diagnostic from source"))
        symbol = document["symbols"]["org.omg.kerml.xtext.validation.KerMLValidator.INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG"]
        self.assertEqual(symbol["constant_value"], "Changed diagnostic from source")

    def test_mutable_static_field_is_not_folded(self):
        before = "public static val INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG"
        self.assertIn(before, self.source)
        document = self.export(self.source.replace(before, "public static var INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG"))
        symbol = document["symbols"]["org.omg.kerml.xtext.validation.KerMLValidator.INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG"]
        self.assertTrue(symbol["static"])
        self.assertTrue(symbol["source_static"])
        self.assertFalse(symbol["final"])
        self.assertFalse(symbol["source_immutable"])
        self.assertIn("var", symbol["source_modifiers"])
        self.assertNotIn("constant_value", symbol)
        self.assertEqual(symbol["initializer"]["fields"]["value"], "Top level import must be private")

    def test_unresolved_property_fails_closed(self):
        error = self.export(self.source.replace("ann.ownedAnnotatingElement", "ann.nonexistentPropertyForTest"), success=False)
        self.assertRegex(error, "Unresolved|diagnostic|invalid feature")

    def test_invalid_xtend_syntax_fails(self):
        error = self.export(self.source.replace("def checkImport(Import import_)", "def checkImport(Import import_"), success=False)
        self.assertIn("Xtend parse failed", error)


if __name__ == "__main__":
    unittest.main()
