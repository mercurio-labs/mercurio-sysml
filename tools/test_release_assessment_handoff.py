"""Assessment handoff checks: graph files are not ready merely because they exist."""
import json
from pathlib import Path
import tempfile
import unittest

from run_release_assessment import wait_for_oracle


class OracleHandoffTests(unittest.TestCase):
    def test_consumer_waits_until_export_process_completed(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / 'pilot-semantic-timings.json').write_text('{}', encoding='utf-8')
            (directory / 'pilot-export.json').write_text('{"elements": [', encoding='utf-8')
            self.assertTrue(wait_for_oracle(directory, timeout=0.01)['timeout'])
            (directory / 'pilot-export.json').write_text('{"elements": []}', encoding='utf-8')
            (directory / 'oracle-execution.json').write_text(json.dumps({'returncode': 0}), encoding='utf-8')
            self.assertEqual(wait_for_oracle(directory)['returncode'], 0)

    def test_failed_export_does_not_reuse_partial_graph(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / 'pilot-semantic-timings.json').write_text('{}', encoding='utf-8')
            (directory / 'pilot-export.json').write_text('{', encoding='utf-8')
            (directory / 'oracle-execution.json').write_text(json.dumps({'returncode': 1}), encoding='utf-8')
            self.assertEqual(wait_for_oracle(directory)['returncode'], 1)


if __name__ == '__main__':
    unittest.main()
