"""Hermetic CLI and guarded-product fixtures for the measurement kit."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

MEASURE = Path(__file__).resolve().parents[1]
REPO = MEASURE.parents[1]
DEFAULT_VERSION = "9d1a7d06f653"


def sha256_of(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_tree(root, files):
    for name, value in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value if isinstance(value, bytes) else value.encode())


def fixture_digest(root):
    """Independent small-fixture implementation of the §2 records."""
    records = []
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        if ".git" in relative.parts:
            continue
        if path.is_symlink():
            kind, value = b"L", os.fsencode(os.readlink(path))
        elif path.is_file():
            kind, value = b"F", path.read_bytes()
        else:
            continue
        name = os.fsencode(relative.as_posix())
        records.append((name, kind + len(name).to_bytes(8, "big") + name
                        + len(value).to_bytes(8, "big") + value))
    return hashlib.sha256(b"".join(record for _, record in sorted(records))).hexdigest()


def executable(path, body):
    path.write_text(f"#!{sys.executable}\n" + body, encoding="utf-8")
    path.chmod(0o755)
    return path


class Workbench(unittest.TestCase):
    def setUp(self):
        temporary = TemporaryDirectory(prefix=".measure-test-", dir=REPO)
        self.addCleanup(temporary.cleanup)
        self.work = Path(temporary.name)
        self.source = self.work / "corpus"
        self.out = self.work / "out"
        self.log = self.work / "product.jsonl"
        self.fake = self.work / "prism"
        self.manifest = self.work / "corpora.json"
        self.guard = self.work / "guard"
        self.home = self.work / "home"
        self.home.mkdir()
        self.env = dict(os.environ, HOME=str(self.home), GIT_CONFIG_NOSYSTEM="1",
                        GIT_CONFIG_GLOBAL=os.devnull, PYTHONDONTWRITEBYTECODE="1")
        self.env.pop("PRISM_MEASURE_CORPORA", None)
        self.key = "Z"
        self.make_corpus()
        self.make_fake()
        self.make_guard()

    def git(self, *args, env=None):
        return subprocess.check_output(["git", "-C", str(self.source), *args],
                                       env=env or self.env, stderr=subprocess.PIPE).decode().strip()

    def commit_files(self, files):
        write_tree(self.source, files)
        self.git("add", "-A")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                 "-c", "commit.gpgsign=false", "commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def make_corpus(self, *, git=True, files=None, links=None, ignore=None,
                    root=".", local=None, pin_tree=False):
        self.source.mkdir(exist_ok=True)
        files = dict({"a.ts": "export {};\n"} if files is None else files)
        if ignore is not None:
            files[".gitignore"] = ignore
        write_tree(self.source, files)
        for name, target in (links or {}).items():
            (self.source / name).symlink_to(target)
        self.commit = None
        if git:
            self.git("init", "-q")
            self.commit = self.commit_files({})
        self.entry = dict(name="Fixture", kind="git" if git else "tree",
                          url=None, commit=self.commit, root=root, history=False,
                          local=local or [self.source.name], manifest_sha256=None,
                          cloud_asset=None,
                          tree_sha256=fixture_digest(self.source / root) if pin_tree else None)
        self.write_manifest()

    def fresh_tree(self, **kwargs):
        self.source = self.work / "extracted"
        self.make_corpus(git=False, pin_tree=True, **kwargs)

    def write_manifest(self, entries=None):
        self.manifest.write_text(json.dumps(dict(schema="prism.measure.corpora/1",
                                                 corpora=entries if entries is not None
                                                 else {self.key: self.entry})), encoding="utf-8")

    def make_fake(self, cases=None, version=DEFAULT_VERSION, default_exit=0):
        return executable(self.fake, f"""
import json
import os
from pathlib import Path
import sys
args = sys.argv[1:]
command = ("version" if args == ["--version"] else
           "edges" if "--edges" in args else "sites" if "--dump-sites" in args else args[2])
root = Path(args[args.index("--repo") + 1]) if "--repo" in args else None
with open({str(self.log)!r}, "a", encoding="utf-8") as stream:
    print(json.dumps({{"argv": args, "env": dict(os.environ), "command": command}}), file=stream)
print({version!r} if command == "version" else '{{"count":1}}')
print(command, file=sys.stderr)
cases = {cases or {}!r}
exec(cases.get(command, ""))
sys.exit({default_exit})
""")

    def make_guard(self, responses=None):
        return executable(self.guard, f"""
import json
import os
import subprocess
import sys
assert sys.argv[1] == "--status-fd"
fd = int(sys.argv[2])
assert int(sys.argv[3]) > 0
args = sys.argv[4:]
command = ("version" if args[1:] == ["--version"] else
           "edges" if "--edges" in args else "sites" if "--dump-sites" in args else args[3])
responses = {responses or {}!r}
if command in responses:
    code, status = responses[command]
    if isinstance(status, dict):
        status = json.dumps(status) + "\\n"
else:
    code = subprocess.call(args)
    code = 128 - code if code < 0 else code
    status = json.dumps(dict(kind="child", launched=True, child_exit=code)) + "\\n"
if status is not None:
    os.write(fd, status.encode())
sys.exit(code)
""")

    def kit(self, *args):
        return subprocess.run([sys.executable, "-m", "measure", "--manifest",
                               str(self.manifest), *map(str, args)], cwd=MEASURE.parent,
                              env=self.env, capture_output=True, text=True, timeout=20)

    def run_kit(self, *args, binary=None, out=None):
        return self.kit("run", "--binary", binary or self.fake, "--corpus", self.key,
                        "--out", out or self.out, "--memguard", self.guard,
                        "--corpora-root", self.work, *args)

    def verify(self, *keys):
        return self.kit("corpora", "verify", "--corpora-root", self.work, *keys)

    def assert_exit(self, output, expected):
        self.assertEqual(output.returncode, expected, (output.stdout, output.stderr))
        self.assertNotIn("Traceback", output.stderr)

    def receipt(self):
        path = self.out / "receipt.json"
        return json.loads(path.read_bytes()) if path.is_file() else None

    def fake_log(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
