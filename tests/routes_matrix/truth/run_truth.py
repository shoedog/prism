#!/usr/bin/env python3
"""Execute lane-authored routes fixtures with the pinned local dependencies."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from stage import EnvironmentError, HandRequired, HERE, Refusal, methods, stage

DEFAULT = Path.home() / "prism-evidence" / "routes" / "truth"
PYTHON_BOOT = "import runpy, sys; runpy.run_path(sys.argv.pop(1))['python_child'](*sys.argv[1:])"


def read_rows(path):
    rows = {}
    with path.open(encoding="utf-8") as stream:
        for number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            name = row.get("id") if isinstance(row, dict) else None
            if not isinstance(name, str) or not name or name in rows:
                raise Refusal(f"{path}:{number}: missing or duplicate row id: {name!r}")
            if row.get("truth_source") not in {"executed", "hand"}:
                raise Refusal(f"{path}:{number}: invalid truth_source")
            rows[name] = row
    return rows


def normalize(framework, path):
    if framework == "flask":
        path = re.sub(r"<(?:(?:string|int|float|uuid):)?[a-zA-Z_]\w*>", "{}", path)
        invalid = r"[<>]"
    elif framework == "fastapi":
        path = re.sub(r"\{[a-zA-Z_]\w*(?::(?:str|int|float|uuid))?\}", "{}", path)
        invalid = r"[{}]"
        return None if re.search(invalid, path.replace("{}", "")) else path
    else:
        if re.search(r"[?*+()[\]\\|^$]", path):
            return None
        path = re.sub(r":[a-zA-Z0-9_]+", "{}", path)
        invalid = r":"
    return None if re.search(invalid, path) else path


def project(result):
    exposures = {}
    for route in result["served"]:
        if not isinstance(route["path"], str):
            continue
        for call_id in route["call_ids"]:
            exposures.setdefault(call_id, []).append(route["path"])

    records = []
    for call in result["calls"]:
        if call["mounted"] or call["kind"] in {"route", "use"}:
            continue
        site = call["site"]
        record = {
            "call_id": call["id"], "framework": result["framework"],
            "site": {"file": site["file"], "line": site["line"]},
            "paths": [],
        }
        if call.get("path") is not None:
            record["path"] = call["path"]
        declared = call["args"].get("methods")
        if declared == "ANY":
            record["methods"] = {"any": True}
        elif declared is not None:
            record["methods"] = {"known": methods(declared)}
        for path in dict.fromkeys(exposures.get(call["id"], [])):
            exposure = {"status": "full", "value": path}
            normalized = normalize(result["framework"], path)
            if normalized is not None:
                exposure["normalized"] = normalized
            record["paths"].append(exposure)
        callbacks = []
        for position, identity in enumerate(call["callbacks"]):
            if identity["kind"] not in {"python", "javascript"} or identity["file"].startswith("<"):
                break
            callback = {
                "status": "anonymous" if identity.get("anonymous") else "resolved",
                "position": position, "file": identity["file"], "name": identity["name"],
                "start_line": identity["first_line"],
            }
            if "end_line" in identity:
                callback["end_line"] = identity["end_line"]
            callbacks.append(callback)
        if len(callbacks) == len(call["callbacks"]):
            record["callbacks"] = callbacks
        records.append(record)
    return records


def execute(row, root, python, node_modules):
    try:
        with tempfile.TemporaryDirectory(prefix="routes-truth-") as temporary:
            work = Path(temporary).resolve()
            runtime, cwd, entry = stage(root, row, work, node_modules)
            home = work / "home"
            home.mkdir()
            scratch = work / "tmp"
            scratch.mkdir()
            env = {"PATH": os.environ.get("PATH", os.defpath), "HOME": str(home), "TMPDIR": str(scratch)}
            if runtime == "python":
                command = [str(python), "-I", "-B", "-c", PYTHON_BOOT, str(HERE / "stage.py")]
            else:
                adapter = work / "express_routes.mjs"
                adapter.write_bytes((HERE / adapter.name).read_bytes())
                command = ["node", "--expose-internals", "--no-global-search-paths", str(adapter)]
            try:
                child = subprocess.run(
                    [*command, str(cwd), entry], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                    capture_output=True, encoding="utf-8", timeout=10,
                )
            except (OSError, subprocess.TimeoutExpired) as exc:
                message = "child exceeded 10 s" if isinstance(exc, subprocess.TimeoutExpired) else str(exc)
                raise EnvironmentError(message) from exc
            if child.returncode != 0:
                raise EnvironmentError(f"child exited {child.returncode}: {child.stderr.strip()}")
            try:
                result = json.loads(child.stdout)
                if result["status"] == "executed":
                    result["projection"] = project(result)
                elif result["status"] not in {"hand_required", "error", "environment_error"}:
                    raise ValueError("unexpected child status")
            except (KeyError, TypeError, ValueError) as exc:
                raise EnvironmentError(f"bad child JSON: {exc}") from exc
            if "error" in result:
                result["error"] = result["error"].replace(str(work), "<stage>")
            return result
    except HandRequired as exc:
        return {"status": "hand_required", "error": str(exc)}
    except EnvironmentError as exc:
        return {"status": "environment_error", "error": str(exc)}
    except (Refusal, OSError) as exc:
        return {"status": "error", "error": str(exc)}


def tree_sha256(directory):
    """SHA-256 over the row's files: each relative path, NUL, bytes, NUL, in path order (the Rust test mirrors this)."""
    digest = hashlib.sha256()
    for path in sorted((p for p in directory.rglob("*") if p.is_file()), key=lambda p: p.relative_to(directory).as_posix()):
        digest.update(path.relative_to(directory).as_posix().encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--only", nargs="+", metavar="ID")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--verify-sources", action="store_true", help="also probe hand rows for check.sh")
    args = parser.parse_args()
    try:
        rows = read_rows(args.rows)
        root = args.root.resolve(strict=True)
        if not root.is_dir():
            raise Refusal("fixture root must be a directory")
        selected = set(args.only) if args.only else set(rows)
        if selected - rows.keys():
            raise Refusal(f"unknown row IDs: {', '.join(sorted(selected - rows.keys()))}")
        python = Path(os.environ.get("ROUTES_TRUTH_VENV", DEFAULT / "venv/bin/python")).expanduser().absolute()
        if python.is_dir():
            python /= "bin/python"
        node_modules = Path(os.environ.get("ROUTES_TRUTH_NODE", DEFAULT / "node/node_modules")).expanduser().absolute()
        failed = False
        with args.out.open("w", encoding="utf-8") as output:
            for name, row in rows.items():
                if name not in selected or (row["truth_source"] == "hand" and not args.verify_sources):
                    continue
                result = execute(row, root, python, node_modules)
                status = result["status"]
                if row["truth_source"] == "executed":
                    if status == "executed":
                        result["fixture_sha256"] = tree_sha256(root / row["dir"])
                    print(json.dumps({"id": name, **result}, sort_keys=True, allow_nan=False), file=output)
                    if status != "executed":
                        print(f"{name}: {status}: {result['error']}", file=sys.stderr)
                        failed = True
                elif status != "hand_required" and not (
                        status == "error" and str(result.get("error", "")).startswith("untraced ")):
                    # an untraced registration is a shape the runner cannot observe: the hand label is justified
                    message = "hand row can execute" if status == "executed" else result["error"]
                    print(f"{name}: {message}", file=sys.stderr)
                    failed = True
        return int(failed)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))


if __name__ == "__main__":
    sys.exit(main())
