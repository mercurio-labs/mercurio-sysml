import unittest
from qualify_definition_pipeline import inventory, inputs, check_test_output, REQUIRED_TESTS

class QualificationTests(unittest.TestCase):
    def passing(self):
        return "\n".join("test test_module::"+name+" ... ok" for name in sorted(REQUIRED_TESTS))+f"\ntest result: ok. {len(REQUIRED_TESTS)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"

    def test_pinned_inventory_has_exact_nonoverlapping_identities(self):
        rows = inventory()
        self.assertEqual([len(rows[k]) for k in ['stored_features','derived_features','operations']], [87,328,70])
        self.assertFalse(set(rows['stored_features']) & set(rows['derived_features']))

    def test_fingerprints_include_build_scripts_resources_and_embedded_controls(self):
        fingerprints = inputs()
        for relative in [
            "mercurio-foundation/Cargo.toml",
            "mercurio-foundation/Cargo.lock",
            "mercurio-sysml/crates/mercurio-sysml/build.rs",
            "mercurio-sysml/crates/mercurio-sysml/resources/kernel/kerml-mappings.overlay.json",
            "mercurio-sysml/docs/conformance/2026-08-support/expression-model-pilot-controls.json",
        ]:
            self.assertIn(relative, fingerprints)
        self.assertNotIn("mercurio-sysml/docs/conformance/2026-08-support/definition-pipeline-qualification.json", fingerprints)
        self.assertFalse(any("definition-pipeline-evidence/" in p for p in fingerprints))

    def test_complete_named_tests_are_required(self):
        self.assertEqual(len(check_test_output(self.passing())), len(REQUIRED_TESTS))
        with self.assertRaisesRegex(ValueError, 'Missing passing'):
            check_test_output(self.passing().replace(' ... ok', ' ... ignored', 1))

    def test_filtered_or_failed_run_cannot_qualify(self):
        for old, new in [('0 filtered out','1 filtered out'),('0 ignored','1 ignored'),('0 failed','1 failed')]:
            with self.assertRaises(ValueError): check_test_output(self.passing().replace(old,new))

    def test_counter_must_match_individual_test_outcomes(self):
        with self.assertRaises(ValueError): check_test_output(self.passing().replace(str(len(REQUIRED_TESTS))+' passed','999 passed'))

if __name__ == '__main__': unittest.main()
