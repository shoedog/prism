#!/usr/bin/env python3
"""Execute the lane 10 fixture programs and record their observations."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile

from containment import Refused, check_source


HERE = Path(__file__).resolve().parent
MATRIX = HERE.parent
PRELUDES = {"py": "prelude.py", "js": "prelude.mjs", "go": "prelude.go"}
EXECUTED = {"py", "js", "ts", "go"}
LANGUAGES = EXECUTED | {"rs", "java", "c"}
TIMEOUT = 10


def load_jsonl(path):
    records = []
    with path.open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            try:
                record = json.loads(line)
                if not isinstance(record, dict):
                    raise ValueError("expected a JSON object")
            except ValueError as error:
                raise ValueError(f"{path}:{number}: {error}") from error
            records.append(record)
    if not records:
        raise ValueError(f"{path}: empty JSONL")
    return records


def load_rows(path):
    rows = load_jsonl(path)
    seen = set()
    for row in rows:
        for name in ("id", "lang", "file"):
            if not isinstance(row.get(name), str) or not row[name]:
                raise ValueError(f"{path}: missing or invalid {name} in {row!r}")
        if row["id"] in seen:
            raise ValueError(f"{path}: duplicate row ID {row['id']!r}")
        seen.add(row["id"])
        if row["lang"] not in LANGUAGES:
            raise ValueError(f"{path}: {row['id']}: unknown language {row['lang']!r}")
    return rows


def write_jsonl(path, records):
    # Publish only after the entire run has finished.
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as stream:
        pending = Path(stream.name)
        try:
            for record in records:
                stream.write(json.dumps(record, ensure_ascii=False) + "\n")
            stream.close()
            pending.replace(path)
        finally:
            if pending.exists():
                pending.unlink()


def load_preludes():
    preludes = {}
    for lang, name in PRELUDES.items():
        path = HERE / name
        source = path.read_text(encoding="utf-8")
        check_source(path, lang, source, prelude=True)
        preludes[lang] = source
    return preludes


def load_cases(directory, rows):
    root = directory.resolve(strict=True)
    cases = []
    for row in rows:
        path = root / row["file"]
        if Path(row["file"]).is_absolute() or not path.resolve().is_relative_to(root):
            raise Refused(f"{path}: case file must resolve inside {root}")
        source = path.read_text(encoding="utf-8")
        check_source(path, row["lang"], source)
        cases.append(source)
    return cases


def run_process(command, directory, env, timeout):
    # Kill the group on timeout so a go run child cannot outlive its launcher.
    with subprocess.Popen(
        command, cwd=directory, env=env, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8",
        errors="replace", start_new_session=True,
    ) as process:
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.communicate()
            raise
        return process.returncode, stdout, stderr


class Runner:
    def __init__(self, scratch):
        self.scratch = scratch
        self.path = os.environ.get("PATH", os.defpath)
        self.commands = {
            "node": shutil.which("node", path=self.path),
            "go": shutil.which("go", path=self.path),
        }
        self.go_ready = False
        self.versions = {}
        self.commands["python"], self.versions["python"] = self.python()
        with tempfile.TemporaryDirectory(dir=self.scratch) as directory:
            env = {"PATH": self.path, "HOME": directory}
            for name in ("node", "go"):
                command = self.commands[name]
                if command is None:
                    self.versions[name] = "unavailable"
                    continue
                if name == "go":
                    self.disable_go_telemetry(directory, env)
                argument = "--version" if name == "node" else "version"
                code, output, _ = run_process([command, argument], directory, env, TIMEOUT)
                self.versions[name] = output.strip() if code == 0 else "unavailable"

    def python(self):
        uv = shutil.which("uv", path=self.path)
        reason = "uv is unavailable"
        identity = "import sys; print(sys.executable); print(sys.version.split()[0])"
        if uv:
            command = [
                uv, "--offline", "--no-config", "run", "--no-project", "--no-python-downloads",
                "--python", "3.12", "python", "-I", "-S", "-c", identity,
            ]
            env = {**os.environ, "UV_CACHE_DIR": str(self.scratch / "uv-cache")}
            try:
                code, output, error = run_process(command, self.scratch, env, TIMEOUT)
                lines = output.splitlines()
                if code == 0 and len(lines) == 2 and lines[1].startswith("3.12."):
                    return lines
                reason = error.strip() or "uv did not report a Python 3.12 executable"
            except subprocess.TimeoutExpired:
                reason = "uv timed out"
        command = shutil.which("python3", path=self.path)
        if command is None:
            raise ValueError(f"Python 3.12 offline selection failed ({reason}); python3 is unavailable")
        print(f"Python 3.12 offline selection failed ({reason}); falling back to python3",
              file=sys.stderr)
        env = {"PATH": self.path, "HOME": str(self.scratch)}
        code, output, error = run_process(
            [command, "-I", "-S", "-c", identity], self.scratch, env, TIMEOUT
        )
        if code != 0:
            raise ValueError(f"python3 version probe failed: {error.strip()}")
        return output.splitlines()

    def execute(self, lang, source, prelude):
        with tempfile.TemporaryDirectory(dir=self.scratch) as working:
            directory = Path(working)
            home = directory / "home"
            home.mkdir()
            env = {"PATH": self.path, "HOME": str(home)}
            if lang == "py":
                entry = directory / "case.py"
                entry.write_text(prelude + "\n\n" + source, encoding="utf-8")
                command = [self.commands["python"], "-I", "-S", entry.name]
            elif lang in {"js", "ts"}:
                entry = directory / ("case.ts" if lang == "ts" else "case.mjs")
                entry.write_text(prelude + "\n\n" + source, encoding="utf-8")
                (directory / "package.json").write_text('{"type":"module"}\n', encoding="utf-8")
                command = [self.commands["node"], entry.name]
            else:
                (directory / "case.go").write_text(source, encoding="utf-8")
                (directory / "prelude.go").write_text(prelude, encoding="utf-8")
                (directory / "go.mod").write_text("module fixture\n\ngo 1.26.2\n", encoding="utf-8")
                env.update(
                    GOFLAGS="-mod=mod", GOCACHE=str(self.scratch / "go-cache"),
                    CGO_ENABLED="0", GOTOOLCHAIN="local",
                )
                command = [self.commands["go"], "run", "case.go", "prelude.go"]
                if command[0] is not None and not self.go_ready:
                    self.warm_go(env)
            if command[0] is None:
                return None, f"{lang} interpreter unavailable"
            try:
                if lang == "go":
                    self.disable_go_telemetry(directory, env)
                code, output, error = run_process(command, directory, env, TIMEOUT)
            except subprocess.TimeoutExpired:
                return None, f"timeout after {TIMEOUT} seconds"
            except (OSError, ValueError) as error:
                return None, str(error)
            if code != 0:
                detail = error.strip().splitlines()
                return None, f"exit {code}" + (f" ({detail[0]})" if detail else "")
            return output, None

    def disable_go_telemetry(self, directory, env):
        # Go otherwise starts a background writer that can race HOME cleanup.
        code, _, error = run_process(
            [self.commands["go"], "telemetry", "off"], directory, env, TIMEOUT
        )
        if code:
            raise ValueError(f"Go telemetry setup failed: {error.strip()}")

    def warm_go(self, env):
        # Populate only the standard-library cache before timing the first program.
        self.go_ready = True
        try:
            with tempfile.TemporaryDirectory(dir=self.scratch) as directory:
                build_env = {**env, "HOME": directory}
                self.disable_go_telemetry(directory, build_env)
                code, _, error = run_process(
                    [self.commands["go"], "build", "fmt", "reflect", "strings"],
                    directory, build_env, 60,
                )
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            print(f"Go standard-library cache setup: {error}", file=sys.stderr)
            return
        if code != 0:
            print(f"Go standard-library cache setup: {error.strip()}", file=sys.stderr)


ENTRY = {
    "py": "\n\ncase()\n",
    "js": "\n\ncaseFn();\n",
    "ts": "\n\ncaseFn();\n",
    "go": "\n\nfunc main() { caseFn() }\n",
}


def row_truth(runner, preludes, row, source):
    lang = row["lang"]
    if lang not in EXECUTED:
        return "n/a"
    prelude = preludes["js" if lang == "ts" else lang]
    output, error = runner.execute(lang, source + ENTRY[lang], prelude)
    if error is None:
        observations = [line for line in output.splitlines() if re.match(r"^OBSERVE(?:\s|$)", line)]
        if observations == ["OBSERVE reaches"]:
            return "reaches"
        if observations == ["OBSERVE not"]:
            return "not"
        error = f"expected one valid observation, received {observations!r}"
    print(f"{row['id']}: inadmissible ({error})", file=sys.stderr)
    return "inadmissible"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cases", type=Path, nargs="?")
    parser.add_argument("rows", type=Path, nargs="?")
    parser.add_argument("--spellings", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if args.spellings is not None:
        if args.cases is not None or args.rows is not None:
            parser.error("--spellings does not take cases or rows")
    elif args.cases is None or args.rows is None:
        parser.error("supply the cases directory and rows JSONL, or --spellings")

    try:
        preludes = load_preludes()
        if args.spellings is not None:
            from spelling_truth import load_spellings, run_spellings
            spellings = load_spellings(args.spellings)
        else:
            rows = load_rows(args.rows)
            cases = load_cases(args.cases, rows)

        with tempfile.TemporaryDirectory(prefix="subscript-truth-") as scratch:
            runner = Runner(Path(scratch))
            if args.spellings is not None:
                print(json.dumps({"versions": runner.versions}), file=sys.stderr)
                write_jsonl(args.out or HERE / "spelling_truth.jsonl",
                            run_spellings(runner, preludes, spellings))
            else:
                records = [{"versions": runner.versions}]
                records.extend(
                    {"id": row["id"], "truth": row_truth(runner, preludes, row, source)}
                    for row, source in zip(rows, cases)
                )
                write_jsonl(args.out or HERE / "truth.jsonl", records)
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        parser.exit(2, f"{error}\n")


if __name__ == "__main__":
    main()
