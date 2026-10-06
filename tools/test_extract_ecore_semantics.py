"""Focused checks for source identity, semantic preservation and drift detection."""
import copy
from pathlib import Path
import tempfile
import unittest

from extract_ecore_semantics import (
    ECORE, GENMODEL, SYSML_DELEGATE, bind_delegates, extract_model,
    extract_registrations, inventory_delegates, render, write_or_check,
)


def model(body, annotations="", namespace="urn:root", prefix="ecore"):
    return (
        f'<{prefix}:EPackage xmlns:{prefix}="{ECORE}" '
        'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" '
        f'name="root" nsURI="{namespace}">{annotations}{body}</{prefix}:EPackage>'
    ).encode("utf-8")


def registration():
    return (
        f'<eAnnotations source="{ECORE}">'
        f'<details key="settingDelegates" value="{SYSML_DELEGATE} urn:other"/>'
        f'<details key="invocationDelegates" value="{SYSML_DELEGATE}"/>'
        '</eAnnotations>'
    )


def marker(uri=SYSML_DELEGATE):
    return f'<eAnnotations source="{uri}"/>'


class ExtractEcoreSemanticsTests(unittest.TestCase):
    def test_structural_attributes_and_unknown_children_are_lossless(self):
        data = model("""
          <eClassifiers xsi:type="ecore:EClass" name="A" abstract="true" eSuperTypes="#//B">
            <eStructuralFeatures xsi:type="ecore:EReference" name="items" eType="#//B"
              lowerBound="1" upperBound="-1" containment="true" eOpposite="#//B/owner"
              ordered="false" unique="false" changeable="false" unsettable="true"
              derived="true" transient="true" volatile="true" resolveProxies="false"
              defaultValueLiteral="custom"><futureExtension value="preserve"/></eStructuralFeatures>
          </eClassifiers><eClassifiers xsi:type="ecore:EClass" name="B"/>
        """)
        result = extract_model(data)
        import xml.etree.ElementTree as ET
        from extract_ecore_semantics import raw_xml
        self.assertEqual(result["source_tree"], raw_xml(ET.fromstring(data)))
        feature = next(r for r in result["elements"] if r["name"] == "items")
        attrs = feature["source_attributes"]
        self.assertEqual(attrs["eOpposite"], "#//B/owner")
        self.assertEqual(attrs["upperBound"], "-1")
        self.assertEqual(attrs["ordered"], "false")
        self.assertEqual(attrs["defaultValueLiteral"], "custom")
        self.assertIn("futureExtension", str(result["source_tree"]))
        plain = next(r for r in result["elements"] if r["name"] == "B")
        self.assertNotIn("abstract", plain["source_attributes"])

    def test_namespaces_overloads_and_ordered_parameter_metadata(self):
        data = model("""
          <eClassifiers xsi:type="alternate:EClass" name="A">
            <eOperations name="resolve" eType="#//A" upperBound="-1">
              <eParameters name="name" eType="ecore:EDataType urn:types#//String"/>
              <eParameters name="excluded" eType="#//A" lowerBound="1"/>
            </eOperations>
            <eOperations name="resolve" eType="#//A">
              <eParameters name="name" eType="ecore:EDataType urn:types#//Integer"/>
            </eOperations>
          </eClassifiers>
          <eSubpackages name="nested" nsURI="urn:nested">
            <eClassifiers xsi:type="alternate:EClass" name="A"/>
          </eSubpackages>
        """, prefix="alternate")
        result = extract_model(data)
        classes = [row for row in result["elements"] if row["kind"] == "EClass"]
        self.assertEqual({row["id"] for row in classes}, {"urn:root#//A", "urn:nested#//A"})
        operations = [row for row in result["elements"] if row["kind"] == "EOperation"]
        self.assertEqual(len(operations), 2)
        self.assertEqual(len({row["id"] for row in operations}), 2)
        two_arg = next(row for row in operations if "excluded" in str(row["definition"]))
        self.assertIn("urn:types#//String,urn:root#//A", two_arg["signature"])
        self.assertEqual(two_arg["definition"]["attributes"]["upperBound"], "-1")
        self.assertEqual(
            [child["attributes"]["name"] for child in two_arg["definition"]["children"]],
            ["name", "excluded"],
        )

    def test_documentation_formulas_and_unknown_annotations_stay_raw(self):
        data = model(f"""
          <eClassifiers xsi:type="ecore:EClass" name="A">
            <eAnnotations source="{GENMODEL}">
              <details key="documentation" value="&lt;p&gt;Constraint&lt;/p&gt;&#10;x = y-&gt;first()"/>
            </eAnnotations>
            <eAnnotations source="urn:unknown" references="#//A">
              <details key="body" value="x = 2"/>
            </eAnnotations>
          </eClassifiers>
        """)
        result = extract_model(data)
        entry = next(row for row in result["elements"] if row["kind"] == "EClass")
        doc, unknown = entry["annotations"]
        self.assertEqual(doc["interpretation"], "documentation_only_not_executable")
        self.assertEqual(doc["children"][0]["attributes"]["value"],
                         "<p>Constraint</p>\nx = y->first()")
        self.assertEqual(unknown["interpretation"], "uninterpreted_annotation_not_executed")
        self.assertEqual(unknown["attributes"]["references"], "#//A")
        self.assertNotIn("executable", result)

    def test_duplicate_classifiers_features_operations_and_parameters_fail(self):
        cases = [
            '<eClassifiers name="A"/><eClassifiers name="A"/>',
            '<eClassifiers name="A"><eStructuralFeatures name="x"/>'
            '<eStructuralFeatures name="x"/></eClassifiers>',
            '<eClassifiers name="A"><eOperations name="x"/>'
            '<eOperations name="x"/></eClassifiers>',
            '<eClassifiers name="A"><eOperations name="x">'
            '<eParameters name="p"/><eParameters name="p"/></eOperations></eClassifiers>',
            '<eSubpackages name="a" nsURI="urn:x"/><eSubpackages name="b" nsURI="urn:x"/>',
        ]
        for body in cases:
            with self.subTest(body=body), self.assertRaisesRegex(ValueError, "duplicate"):
                extract_model(model(body))

    def test_duplicate_annotation_details_fail(self):
        with self.assertRaisesRegex(ValueError, "duplicate annotation detail"):
            extract_model(model(
                '<eAnnotations source="urn:x"><details key="k"/><details key="k"/></eAnnotations>'
            ))

    def test_generic_operation_signatures_preserve_bounds_and_parameters(self):
        result = extract_model(model("""
          <eClassifiers xsi:type="ecore:EClass" name="A">
            <eOperations name="map" eExceptions="urn:types#//Failure">
              <eTypeParameters name="T"><eBounds eClassifier="#//A"/></eTypeParameters>
              <eGenericType eTypeParameter="#//A/map/T"/>
              <eParameters name="items" upperBound="-1">
                <eGenericType eClassifier="urn:types#//List">
                  <eTypeArguments eTypeParameter="#//A/map/T"/>
                </eGenericType>
              </eParameters>
            </eOperations>
          </eClassifiers>
        """))
        operation = next(row for row in result["elements"] if row["kind"] == "EOperation")
        self.assertIn("urn:types#//List", operation["signature"])
        self.assertEqual(operation["definition"]["attributes"]["eExceptions"], "urn:types#//Failure")
        self.assertIn("eBounds", str(operation["definition"]))
        self.assertIn("eTypeArguments", str(operation["definition"]))

    def test_delegate_markers_custom_fallback_dynamic_and_unknown_bindings(self):
        data = extract_model(model(f"""
          <eClassifiers xsi:type="ecore:EClass" name="A">
            <eStructuralFeatures xsi:type="ecore:EReference" name="direct" derived="true">
              {marker()}
            </eStructuralFeatures>
            <eStructuralFeatures xsi:type="ecore:EReference" name="fallback" derived="true">
              {marker()}
            </eStructuralFeatures>
            <eStructuralFeatures xsi:type="ecore:EReference" name="unknown">
              {marker("urn:other")}
            </eStructuralFeatures>
            <eStructuralFeatures xsi:type="ecore:EReference" name="noMarker" derived="true"/>
            <eOperations name="run">{marker()}</eOperations>
          </eClassifiers>
          <eClassifiers xsi:type="ecore:EClass" name="B" eSuperTypes="#//A"/>
          <eClassifiers xsi:type="ecore:EClass" name="Unrelated"/>
        """, registration()))
        sources = [
            {"class": "p.A_direct_SettingDelegate", "kind": "setting",
             "filename_convention": {"classifier": "A", "member": "direct"}},
            {"class": "p.A_run_InvocationDelegate", "kind": "invocation",
             "filename_convention": {"classifier": "A", "member": "run"}},
            {"class": "p.B_run_InvocationDelegate", "kind": "invocation",
             "filename_convention": {"classifier": "B", "member": "run"}},
            {"class": "p.Unrelated_run_InvocationDelegate", "kind": "invocation",
             "filename_convention": {"classifier": "Unrelated", "member": "run"}},
        ]
        bindings = bind_delegates(data, sources)
        self.assertEqual(len(bindings), 4)
        direct = next(row for row in bindings if row["element"].endswith("/direct"))
        self.assertEqual(direct["candidate_source_classes"], ["p.A_direct_SettingDelegate"])
        fallback = next(row for row in bindings if row["element"].endswith("/fallback"))
        self.assertEqual(fallback["binding_status"], "default_setting_delegate_fallback_source")
        unknown = next(row for row in bindings if row["element"].endswith("/unknown"))
        self.assertEqual(unknown["binding_status"], "marker_only_unresolved_factory")
        invocation = next(row for row in bindings if row["kind"] == "invocation")
        self.assertEqual(invocation["binding_status"], "dynamic_invocation_candidates_not_selected")
        self.assertEqual(invocation["candidate_source_classes"],
                         ["p.A_run_InvocationDelegate", "p.B_run_InvocationDelegate"])

    def test_invocation_candidates_follow_ecore_fragments_across_package_namespaces(self):
        data = extract_model(model(f"""
          <eClassifiers xsi:type="ecore:EClass" name="A">
            <eOperations name="run">{marker()}</eOperations>
          </eClassifiers>
          <eSubpackages name="nested" nsURI="urn:alphabetically-before-root">
            <eClassifiers xsi:type="ecore:EClass" name="B" eSuperTypes="#//A"/>
            <eClassifiers xsi:type="ecore:EClass" name="C" eSuperTypes="#//nested/B"/>
          </eSubpackages>
        """, registration(), namespace="urn:z-root"))
        sources = [
            {"class": "p.C_run_InvocationDelegate", "kind": "invocation",
             "filename_convention": {"classifier": "C", "member": "run"}},
        ]
        bindings = bind_delegates(data, sources)
        self.assertEqual(bindings[0]["candidate_source_classes"], ["p.C_run_InvocationDelegate"])

    def test_delegate_inventory_and_plugin_registration_retain_source_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting"
            directory.mkdir(parents=True)
            source = directory / "A_value_SettingDelegate.java"
            source.write_text("class A_value_SettingDelegate {}\n", encoding="utf-8")
            first = inventory_delegates(root)
            self.assertEqual(len(first), 1)
            self.assertEqual(first[0]["filename_convention"], {"classifier": "A", "member": "value"})
            self.assertEqual(first[0]["algorithm_status"], "source_inventoried_not_translated_or_executed")
            source.write_text("class A_value_SettingDelegate { int x; }\n", encoding="utf-8")
            self.assertNotEqual(first[0]["sha256"], inventory_delegates(root)[0]["sha256"])
            plugin = root / "plugin.xml"
            plugin.write_text(
                '<plugin><extension point="org.eclipse.emf.ecore.setting_delegate">'
                '<factory uri="urn:delegate" class="p.Factory"/></extension>'
                '<extension point="unrelated"><factory/></extension></plugin>', encoding="utf-8"
            )
            registrations = extract_registrations(plugin)
            self.assertEqual(len(registrations), 1)
            self.assertEqual(registrations[0]["declarations"][0]["attributes"],
                             {"uri": "urn:delegate", "class": "p.Factory"})

    def test_deterministic_output_and_check_detect_semantic_or_source_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "extract.json"
            document = {"source": {"sha256": "old"}, **extract_model(model(
                '<eClassifiers xsi:type="ecore:EClass" name="A"/>'
            ))}
            write_or_check(document, path, False)
            self.assertEqual(path.read_bytes(), render(document))
            write_or_check(document, path, True)
            for changed in (
                {**document, "source": {"sha256": "new"}},
                {**document, "elements": []},
            ):
                with self.assertRaisesRegex(ValueError, "drift"):
                    write_or_check(changed, path, True)
            self.assertEqual(render(document), render(copy.deepcopy(document)))
            with self.assertRaisesRegex(ValueError, "drift"):
                write_or_check(document, Path(temporary) / "missing.json", True)


if __name__ == "__main__":
    unittest.main()

