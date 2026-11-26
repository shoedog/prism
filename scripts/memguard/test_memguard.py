from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

HERE = Path(__file__).resolve().parent


class MemguardTests(unittest.TestCase):
    def setUp(self):
        temporary = TemporaryDirectory(prefix=".memguard-test-", dir=HERE.parents[1])
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "memguard").mkdir()
        (self.root / "hostquiet").mkdir()
        self.guard = self.root / "memguard" / "memguard.sh"
        shutil.copy2(HERE / "memguard.sh", self.guard)
        (self.root / "hostquiet" / "host_quiet.py").write_text(
            'import os\nprint(os.environ.get("FAKE_QUIET", "free"))\n', encoding="utf-8")
        self.log = self.root / "state" / "kills.log"
        self.child_pid = self.root / "child.pid"
        self.env = dict(os.environ, PATH=f"{self.root}{os.pathsep}{os.environ['PATH']}",
                        MEMGUARD_LOG=str(self.log), FAKE_RSS="10",
                        FAKE_CHILD_PID=str(self.child_pid))
        self.env.pop("MEMGUARD_IGNORE_QUIET", None)
        ps = self.root / "ps"
        ps.write_text(f"""#!{sys.executable}
import os
from pathlib import Path
import sys
import time
if "FAKE_PS_FAILURE" in os.environ:
    sys.exit(1)
if sys.argv[1] == "-o":
    print("10")
else:
    pid_file = Path(os.environ["FAKE_CHILD_PID"])
    for _ in range(100):
        if pid_file.is_file() and pid_file.stat().st_size:
            break
        time.sleep(0.005)
    if "FAKE_SAMPLE_FAILURE" in os.environ:
        sys.exit(1)
    print(pid_file.read_text(), 1, os.environ["FAKE_RSS"])
""", encoding="utf-8")
        ps.chmod(0o755)

    def guarded(self, command, *, limit="1", structured=True):
        read, write = os.pipe()
        try:
            opts = ["--status-fd", str(write)] if structured else []
            result = subprocess.run([str(self.guard), *opts, limit, *command],
                                    env=dict(self.env, TEST_GUARD_FD=str(write)),
                                    pass_fds=(write,) if structured else (),
                                    capture_output=True, timeout=12)
        finally:
            os.close(write)
        with os.fdopen(read, "rb") as stream:
            status = stream.read()
        return result, status

    def assert_status(self, result, raw, code, kind, launched, child_exit):
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertEqual(len(raw.splitlines()), 1, raw)
        self.assertTrue(raw.endswith(b"\n"))
        self.assertEqual(json.loads(raw), dict(kind=kind, launched=launched, child_exit=child_exit))

    def test_below_limit_keeps_child_exit_and_closes_status_fd(self):
        code = ("import os, sys, time\n"
                "from pathlib import Path\n"
                "Path(os.environ['FAKE_CHILD_PID']).write_text(str(os.getpid()))\n"
                "time.sleep(0.1)\n"
                "assert not ({'PWD', 'SHLVL', '_', 'MEMGUARD_LOG'} & os.environ.keys())\n"
                "try: os.fstat(int(os.environ['TEST_GUARD_FD']))\n"
                "except OSError: sys.exit(75)\n"
                "sys.exit(5)")
        result, raw = self.guarded([sys.executable, "-c", code])
        self.assert_status(result, raw, 75, "child", True, 75)
        self.assertFalse(self.log.exists())

    def test_marker_refusal(self):
        self.env["FAKE_QUIET"] = '{"stale": false}'
        marker = self.root / "launched"
        result, raw = self.guarded(["touch", str(marker)])
        self.assert_status(result, raw, 75, "host_quiet", False, None)
        self.assertFalse(marker.exists())

    def test_ps_failure_refuses(self):
        self.env["FAKE_PS_FAILURE"] = "1"
        marker = self.root / "launched"
        result, raw = self.guarded(["touch", str(marker)])
        self.assert_status(result, raw, 76, "rss_unavailable", False, None)
        self.assertFalse(marker.exists())

    def test_above_limit_reaps_child_and_logs_once(self):
        self.env["FAKE_RSS"] = str(2 * 1024 * 1024)
        result, raw = self.guarded(self.sleeping_child())
        self.assert_status(result, raw, 137, "rss_limit", True, 143)
        self.assertEqual(len(self.log.read_text().splitlines()), 1)

    def test_ps_failure_after_launch_reaps_child(self):
        self.env["FAKE_SAMPLE_FAILURE"] = "1"
        result, raw = self.guarded(self.sleeping_child())
        self.assert_status(result, raw, 76, "rss_unavailable", True, 143)
        self.assertFalse(self.log.exists())

    def sleeping_child(self):
        return ["sh", "-c", 'printf "%s" "$$" > "$1"; exec sleep 8', "test", str(self.child_pid)]

    def test_bad_limit_is_guard_error(self):
        result, raw = self.guarded(["true"], limit="0")
        self.assert_status(result, raw, 1, "guard_error", False, None)

    def test_standalone_preserves_raw_exit(self):
        result, raw = self.guarded(["sh", "-c", "exit 76"], structured=False)
        self.assertEqual(result.returncode, 76, result.stderr)
        self.assertEqual(raw, b"")
