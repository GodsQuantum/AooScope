import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from aooscope.page_compiler import CompileResult
from aooscope.revisions import RevisionManager


def fake_compiler(doc, state, output_dir):
    return CompileResult(
        monitor_config={"setup":{"switchTime":"8","refresh":1},"mianban":[1],"diy":[{"id":"p","type":1,"img":"assets/page.jpg","sensor":[]}]},
        files=[],
        warnings=["sample warning"],
    )


class RevisionManagerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.manager = RevisionManager(self.root, keep=3)

    def tearDown(self):
        self.tmp.cleanup()

    def test_stage_promote_and_current_pointer(self):
        rid = self.manager.stage({"revision":2}, {}, fake_compiler)
        staged = self.root / "compiled" / rid
        self.assertTrue((staged / "monitor.json").is_file())
        self.manager.promote(rid)
        current = self.manager.current()
        self.assertEqual(current["revision_id"], rid)
        self.assertTrue((self.root / "compiled" / "current.json").is_file())
    def test_failed_promoted_revision_rolls_back_previous(self):
        first = self.manager.stage({"revision":1}, {}, fake_compiler)
        self.manager.promote(first)
        second = self.manager.stage({"revision":2}, {}, fake_compiler)
        self.manager.promote(second)
        self.assertEqual(self.manager.current()["revision_id"], second)

        restored = self.manager.mark_failed(second, "asterctl exited")
        self.assertEqual(restored["revision_id"], first)
        failed = json.loads((self.root / "compiled" / second / "failed.json").read_text())
        self.assertEqual(failed["reason"], "asterctl exited")

    def test_retention_keeps_current_previous_and_bounded_history(self):
        ids = []
        for revision in range(1, 7):
            rid = self.manager.stage({"revision":revision}, {}, fake_compiler)
            self.manager.promote(rid)
            ids.append(rid)
        dirs = [p.name for p in (self.root / "compiled").iterdir() if p.is_dir()]
        self.assertLessEqual(len(dirs), 3)
        self.assertIn(self.manager.current()["revision_id"], dirs)
        self.assertIn(self.manager.previous()["revision_id"], dirs)

    def test_supervisor_loads_promoted_revision(self):
        from aooscope.supervisor import DisplaySupervisor
        rid = self.manager.stage({"revision":1}, {}, fake_compiler)
        self.manager.promote(rid)
        with patch.dict(os.environ, {"AOOSCOPE_CONFIG_DIR_IN_CONTAINER": str(self.root)}, clear=False):
            supervisor = DisplaySupervisor()
            promoted = supervisor.load_promoted_revision()
        self.assertEqual(promoted["revision_id"], rid)
        self.assertEqual(Path(promoted["config_dir"]), self.root / "compiled" / rid)


if __name__ == "__main__":
    unittest.main()
