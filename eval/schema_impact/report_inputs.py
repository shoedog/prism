"""Freeze a first-parent comparison before any replay evidence is opened."""

from contextlib import contextmanager
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timezone
from hashlib import sha256
import json
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory
import tomllib
from urllib.parse import urlsplit

from . import _ts_syntax as ts, config, producer_go as go, producer_route_table as routes
from ._replay_git import changed_files, git, parent, revision
from .config import DEFAULT_SCHEMA, select_config
from .report_schema import _empty, schema_inventory
from .schema import _hash_shapes
from .report_sources import account_changed_route_regions, client_calls, diagnostic, in_root, inventory, operations


@dataclass(frozen=True)
class ReportRequest:
    repo: Path
    subject: str
    role: str
    configs: Path
    root: str = "ui/src"
    schema: str | None = DEFAULT_SCHEMA
    service_root: str = "."
    route_spec: Path | None = None
    selection_time: str | None = None
    source_url: str | None = None
    counterpart_repo: Path | None = None
    counterpart_rev: str | None = None
    counterpart_source_url: str | None = None
    schema_omitted: bool = False
    mode: str = "version_comparison"
    cutoff_receipt: dict | None = None
    study_repo: Path | None = None
    study_rev: str | None = None


@dataclass(frozen=True)
class FrozenReport:
    request: ReportRequest
    before: dict
    after: dict
    changed_paths: tuple[str, ...]
    before_sources: dict
    after_sources: dict
    reference: dict | None = None


def _repo_path(path):
    if Path(path).is_absolute() or ".." in Path(path).parts:
        raise ValueError(f"Expected a relative tree path: {path}")
    return Path(path).as_posix()


def _metadata(repo, commit):
    tree, date, subject = git(repo, "show", "-s", "--format=%T%x00%cI%x00%s", commit).decode().rstrip("\n").split("\0")
    return {"rev": commit, "tree": tree, "committed_at": datetime.fromisoformat(date).astimezone(timezone.utc).isoformat(),
            "subject": subject, "dirty": False, "snapshot_kind": "commit"}


def _tree(repo, tree):
    result = {}
    for row in git(repo, "ls-tree", "-r", "-z", tree).split(b"\0"):
        if row:
            metadata, path = row.split(b"\t", 1)
            mode, _, blob = metadata.decode().split()
            result[path.decode()] = {"mode": mode, "blob": blob}
    return result


@contextmanager
def _source_tree(repo, files):
    """Materialize only selected regular blobs, including sources needed by import resolution."""
    with TemporaryDirectory(prefix="schema-impact-report-") as directory:
        root = Path(directory).resolve()
        selected = {}
        with subprocess.Popen(["git", "-C", str(repo), "cat-file", "--batch"],
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE) as process:
            for file, record in files.items():
                selected[file] = None
                if not record["mode"].startswith("100"):
                    continue
                oid = record["blob"]
                process.stdin.write(f"{oid}\n".encode())
                process.stdin.flush()
                header = process.stdout.readline().decode().split()
                if len(header) != 3 or header[:2] != [oid, "blob"]:
                    continue
                size = int(header[2])
                data = process.stdout.read(size)
                if len(data) != size or process.stdout.read(1) != b"\n":
                    raise ValueError(f"Incomplete blob for {file}")
                try:
                    selected[file] = data.decode("utf-8")
                except UnicodeDecodeError:
                    continue
                destination = root / _repo_path(file)
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            process.stdin.close()
            if process.wait():
                raise subprocess.CalledProcessError(process.returncode, process.args)
        yield root, selected


CAPTURE_WORDING = "read at capture time, not shown to have existed at the change time"


def selected_schema(schema, route_spec, *, omitted=False):
    return None if omitted else schema or (None if route_spec else DEFAULT_SCHEMA)


def _source_receipt(path, data, captured_at, error=None):
    return {"file": path.name, "origin": CAPTURE_WORDING, "captured_at": captured_at,
            "revision": f"sha256:{sha256(data).hexdigest()}" if data is not None else None,
            "tree": None, "blob": None, "error": error}


def _driver(captured_at, *, include_checkout=True):
    directory = Path(__file__).resolve().parent
    hashes = {path.name: sha256(path.read_bytes()).hexdigest() for path in sorted(directory.glob("*.py"))
              if path.name not in {"render.py", "report_evaluation.py", "report_batch.py",
                                   "report_batch_evaluation.py", "_replay_measure.py"}}
    version = sha256(json.dumps(hashes, sort_keys=True).encode()).hexdigest()
    commit = dict.fromkeys(("rev", "committed_at", "subject"))
    if include_checkout:
        try:
            commit = _metadata(directory, revision(directory, "HEAD"))
        except subprocess.CalledProcessError:
            pass
    return {"name": "schema-impact", "version": "2", "rev": commit["rev"], "committed_at": commit["committed_at"],
            "subject": commit["subject"], "build_identity": f"sha256:{version}",
            "source_hashes": hashes, "origin": CAPTURE_WORDING, "captured_at": captured_at}


def _config(repo, rev, source, captured_at, tree_reader=None):
    selected_by = "first-parent ancestry"
    is_file = (lambda path: tree_reader.contains(path)) if tree_reader else Path.is_file
    if source.suffix.lower() == ".toml" or is_file(source):
        path, selected_by = source, "single supplied file"
    else:
        path = select_config(repo, rev, source)
        if not is_file(path):
            files = tree_reader.configs(source) if tree_reader else sorted(source.glob("*.toml"))
            if len(files) == 1:
                path, selected_by = files[0], "single supplied file"
    receipt = tree_reader.receipt if tree_reader else _source_receipt
    try:
        data = tree_reader.read(path) if tree_reader else path.read_bytes()
    except FileNotFoundError:
        return None, receipt(path, None, captured_at, "client configuration is missing")
    except OSError:
        return None, receipt(path, None, captured_at, "client configuration could not be read")
    try:
        document = tomllib.loads(data.decode("utf-8"))
        aliases = document.get("aliases", {})
        if not isinstance(aliases, dict) or not all(isinstance(value, str) for value in aliases.values()):
            raise ValueError("invalid client aliases")
        parsed = config.Config(aliases, [config.ClientSpec(**entry) for entry in document.get("client", [])])
    except (UnicodeError, ValueError, TypeError):
        return None, receipt(path, data, captured_at, "client configuration could not be parsed")
    return parsed, {**receipt(path, data, captured_at), "selected_by": selected_by,
                    "selected_for": rev, "aliases": parsed.aliases,
                    "entries": [{"entry": i, **asdict(entry)} for i, entry in enumerate(parsed.clients, start=1)]}


def _route_spec(path, captured_at, tree_reader=None):
    if path is None:
        return None, None
    receipt = tree_reader.receipt if tree_reader else _source_receipt
    try:
        raw = tree_reader.read(path) if tree_reader else path.read_bytes()
    except FileNotFoundError:
        return None, receipt(path, None, captured_at, "route specification is missing")
    except OSError:
        return None, receipt(path, None, captured_at, "route specification could not be read")
    try:
        spec = routes.RouteSpec(**json.loads(raw))
        mapping = asdict(spec)
        if not all(isinstance(mapping[key], str) and (mapping[key] or key == "mount") for key in (
            "callee", "method_key", "path_key", "handler_key", "validation_key", "mount"
        )) or not all(isinstance(mapping[key], (list, tuple))
                      and all(isinstance(entry, str) for entry in mapping[key])
                      for key in ("handler_under", "validation_under")):
            raise ValueError("invalid route specification")
    except (ValueError, TypeError):
        return None, receipt(path, raw, captured_at, "route specification could not be parsed or validated")
    return spec, {**receipt(path, raw, captured_at), "mapping": mapping}


class StudyTree:
    def __init__(self, repo, rev):
        self.repo, self.rev = repo, rev
        self.tree_id = _metadata(repo, rev)["tree"]
        self.files = _tree(repo, self.tree_id)

    def contains(self, path):
        return _repo_path(path) in self.files

    def configs(self, directory):
        return sorted(Path(file) for file in self.files if Path(file).parent == directory and file.endswith(".toml"))

    def read(self, path):
        record = self.files.get(_repo_path(path))
        if record is None:
            raise FileNotFoundError(path)
        if not record["mode"].startswith("100"):
            raise OSError("study input is not a regular file")
        return git(self.repo, "cat-file", "blob", record["blob"])

    def receipt(self, path, data, captured_at, error=None):
        return _source_receipt(path, data, captured_at, error) | {
            "file": _repo_path(path), "origin": "named study tree", "tree": self.tree_id,
            "study_rev": self.rev, "blob": self.files.get(_repo_path(path), {}).get("blob"),
        }


def _location(request, counterpart):
    if counterpart:
        return request.counterpart_repo, request.counterpart_source_url
    return request.repo, request.source_url


def _snapshot_record(request, commit, driver, counterpart):
    repo, url = _location(request, counterpart)
    service_repo = request.counterpart_repo if request.role == "consumer" and request.counterpart_repo else request.repo
    return dict(commit, api=service_repo.name, repo=repo.name,
                repository_id=url or f"sha256:{sha256(str(repo.resolve()).encode()).hexdigest()}",
                tool=driver, origin="selected commit tree",
                sha256=None, content_hash=None, config_sources=[], dependencies=[], source_url=url)


def _role_record(request, common, role):
    rev = common["rev"] or "unavailable"
    if role == "service":
        return dict(common, role="producer", input_id=f"service:{rev}", scope=request.service_root,
                    operation_authority="route table" if request.route_spec else "Go annotations")
    return dict(common, role="consumer", input_id=f"client:{rev}", scope=request.root, schema=None,
                operation_authority=None)


def _snapshot(request, rev, driver, spec, spec_source, configuration, *, reused=None,
              roles=("service", "client"), counterpart=False):
    repo, _ = _location(request, counterpart)
    commit = _metadata(repo, rev)
    common = _snapshot_record(request, commit, driver, counterpart)
    common["tree_files"] = _tree(repo, common["tree"])
    source_type = routes._EXTENSIONS if request.route_spec else (".go",)
    reused = reused or {}
    scan_service = "service" in roles and "service" not in reused
    scan_client = "client" in roles and "client" not in reused

    def service_wanted(file):
        return in_root(file, request.service_root) and (
            Path(file).suffix in source_type and (request.route_spec is not None or go._is_go_file(file))
        )
    wanted = {file: info for file, info in common["tree_files"].items()
              if (scan_client and Path(file).suffix in ts.EXTENSIONS)
              or (scan_service and (service_wanted(file) or file == request.schema))
              or info["mode"] in {"120000", "160000"}}
    snapshots = {role: _role_record(request, common, role) for role in roles if role not in reused}
    with _source_tree(repo, wanted) as (directory, selected):
        unresolved = {file for file, contents in selected.items() if contents is None
                      and common["tree_files"][file]["mode"] in {"120000", "160000"}}
        if scan_service:
            service = snapshots["service"]
            relevant = {name: contents for name, contents in selected.items()
                        if service_wanted(name) or name in unresolved}
            if spec_source is not None:
                service["config_sources"] = [spec_source]
            if request.route_spec and spec is None:
                reason = spec_source["error"]
                gap = diagnostic(spec_source["file"] or request.service_root, reason, check="route specification")
                paths = sorted(file for file in relevant if in_root(file, request.service_root))
                service.update(inventory(service, paths, [], [gap, *(
                    diagnostic(file, reason, status="unknown") for file in paths
                )]), operations={}, registration_inventory=None, outside_registration=None)
            else:
                service.update(operations(service, directory, request.service_root, relevant, spec))
            service.update(schema_inventory(service, request.schema, selected.get(request.schema),
                                            omitted=request.schema_omitted))
            if request.schema is None and spec is not None:
                service["schema"]["source"]["derived"] = "route_table"
                if not service["unaccounted_scopes"]:
                    digest = _hash_shapes(dict.fromkeys(service["operations"], ""))
                    service["schema"]["content_hash"] = service["content_hash"] = digest
        if scan_client:
            client = snapshots["client"]
            cfg, cfg_source = configuration
            if cfg_source:
                client["config_sources"] = [cfg_source]
            relevant_client = {name: contents for name, contents in selected.items()
                               if Path(name).suffix in ts.EXTENSIONS or name in unresolved}
            client.update(client_calls(client, directory, request.root, relevant_client, cfg, cfg_source))
    for snapshot in snapshots.values():
        del snapshot["tree_files"]
    return {**snapshots, **reused, "commit": commit}, selected


def _select_counterpart(request, subject):
    repo, supplied = request.counterpart_repo, request.counterpart_rev
    if request.mode != "version_comparison":
        receipt = request.cutoff_receipt or {}
        tip = receipt.get("counterpart_tip")
        if not tip:
            return None, None, f"no record of the tip of the counterpart branch at {request.selection_time}"
        try:
            commit = revision(repo, tip)
            if commit != tip:
                raise ValueError("cutoff receipt must identify an exact counterpart commit")
            return commit, None, None
        except subprocess.CalledProcessError:
            return None, None, f"the counterpart revision from the cutoff receipt {tip} could not be read"
    if supplied is not None:
        try:
            return revision(repo, supplied), None, None
        except subprocess.CalledProcessError:
            return None, None, f"the supplied counterpart revision {supplied} could not be read"
    tip_record = None
    try:
        tip = revision(repo, "HEAD")
        tip_record = _metadata(repo, tip)
        branch = git(repo, "rev-parse", "--abbrev-ref", "HEAD").decode().strip()
        tip_record["branch"] = branch if branch != "HEAD" else "detached HEAD"
        history = git(repo, "rev-list", "--first-parent", "--timestamp", tip).decode().splitlines()
    except subprocess.CalledProcessError:
        return None, tip_record, "the counterpart repository's first-parent history could not be read"
    cutoff = datetime.fromisoformat(subject["committed_at"]).timestamp()
    for row in history:
        seconds, rev = row.split()
        if int(seconds) <= cutoff:
            return rev, tip_record, None
    return None, tip_record, f"no first-parent commit in the counterpart repository at or before {subject['committed_at']}"


def _unavailable_counterpart(request, role, driver, reason):
    commit = dict.fromkeys(("rev", "tree", "committed_at", "subject", "dirty"))
    common = _snapshot_record(request, dict(commit, snapshot_kind="unavailable"), driver, True)
    snapshot = _role_record(request, dict(common, origin=None, tree_files={}), role)
    snapshot.update(inventory(snapshot, [], [], [diagnostic(snapshot["scope"], reason, status="unknown")]))
    if role == "service":
        snapshot.update(_empty(snapshot, request.schema, reason, "unreadable"), operations={},
                        registration_inventory=None, outside_registration=None)
    else:
        snapshot.update(calls=[], functions={})
    del snapshot["tree_files"]
    return snapshot


def _cross_inputs(request, base, head, driver, changes, study):
    changing, fixed = ("service", "client") if request.role == "producer" else ("client", "service")
    spec, spec_source = study("route specification") if changing == "service" else (None, None)

    def scan(rev, role, counterpart=False):
        repo, _ = _location(request, counterpart)
        cfg = study("client configuration", repo, rev) if role == "client" else None
        return _snapshot(request, rev, driver, spec, spec_source, cfg, roles=(role,), counterpart=counterpart)

    before, before_sources = scan(base, changing)
    after, after_sources = scan(head, changing)
    if changing == "service" and spec is not None:
        account_changed_route_regions(before["service"], after["service"], changes, before_sources, after_sources)
        account_changed_route_regions(after["service"], before["service"], changes, after_sources, before_sources)
    selected, tip, reason = _select_counterpart(request, after["commit"])
    if selected is not None:
        if fixed == "service":
            spec, spec_source = study("route specification")
        try:
            scanned, _ = scan(selected, fixed, counterpart=True)
            reference = scanned[fixed]
        except subprocess.CalledProcessError:
            reason = f"the selected counterpart revision {selected} could not be read"
    if reason is not None:
        reference = _unavailable_counterpart(request, fixed, driver, reason)
    before[fixed] = after[fixed] = reference

    admitted_cutoff = request.mode != "version_comparison"
    supplied = request.counterpart_rev is not None and not admitted_cutoff
    branch = (tip or {}).get("branch") or (request.cutoff_receipt or {}).get("counterpart_branch")
    date = reference["committed_at"]
    offset = (datetime.fromisoformat(date) - datetime.fromisoformat(after["commit"]["committed_at"])) if date else None
    selection = {
        "reference_input_id": reference["input_id"], "repo": reference["repo"],
        "rev": reference["rev"], "tree": reference["tree"], "committed_at": date,
        "requested_rev": request.counterpart_rev if supplied else None,
        "selected_by": "cutoff_receipt" if admitted_cutoff else "supplied_revision" if supplied else "commit_date",
        "selection_reason": ("Selected from the supplied cutoff receipt" if admitted_cutoff else
                             "Revision supplied for this review" if supplied else
                             f"Selected by commit date. When this revision reached {branch or 'HEAD'} is unknown."),
        "selected_rev": selected,
        "availability_status": "unavailable" if reason else "selected", "unavailable_reason": reason,
        "availability_evidence": ({"tree": reference["tree"], "observed_at": request.selection_time,
                                   "branch": branch} if admitted_cutoff and date else
                                  {"tree": reference["tree"]} if date else None),
        "duration_seconds": int(offset.total_seconds()) if offset is not None else None,
        "branch": branch, "observed_tip": selected if admitted_cutoff else None,
        "observed_at": request.selection_time if admitted_cutoff and selected else None,
        "selection_history_tip": tip["rev"] if tip else None,
        "selection_history_tip_committed_at": tip["committed_at"] if tip else None,
    }
    return FrozenReport(request, before, after, changes, before_sources, after_sources, selection)


def build_inputs(request):
    if request.role not in {"producer", "consumer"}:
        raise ValueError(f"Unknown subject role: {request.role}")
    for role_path in (request.root, request.service_root, request.schema):
        if role_path is not None:
            _repo_path(role_path)
    if request.counterpart_repo is None and (request.counterpart_rev is not None or request.counterpart_source_url is not None):
        raise ValueError("Counterpart revision and source URL require a counterpart repository")
    for source_url in filter(None, (request.source_url, request.counterpart_source_url)):
        url = urlsplit(source_url)
        if url.scheme != "https" or not url.hostname or url.username or url.query or url.fragment:
            raise ValueError("Source URL must be an HTTPS repository URL")
    receipt = request.cutoff_receipt or {}
    if request.mode != "version_comparison" and receipt.get("subject_query") == request.subject:
        head = revision(request.repo, receipt["subject"])
        if head != receipt["subject"]:
            raise ValueError("cutoff receipt must identify an exact subject commit")
    else:
        head = revision(request.repo, request.subject)
    base = parent(request.repo, head)
    if base is None:
        raise ValueError("The selected subject has no first parent")
    # An explicitly fixed selection time keeps repeated selected-version comparisons reproducible.
    if request.mode != "version_comparison":
        if request.mode not in {"live", "historical_as_of"}:
            raise ValueError(f"Unknown report mode: {request.mode}")
        receipt = request.cutoff_receipt or {}
        change_time = _metadata(request.repo, head)["committed_at"]
        cutoff = change_time if request.mode == "historical_as_of" else request.selection_time
        observed_at = receipt.get("observed_at")
        if (not cutoff or not observed_at or datetime.fromisoformat(observed_at) != datetime.fromisoformat(cutoff)
                or receipt.get("subject") != head):
            raise ValueError("the requested mode requires an independent receipt for the exact subject and cutoff")
        request = replace(request, selection_time=datetime.fromisoformat(cutoff).astimezone(timezone.utc).isoformat())
    invocation = effective_invocation(request)
    if request.mode != "version_comparison":
        recorded = receipt.get("invocation")
        defaults = effective_invocation(ReportRequest(request.repo, request.subject, request.role,
                                                      request.configs, route_spec=request.route_spec))
        defaults["schema"] = selected_schema(None, request.route_spec)
        if (recorded is not None and recorded != invocation) or (recorded is None and invocation != defaults):
            raise ValueError("the requested study invocation has no matching cutoff record")
    captured_at = request.selection_time if request.mode == "version_comparison" else None
    driver = _driver(captured_at, include_checkout=request.mode == "version_comparison")
    if (request.study_repo is None) != (request.study_rev is None):
        raise ValueError("a named study tree requires both --study-repo and --study-rev")
    study_tree = None
    if request.study_repo is not None:
        if request.mode == "version_comparison":
            study_tree = StudyTree(request.study_repo, revision(request.study_repo, request.study_rev))
        elif receipt.get("study_rev") == request.study_rev:
            if revision(request.study_repo, request.study_rev) != request.study_rev:
                raise ValueError("cutoff receipt must identify an exact study commit")
            study_tree = StudyTree(request.study_repo, request.study_rev)

    def study(kind, repo=None, rev=None):
        source = request.route_spec if repo is None else request.configs
        if source is None:
            return None, None
        if request.mode != "version_comparison" and study_tree is None:
            return None, {"file": None, "revision": None,
                          "kind": kind,
                          "error": f"{kind} has no evidence of availability at the cutoff"}
        result, evidence = (_route_spec(source, captured_at, study_tree) if repo is None else
                            _config(repo, rev, source, captured_at, study_tree))
        return result, {**evidence, "kind": kind}

    changes = tuple(changed_files(request.repo, base, head))
    if request.counterpart_repo is not None:
        return _cross_inputs(request, base, head, driver, changes, study)
    spec, spec_source = study("route specification")
    base_cfg, head_cfg = (study("client configuration", request.repo, rev) for rev in (base, head))
    before, before_sources = _snapshot(request, base, driver, spec, spec_source, base_cfg)
    reused = {}
    client_paths = set(before["client"]["expected_files"]) | {
        dep["path"] for dep in before["client"]["dependencies"] if dep["path"]
    }
    if (request.role == "producer" and not any(
            file in client_paths or Path(file).suffix in ts.EXTENSIONS or
            in_root(file, request.root) for file in changes
    ) and base_cfg[0] == head_cfg[0]):
        reused["client"] = before["client"]
    service_paths = routes._EXTENSIONS if request.route_spec else (".go",)
    if (request.role == "consumer" and request.schema not in changes and not any(
            Path(file).suffix in service_paths and in_root(file, request.service_root)
            or file in before["service"]["expected_files"] for file in changes
    )):
        reused["service"] = before["service"]
    after, after_sources = _snapshot(request, head, driver, spec, spec_source, head_cfg, reused=reused)
    if spec is not None and "service" not in reused:
        account_changed_route_regions(before["service"], after["service"], changes, before_sources, after_sources)
        account_changed_route_regions(after["service"], before["service"], changes, after_sources, before_sources)
    return FrozenReport(request, before, after, changes, before_sources, after_sources)


def effective_invocation(request):
    return {key: getattr(request, key) for key in ("root", "service_root", "schema", "schema_omitted",
                                                 "source_url", "counterpart_source_url")} | {
        "configs": request.configs.as_posix() if request.study_repo else request.configs.name,
        "route_spec": ((request.route_spec.as_posix() if request.study_repo else request.route_spec.name)
                       if request.route_spec else None),
    }
