import json
from pathlib import Path
import tempfile
import unittest

from check_clippy import diagnostics


class DiagnosticBudgetTests(unittest.TestCase):
    def report(self, items):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "report.jsonl"
            path.write_text("\n".join(json.dumps(item) for item in items), encoding="utf-8")
            return diagnostics(path)

    def test_duplicate_targets_do_not_double_the_budget(self):
        message = {"reason": "compiler-message", "message": {
            "level": "warning", "message": "same diagnostic",
            "code": {"code": "clippy::needless_borrow"},
            "spans": [{"file_name": "src\\canvas.rs", "line_start": 4,
                       "column_start": 1, "is_primary": True}]}}
        counts, errors = self.report([message, message, {"reason": "build-finished", "success": True}])
        self.assertEqual(dict(counts), {"src/canvas.rs | clippy::needless_borrow": 1})
        self.assertFalse(errors)

    def test_empty_truncated_and_failed_reports_are_not_clean(self):
        for items in [[], [{"reason": "build-finished", "success": False}],
                      [{"reason": "compiler-message", "message": {"level": "error", "message": "failed"}}]]:
            _, errors = self.report(items)
            self.assertTrue(errors)


if __name__ == "__main__":
    unittest.main()
