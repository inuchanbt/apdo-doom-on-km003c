# SPDX-License-Identifier: MIT
"""Regression: public source preparation never depends on a device backup."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location("prepare",Path(__file__).with_name("prepare-sources.py"))
prepare=importlib.util.module_from_spec(spec);spec.loader.exec_module(prepare)
class IndependentPreparation(unittest.TestCase):
    def test_only_engine_sources_are_prepared_without_local_device_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            lock={"DMC100-Doom":{"url":"engine-url","revision":"engine-rev"},
                  "doomgeneric":{"url":"converter-url","revision":"converter-rev"},
                  "km003c-firmware":{"url":"must-not-fetch","revision":"unused"}}
            (root/"SOURCE_LOCK.json").write_text(json.dumps(lock))
            with patch.object(prepare,"ROOT",root),patch.object(sys,"argv",["prepare-sources.py"]), \
                 patch.object(prepare,"ensure_repo") as repo,patch.object(prepare,"apply_patch") as apply:
                prepare.main()
            self.assertEqual(repo.call_count,2)
            self.assertEqual([c.args[1] for c in repo.call_args_list],["engine-url","converter-url"])
            self.assertEqual(apply.call_count,1)
            self.assertEqual(apply.call_args.args[1].name,"dmc100-doom.patch")
            self.assertFalse((root/"local").exists())
    def test_wrong_revision_is_rejected_without_reset_or_checkout(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo=Path(tmp);(repo/".git").mkdir()
            import subprocess
            result=subprocess.CompletedProcess([],0,"different\n","")
            with patch.object(prepare,"git",return_value=result) as git:
                with self.assertRaisesRegex(RuntimeError,"Revision mismatch"):
                    prepare.ensure_repo(repo,"url","expected")
                git.assert_called_once_with(repo,"rev-parse","HEAD")
if __name__=="__main__":unittest.main()
