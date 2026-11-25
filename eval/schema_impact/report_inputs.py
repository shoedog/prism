"""Freeze a first-parent comparison before any replay evidence is opened."""

from contextlib import contextmanager
from dataclasses import asdict, dataclass
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
from .replay import DEFAULT_SCHEMA, select_config
from .report_schema import schema_inventory
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


@dataclass(frozen=True)
class FrozenReport:
    request: ReportRequest
    before: dict
    after: dict
    changed_paths: tuple[str, ...]
    before_sources: dict
    after_sources: dict


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


def _source_receipt(path, data):
    return {"file": path.name, "origin": "study-supplied selection receipt",
            "revision": f"sha256:{sha256(data).hexdigest()}", "tree": None, "blob": None}


def _driver():
    directory = Path(__file__).resolve().parent
    hashes = {path.name: sha256(path.read_bytes()).hexdigest() for path in sorted(directory.glob("*.py"))
              if path.name not in {"render.py", "report_evaluation.py", "report_batch.py", "_replay_measure.py"}}
    version = sha256(json.dumps(hashes, sort_keys=True).encode()).hexdigest()
    try:
        base = revision(directory, "HEAD")
    except subprocess.CalledProcessError:
        base = None
    return {"name": "schema-impact", "version": "2", "rev": base, "build_identity": f"sha256:{version}",
            "source_hashes": hashes}


def _config(repo, rev, directory):
    path = select_config(repo, rev, directory)
    try:
        data = path.read_bytes()
    except OSError:
        return None, None
    try:
        document = tomllib.loads(data.decode("utf-8"))
        parsed = config.Config(document.get("aliases", {}), [config.ClientSpec(**entry) for entry in document.get("client", [])])
    except (UnicodeError, ValueError, TypeError):
        return None, _source_receipt(path, data) | {"error": "could not be read as client configuration"}
    return parsed, {**_source_receipt(path, data), "selected_by": "first-parent ancestry",
                    "selected_for": rev, "aliases": parsed.aliases,
                    "entries": [{"entry": i, **asdict(entry)} for i, entry in enumerate(parsed.clients, start=1)]}


def _snapshot(request, rev, driver, spec, spec_source, configuration, *, reused=None):
    common = _metadata(request.repo, rev)
    common.update(api=request.repo.name, repo=request.repo.name, tool=driver, origin="selected commit tree",
                  sha256=None, content_hash=None, config_sources=[], dependencies=[], source_url=request.source_url)
    common["tree_files"] = _tree(request.repo, common["tree"])
    source_type = routes._EXTENSIONS if request.route_spec else (".go",)
    reused = reused or {}
    def service_wanted(file):
        return in_root(file, request.service_root) and (
            Path(file).suffix in source_type and (request.route_spec is not None or go._is_go_file(file))
        )
    wanted = {file: info for file, info in common["tree_files"].items()
              if ("client" not in reused and Path(file).suffix in ts.EXTENSIONS)
              or ("service" not in reused and (service_wanted(file) or file == request.schema))
              or info["mode"] in {"120000", "160000"}}
    service = dict(common, role="producer", input_id=f"service:{rev}", scope=request.service_root,
                   operation_authority="route table" if request.route_spec else "Go annotations")
    client = dict(common, role="consumer", input_id=f"client:{rev}", scope=request.root, schema=None,
                  operation_authority=None)
    with _source_tree(request.repo, wanted) as (directory, selected):
        unresolved = {file for file, contents in selected.items() if contents is None
                      and common["tree_files"][file]["mode"] in {"120000", "160000"}}
        if "service" not in reused:
            relevant = {name: contents for name, contents in selected.items()
                        if service_wanted(name) or name in unresolved}
            if spec is not None:
                service["config_sources"] = [spec_source]
            if request.route_spec and spec is None:
                reason = f"route specification for {rev[:8]} is missing"
                gap = diagnostic(request.service_root, reason)
                paths = sorted(file for file in relevant if in_root(file, request.service_root))
                service.update(inventory(service, paths, [], [gap, *(
                    diagnostic(file, reason, status="unknown") for file in paths
                )]), operations={}, registration_inventory=None, outside_registration=None)
            else:
                service.update(operations(service, directory, request.service_root, relevant, spec))
            service.update(schema_inventory(service, request.schema, selected.get(request.schema)))
            if request.schema is None and spec is not None:
                service["schema"]["source"]["derived"] = "route_table"
                if not service["unaccounted_scopes"]:
                    digest = _hash_shapes(dict.fromkeys(service["operations"], ""))
                    service["schema"]["content_hash"] = service["content_hash"] = digest
        if "client" not in reused:
            cfg, cfg_source = configuration
            if cfg_source:
                client["config_sources"] = [cfg_source]
            relevant_client = {name: contents for name, contents in selected.items()
                               if Path(name).suffix in ts.EXTENSIONS or name in unresolved}
            client.update(client_calls(client, directory, request.root, relevant_client, cfg, cfg_source))
        del service["tree_files"], client["tree_files"]
        commit = {key: common[key] for key in ("rev", "tree", "committed_at", "subject", "dirty", "snapshot_kind")}
        return {"service": service, "client": client, **reused, "commit": commit}, selected


def build_inputs(request):
    if request.role not in {"producer", "consumer"}:
        raise ValueError(f"Unknown subject role: {request.role}")
    for role_path in (request.root, request.service_root, request.schema):
        if role_path is not None:
            _repo_path(role_path)
    if request.source_url:
        url = urlsplit(request.source_url)
        if url.scheme != "https" or not url.hostname or url.username or url.query or url.fragment:
            raise ValueError("Source URL must be an HTTPS repository URL")
    head = revision(request.repo, request.subject)
    base = parent(request.repo, head)
    if base is None:
        raise ValueError("The selected subject has no first parent")
    spec, spec_source = None, None
    if request.route_spec is not None and request.route_spec.is_file():
        raw = request.route_spec.read_bytes()
        spec = routes.RouteSpec(**json.loads(raw))
        spec_source = {**_source_receipt(request.route_spec, raw), "mapping": asdict(spec)}
    driver = _driver()
    changes = tuple(changed_files(request.repo, base, head))
    base_cfg, head_cfg = (_config(request.repo, rev, request.configs) for rev in (base, head))
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
        account_changed_route_regions(before["service"], after["service"], changes)
        account_changed_route_regions(after["service"], before["service"], changes)
    return FrozenReport(request, before, after, changes, before_sources, after_sources)
