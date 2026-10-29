#!/usr/bin/env python3
"""Paired prism timing runs for a cloud VM: build refs, rotate runs, write RESULTS.md."""
import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import fetch_corpora  # noqa: E402
import reporting  # noqa: E402
from summarize_profile import load_records  # noqa: E402

DEFAULT_QUERY = "nav --no-cache dfg-stats"
REPO_ROOT = Path(__file__).resolve().parents[3]


def run_order(refs, corpora, rounds):
    """Per round, per corpus, every ref; round r starts at ref r mod k."""
    k = len(refs)
    for r in range(rounds):
        for corpus in corpora:
            for i in range(k):
                yield r, corpus, refs[(r + i) % k]


def rss_bytes(value, system=None):
    """ru_maxrss is KiB on Linux and bytes on macOS."""
    system = sys.platform if system is None else system
    if system.startswith("linux"):
        return value * 1024
    if system.startswith("darwin"):
        return value
    raise ValueError(f"unknown ru_maxrss unit for platform {system!r}")


def parse_steal(text):
    for line in text.splitlines():
        fields = line.split()
        if fields and fields[0] == "cpu":
            if len(fields) < 9:
                raise ValueError("cpu line has no steal column")
            return int(fields[8])
    raise ValueError("no aggregate cpu line")


def read_steal(path="/proc/stat"):
    """Steal ticks, or None when /proc/stat is missing (macOS) or unparseable."""
    try:
        return parse_steal(Path(path).read_text())
    except (OSError, ValueError):
        return None


class QuietTimeout(RuntimeError):
    def __init__(self, load, threshold, max_wait):
        self.load = load
        super().__init__(f"host not quiet: 1-min loadavg {load:.2f} >= {threshold} "
                         f"after {max_wait}s")


def wait_quiet(threshold, max_wait):
    """Block until the 1-minute loadavg is below threshold; return the value used."""
    deadline = time.monotonic() + max_wait
    while True:
        load = os.getloadavg()[0]
        if load < threshold:
            return load
        if time.monotonic() >= deadline:
            raise QuietTimeout(load, threshold, max_wait)
        time.sleep(min(5.0, max(0.0, deadline - time.monotonic())))


def run_child(command, cwd, env, stdout_path, stderr_path):
    """Run one child; wall by monotonic, CPU and peak RSS from wait4 rusage of that child only."""
    with open(stdout_path, "wb") as out, open(stderr_path, "wb") as err:
        start = time.monotonic()
        proc = subprocess.Popen(command, cwd=cwd, env=env, stdout=out, stderr=err)
        _, status, usage = os.wait4(proc.pid, 0)
        wall = time.monotonic() - start
        proc.returncode = os.waitstatus_to_exitcode(status)
    if proc.returncode != 0:
        tail = Path(stderr_path).read_bytes()[-2000:].decode(errors="replace")
        raise RuntimeError(f"command failed with exit {proc.returncode}: "
                           f"{shlex.join(map(str, command))}\n{tail}")
    return dict(exit_code=0, wall_s=wall, cpu_user_s=usage.ru_utime,
                cpu_system_s=usage.ru_stime, peak_rss_bytes=rss_bytes(usage.ru_maxrss))


def sha256_file(path):
    return fetch_corpora.sha256_file(path)


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True,
                          check=True).stdout.strip()


def resolve_ref(repo, ref, warnings=None):
    resolved = {}
    explicit = (ref == "HEAD" or ref.startswith(("origin/", "refs/")) or
                re.fullmatch(r"[0-9a-f]{7,40}", ref) is not None)
    candidates = (ref,) if explicit else (ref, f"origin/{ref}")
    for candidate in candidates:
        try:
            resolved[candidate] = git(repo, "rev-parse", "--verify", "--quiet",
                                      candidate + "^{commit}")
        except subprocess.CalledProcessError:
            continue
    remote = f"origin/{ref}"
    if not resolved:
        raise ValueError(f"cannot resolve ref: {ref}")
    if ref in resolved and remote in resolved and resolved[ref] != resolved[remote]:
        identities = (f"{ref} resolves to {resolved[ref]}, but {remote} resolves to "
                      f"{resolved[remote]}")
        try:
            branch = git(repo, "symbolic-ref", "--quiet", "HEAD")
        except subprocess.CalledProcessError as error:
            if error.returncode != 1:
                raise
            branch = None  # Detached HEAD is not a checked-out branch.
        if branch == f"refs/heads/{ref}":
            record_warning(f"{identities}; checked-out branch, using {ref}", warnings)
            return resolved[ref]
        try:
            git(repo, "merge-base", "--is-ancestor", resolved[ref], resolved[remote])
        except subprocess.CalledProcessError as error:
            if error.returncode != 1:
                raise
            raise ValueError(f"{identities}; ambiguous ref: pass {remote} or a SHA explicitly")
        record_warning(f"{identities}; stale local ref, using {remote}", warnings)
    return resolved.get(remote, resolved.get(ref))


def record_warning(message, warnings):
    print(f"warning: {message}", file=sys.stderr, flush=True)
    if warnings is not None:
        warnings.append(message)


def build_binaries(refs, repo, work_dir, allow_dirty=False, warnings=None):
    """Build each ref (a ref equal to HEAD reuses the checkout); return [{ref, sha256, path}]."""
    head = git(repo, "rev-parse", "HEAD")
    # Refuse ambiguous inputs before building even the first requested ref.
    commits = [(ref, resolve_ref(repo, ref, warnings)) for ref in refs]
    result = []
    for ref, commit in commits:
        if commit == head:
            tree = Path(repo)
        else:
            tree = Path(work_dir) / "worktrees" / re.sub(r"[^A-Za-z0-9_.-]", "_", ref)
            if not tree.exists():
                subprocess.run(["git", "-C", str(repo), "worktree", "add", "--detach",
                                str(tree), commit], check=True)
        actual = git(tree, "rev-parse", "HEAD")
        if actual != commit:
            raise ValueError(f"worktree {tree} HEAD {actual} != requested commit {commit}")
        if not allow_dirty and git(tree, "status", "--porcelain", "--untracked-files=no"):
            raise ValueError(f"dirty tracked tree at {tree}; use --allow-dirty to opt in")
        subprocess.run(["cargo", "build", "--release", "--locked", "--bin", "prism"],
                       cwd=tree, check=True)
        binary = tree / "target" / "release" / "prism"
        result.append(dict(ref=ref, commit=commit, path=str(binary), sha256=sha256_file(binary)))
    return result


def parse_args(argv):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--refs", default="main", help="comma-separated git refs")
    p.add_argument("--corpora", default="T,X")
    p.add_argument("--corpus-dir", action="append", default=[], metavar="KEY=PATH",
                   help="use an existing directory for a corpus key (skips the registry root)")
    p.add_argument("--rounds", type=int, default=3)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--query", default=DEFAULT_QUERY)
    p.add_argument("--work-dir", type=Path, default=Path("/work"))
    p.add_argument("--out", type=Path, default=Path("measure-out"))
    p.add_argument("--repo", type=Path, default=REPO_ROOT)
    p.add_argument("--prism-bin", type=Path,
                   help="use this binary for the single ref and skip the build")
    p.add_argument("--allow-dirty", action="store_true",
                   help="allow building a checkout with tracked changes")
    p.add_argument("--quiet-threshold", type=float, default=0.5,
                   help="1-min loadavg below this before each run; raise to relax")
    p.add_argument("--quiet-wait", type=float, default=300.0, help="max seconds to wait")
    p.add_argument("--quiet-on-timeout", choices=("fail", "record"), default="record",
                   help="abort on gate timeout or record gate_failed and continue (default)")
    args = p.parse_args(argv)
    for name in ("out", "work_dir", "repo", "prism_bin"):
        path = getattr(args, name)
        if path is not None:
            setattr(args, name, path.resolve())
    args.refs = [r for r in args.refs.split(",")]
    args.corpora = args.corpora.split(",")
    if args.rounds < 1 or args.threads < 1:
        p.error("--rounds and --threads must be at least 1")
    if not all(args.refs) or len(set(args.refs)) != len(args.refs):
        p.error("--refs must be nonempty and distinct")
    if args.prism_bin is not None and len(args.refs) != 1:
        p.error("--prism-bin supports exactly one ref")
    if not math.isfinite(args.quiet_threshold) or args.quiet_wait < 0:
        p.error("--quiet-threshold must be finite and --quiet-wait nonnegative")
    if not shlex.split(args.query):
        p.error("--query must not be empty")
    if not all(args.corpora) or len(set(args.corpora)) != len(args.corpora):
        p.error("--corpora must be nonempty and distinct")
    overrides = {}
    for item in args.corpus_dir:
        key, sep, path = item.partition("=")
        if not sep or not key or not path:
            p.error(f"--corpus-dir needs KEY=PATH: {item!r}")
        overrides[key] = Path(path).resolve()
    args.overrides = overrides
    return args


def corpus_roots(args):
    registry = fetch_corpora.load_registry()
    roots = {}
    for key in args.corpora:
        if key in args.overrides:
            roots[key] = args.overrides[key]
        elif key in registry:
            roots[key] = args.work_dir / "corpora" / key / registry[key]["root"]
        else:
            raise ValueError(f"unknown corpus {key!r}: not in registry and no --corpus-dir")
        if not roots[key].is_dir():
            raise ValueError(f"corpus directory missing: {roots[key]}")
    return roots


def environment(args):
    def read(path):
        try:
            return Path(path).read_text()
        except OSError:
            return None
    mem = read("/proc/meminfo")
    total = re.search(r"MemTotal:\s+(\d+) kB", mem or "")
    return {"platform": platform.platform(), "machine": platform.machine(),
            "cpu_count": os.cpu_count(), "python": platform.python_version(),
            "mem_total_gib": f"{int(total.group(1)) / 2**20:.1f}" if total else "unavailable",
            "threads": args.threads, "rounds": args.rounds, "query": args.query,
            "quiet_threshold": args.quiet_threshold,
            "quiet_on_timeout": args.quiet_on_timeout, "allow_dirty": args.allow_dirty,
            "steal_sampling": "available" if read_steal() is not None else "unavailable",
            "loadavg_at_start": " ".join(f"{x:.2f}" for x in os.getloadavg())}


def main(argv=None):
    args = parse_args(argv)
    runs_dir = args.out / "runs"
    runs_dir.mkdir(parents=True, exist_ok=True)
    env_info, binaries, runs, warnings = {}, [], [], []
    failure = None

    def write_results(reason):
        # Replace a complete file, so interruption during rendering preserves the last checkpoint.
        temporary = args.out / "RESULTS.md.tmp"
        temporary.write_text(reporting.render_results(env_info, binaries, runs,
                                                      failure=reason, warnings=warnings))
        temporary.replace(args.out / "RESULTS.md")

    try:
        env_info = environment(args)
        roots = corpus_roots(args)
        if args.prism_bin is not None:
            binaries = [dict(ref=args.refs[0], commit="unknown (prism-bin)",
                             path=str(args.prism_bin), sha256=sha256_file(args.prism_bin))]
        else:
            binaries = build_binaries(args.refs, args.repo, args.work_dir,
                                      allow_dirty=args.allow_dirty, warnings=warnings)
        by_ref = {b["ref"]: b for b in binaries}
        ref_ids = {ref: i for i, ref in enumerate(args.refs)}
        query = shlex.split(args.query)
        write_results("measurement in progress")
        quiet_wait = args.quiet_wait
        for round_id, corpus, ref in run_order(args.refs, args.corpora, args.rounds):
            run_id = f"r{round_id}-{corpus}-{re.sub(r'[^A-Za-z0-9_.-]', '_', ref)}-{ref_ids[ref]}"
            profile = runs_dir / f"{run_id}.profile.jsonl"
            command = [by_ref[ref]["path"], *query, "--repo", str(roots[corpus])]
            child_env = {**os.environ, "PRISM_PROFILE": str(profile),
                         "RAYON_NUM_THREADS": str(args.threads)}
            gate_reason = None
            gate_wait = quiet_wait
            try:
                load = wait_quiet(args.quiet_threshold, gate_wait)
            except QuietTimeout as error:
                if args.quiet_on_timeout == "fail":
                    raise
                load, gate_reason = error.load, str(error)
                print(f"warning: {run_id}: {gate_reason}; recording gate_failed", file=sys.stderr)
                if quiet_wait > 60:
                    quiet_wait = 60.0
                    record_warning(f"{run_id}: timeout; later quiet gates wait at most "
                                   f"{quiet_wait:g}s (reduced from {gate_wait:g}s)", warnings)
            # A previous invocation's profile cannot satisfy this run's profile requirement.
            profile.unlink(missing_ok=True)
            steal_before = read_steal()
            usage = run_child(command, runs_dir, child_env, runs_dir / f"{run_id}.stdout",
                              runs_dir / f"{run_id}.stderr")
            steal_after = read_steal()
            if not profile.is_file():
                raise RuntimeError(f"{run_id}: missing profile file after successful child: {profile}")
            records = load_records(profile)
            phases = [dict(name=n, time_kind=k, ns=v)
                      for (n, k), v in sorted(reporting.phase_totals(records).items())]
            with gzip.open(runs_dir / f"{run_id}.jsonl.gz", "wt") as stream:
                stream.write("".join(json.dumps(r) + "\n" for r in records))
            profile.unlink()
            run = dict(round=round_id, corpus=corpus, ref=ref, quiet_load=load,
                       quiet_wait_s=gate_wait, quiet_wait_reduced=gate_wait < args.quiet_wait,
                       gate_failed=gate_reason is not None, gate_failure_reason=gate_reason,
                       steal_before=steal_before, steal_after=steal_after, phases=phases,
                       threads=args.threads, command=command,
                       commit=by_ref[ref]["commit"], sha256=by_ref[ref]["sha256"], **usage)
            (runs_dir / f"{run_id}.meta.json").write_text(json.dumps(run, indent=2) + "\n")
            runs.append(run)
            write_results("measurement in progress")
            print(f"{run_id}: wall {usage['wall_s']:.2f}s", flush=True)
    except BaseException as error:
        failure = f"{type(error).__name__}: {error}"
        if isinstance(error, (KeyboardInterrupt, SystemExit)):
            raise
        sys.exit(f"measure failed: {failure}")
    finally:
        write_results(failure)
    print(f"wrote {args.out / 'RESULTS.md'}")


if __name__ == "__main__":
    main()
