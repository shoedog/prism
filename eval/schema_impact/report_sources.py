"""Source inventories collected from the snapshots selected for a report."""

from collections import Counter, defaultdict
from dataclasses import asdict
from functools import cache
from hashlib import sha256
from pathlib import Path
from urllib.parse import quote

from . import _ts_syntax as ts, consumer_ts, producer_go as go, producer_route_table as routes
from ._replay_client import ClientSources
from ._ts_imports import Resolver


def in_root(path, root):
    return Path(path).is_relative_to(root)


def site(snapshot, file, line, **extra):
    url = snapshot["source_url"]
    anchor = f"L{line}"
    if extra.get("end_line", line) > line:
        anchor += f"-L{extra['end_line']}"
    return {
        "rev": snapshot["rev"], "tree": snapshot["tree"], "file": file,
        "blob": snapshot["tree_files"][file]["blob"], "line": line, **extra,
        "url": f"{url.rstrip('/')}/blob/{snapshot['rev']}/{quote(file)}#{anchor}" if url else None,
    }


def diagnostic(file, reason, line=None, *, status="partial"):
    return {"file": file, "line": line, "status": status, "reason": reason}


def inventory(snapshot, paths, entries, gaps, excluded=()):
    by_file = {}
    for entry in entries:
        by_file.setdefault(entry["file"], []).append(entry)
    affected = {gap["file"]: gap for gap in gaps}
    return {
        "expected_files": paths,
        "file_status": [
            {"path": file, **snapshot["tree_files"].get(file, {"blob": None, "mode": None}),
             "status": ("skipped" if file in excluded else affected.get(file, {}).get("status", "complete")),
             "reason": ("test file excluded" if file in excluded else affected.get(file, {}).get("reason")),
             "entries": by_file.get(file, [])}
            for file in sorted(set(paths) | set(excluded))
        ],
        "unaccounted_scopes": gaps,
    }


def operations(snapshot, root, service_root, selected, route_spec=None):
    paths = [file for file in selected if in_root(file, service_root)]
    entries, gaps, operations = [], [], {}
    registrations, outside = None, None
    if not any(in_root(file, service_root) for file in snapshot["tree_files"]):
        gaps.append(diagnostic(service_root, f"source root for {snapshot['rev'][:8]} is missing"))
    readable = {}
    for file in paths:
        source = selected[file]
        if source is None:
            gaps.append(diagnostic(file, "file could not be read", status="unreadable"))
        else:
            readable[Path(file).relative_to(service_root).as_posix()] = source

    def add(operation_id, record, original_path, method, **evidence):
        op = operations.setdefault(operation_id, {
            "operation_id": operation_id, "authority": snapshot["operation_authority"], "sites": [],
        })
        op["sites"].append(site(snapshot, record["file"], record["line"],
                                **{k: record[k] for k in ("end_line", "byte_span", "text")},
                                original_path=original_path, original_method=method,
                                entry_digest=sha256(record["text"].encode()).hexdigest(), **evidence))

    if route_spec is None:
        registrations = {"sites": [], "unknown_paths": [], "other_methods": {},
                         "coverage": "not_checked", "reason": "mounted paths and dispatch were not verified"}
        unextracted = Counter()
        nonliteral = defaultdict(list)
        for relative, source, tree in go._parsed(readable):
            file = (Path(service_root) / relative).as_posix()
            if tree.has_error:
                gaps.append(diagnostic(file, "Go declarations could not be parsed", status="unreadable"))
                continue
            lines = go._source_lines(source)
            for found in go._registration_sites(file, lines, tree, unextracted, nonliteral):
                row = found.registration
                registrations["sites"].append({
                    **asdict(row), "site": site(snapshot, file, row.line, text="\n".join(found.lines)),
                    "full_path": None, "status": "observed_suffix",
                })
            bindings = {binding.line: binding for group in go._annotation_groups(file, lines, tree)
                        for binding in group}
            for node in go._nodes(tree):
                if node.type != "comment" or b"@Router" not in node.text:
                    continue
                line = node.start_point.row + 1
                record = {"file": file, "line": line, "end_line": node.end_point.row + 1,
                          "byte_span": [node.start_byte, node.end_byte], "text": go._text(node),
                          "kind": "annotation", "status": "complete", "reason": None}
                entries.append(record)
                match = go._ROUTER.fullmatch(record["text"])
                if line not in bindings or not match or match[2].upper() not in go._METHODS | {"TRACE"}:
                    reason = "route annotation is unattached, malformed, or names an unsupported method"
                    record.update(status="partial", reason=reason)
                    gaps.append(diagnostic(file, reason, line))
                    continue
                binding = bindings[line]
                record["operation_ids"] = [binding.element]
                add(binding.element, record, match[1], match[2], kind="annotation",
                    function=binding.function, handler_ref=binding.function, handler_site=None,
                    optional_param=False, validation_digest=None, handler_digest=None)
        registrations["other_methods"] = dict(unextracted)
        for found in (row for rows in nonliteral.values() for row in rows):
            registrations["unknown_paths"].append({
                "method": found.method, "site": site(snapshot, found.file, found.line, text="\n".join(found.lines)),
                "reason": "the registration address is built while the program runs",
            })
        dependencies = []
    else:
        directory = root / service_root
        extraction = routes.extract(directory, route_spec) if directory.is_dir() else routes.Extraction([], [], [])
        outside = [dict(row, file=(Path(service_root) / row["file"]).as_posix())
                   for row in extraction.outside_registration]
        index = {}
        for record in extraction.entries:
            file = (Path(service_root) / record["file"]).as_posix()
            record = dict(record, file=file, kind="route declaration",
                          status="partial" if record["reason"] else "complete")
            entries.append(record)
            if record["reason"]:
                gaps.append(diagnostic(file, record["reason"], record["line"]))
            for operation_id in record["operation_ids"]:
                index.setdefault((file, record["line"], operation_id), []).append(record)
        for parsed in extraction.unparsed:
            gaps.append(diagnostic((Path(service_root) / parsed).as_posix(),
                                   "route declaration file could not be parsed", status="unreadable"))
        dependencies = extraction.unextracted
        for route in extraction.routes:
            file = (Path(service_root) / route.file).as_posix()
            identifier = f"op:{route.method} {route.path}"
            record = index[file, route.line, identifier].pop(0)
            handler = route.handler_site
            handler_site = (site(snapshot, (Path(service_root) / handler.file).as_posix(), handler.line,
                                 function=handler.name, kind="handler") if handler else None)
            add(identifier, record, record["original_path"], route.method, kind="route declaration",
                function=handler.name if handler else None, handler_ref=route.handler_text,
                handler_site=handler_site, optional_param=route.optional_param,
                has_validation=route.has_validation,
                validation_digest=None, handler_digest=None)
    return {**inventory(snapshot, paths, entries, gaps), "operations": operations,
            "dependencies": dependencies, "registration_inventory": registrations,
            "outside_registration": outside}


def account_changed_route_regions(previous, current, paths):
    before = {(key, row["file"]) for key, operation in previous["operations"].items() for row in operation["sites"]}
    after = {(key, row["file"]) for key, operation in current["operations"].items() for row in operation["sites"]}
    if not before - after:
        return
    previous_regions = Counter((row["file"], row["text"]) for row in previous["outside_registration"] or [])
    status = {row["path"]: row for row in current["file_status"]}
    for region in current["outside_registration"] or []:
        file, text = region["file"], region["text"]
        if file not in paths or previous_regions[file, text]:
            previous_regions[file, text] -= 1
            continue
        reason = "a changed route-like entry is outside the configured registration call"
        current["unaccounted_scopes"].append(diagnostic(file, reason, region["line"]))
        status[file]["entries"].append(dict(region, kind="unrecognized route", status="partial", reason=reason))
        status[file].update(status="partial", reason=reason)
        if current["schema"]["source"].get("derived"):
            current["content_hash"] = current["schema"]["content_hash"] = None


class RecordingResolver(Resolver):
    def __init__(self, repo, base, config, readable):
        self.readable = readable
        self.dependencies = {}
        super().__init__(repo, base, config)

    def file(self, importer, module):
        resolved = super().file(importer, module)
        if resolved is not None or module.startswith(("./", "../", *(prefix for prefix, _ in self.aliases))):
            if resolved is not None and self.readable.get(resolved.relative_to(self.repo).as_posix()) is None:
                resolved = None
            self.dependencies[importer.relative_to(self.repo).as_posix(), module] = (
                resolved.relative_to(self.repo).as_posix() if resolved is not None else None
            )
        return resolved


def _test_file(file, root):
    path = Path(file)
    return (any(marker in path.name for marker in (".test.", ".spec."))
            or "__tests__" in path.relative_to(root).parts)


def client_calls(snapshot, root, client_root, selected, config, config_source):
    candidates = [file for file in selected if in_root(file, client_root)]
    excluded = [file for file in candidates if _test_file(file, client_root)]
    expected = sorted(set(candidates) - set(excluded))
    gaps, entries, records = [], [], []
    if not any(in_root(file, client_root) for file in snapshot["tree_files"]):
        gaps.append(diagnostic(client_root, f"source root for {snapshot['rev'][:8]} is missing"))
    if config is None:
        reason = (f"client configuration for {snapshot['rev'][:8]} "
                  + ((config_source or {}).get("error") or "is missing"))
        gaps.extend(diagnostic(file, reason, status="unknown") for file in expected or [client_root])
        return {**inventory(snapshot, expected, entries, gaps, excluded),
                "calls": records, "dependencies": []}
    directory = root / client_root
    base = directory.parent if directory.name == "src" and directory != root else directory
    resolver = RecordingResolver(root, base, config, selected)
    files = []
    scans = {}
    for file in expected:
        if selected[file] is None:
            gaps.append(diagnostic(file, "client source could not be read", status="unreadable"))
            continue
        scan = consumer_ts._File(root / file, resolver)
        if scan.tree.has_error:
            gaps.append(diagnostic(file, "client source could not be parsed", status="unreadable"))
            continue
        scans[file] = scan
        files.append(root / file)
    client_paths = {path for path, _ in resolver.clients}
    required = {path for path in client_paths if path.name == "request.ts"} or client_paths
    available = {path for path in required if path.is_relative_to(root)
                 and selected.get(path.relative_to(root).as_posix()) is not None}
    sources = ClientSources(root, config, client_root, files=files, resolver=resolver)
    if required and not available:
        gaps.append(diagnostic(client_root, "the configured primary client source is missing"))
    if available and not sources.has_client_import(available):
        gaps.append(diagnostic(client_root, "no import could be linked to the configured client. "
                               "Uses of this client could have been missed"))
    for file, scan in scans.items():
        for occurrence, observation in enumerate(scan.observations(file), start=1):
            call = observation.call
            source = site(snapshot, file, call.line, end_line=observation.end_line,
                          occurrence=occurrence, byte_span=[observation.start, observation.end],
                          text=observation.expression)
            spec = observation.mapping
            mapping = None
            if spec is not None:
                mapping = {**{key: config_source[key] for key in ("file", "origin", "revision", "selected_for")}, "entry": next(
                    i for i, entry in enumerate(config.clients, start=1) if entry is spec
                ), "strip_prefix": spec.strip_prefix, "add_prefix": spec.add_prefix,
                    "client_entry": next((i for i, entry in enumerate(config.clients, start=1)
                                          if entry is observation.client), None),
                    "delegated_entry": next((i for i, entry in enumerate(config.clients, start=1)
                                             if entry is observation.delegated), None)}
            elif observation.builtin is not None:
                mapping = {"builtin": observation.builtin, "driver": snapshot["tool"]["build_identity"]}
            records.append({
                "site_id": f"{snapshot['input_id']}:{file}:{occurrence}", "site": source,
                "enclosing_function": call.enclosing_function, "function_span": observation.function_span,
                "requested_method": call.method, "original_url_text": call.url_text,
                "mapped_path": call.pattern, "mapping_source": mapping, "target_reason": call.reason,
                "response_type": call.response_type, "url_class": call.url_class,
                "saved_match": None, "field_use": None,
            })
            entries.append({"file": file, "line": call.line, "kind": "client call",
                            "status": "complete", "reason": call.reason, "byte_span": source["byte_span"]})

    @cache
    def search(file, function):
        scope = [p.relative_to(root).as_posix() for p in sources.caller_scope(file, function)]
        return scope, [site(snapshot, row.file, row.line, text=row.text)
                       for row in sources.callers(file, function)]

    for record in records:
        function, file = record["enclosing_function"], record["site"]["file"]
        scope, callers = search(file, function) if function else (None, None)
        record.update(callers_status="complete" if function else "not_checked",
                      callers_reason=None if function else "the function name is missing",
                      caller_method="direct names in the same file and identified importers",
                      depth=1 if function else None, caller_scope=scope, callers=callers)
    dependencies = []
    for (file, imported), resolved in sorted(resolver.dependencies.items()):
        dependencies.append({"importer": file, "import": imported, "path": resolved,
                             "blob": snapshot["tree_files"][resolved]["blob"] if resolved else None,
                             "status": "complete" if resolved else "unknown"})
        if resolved is None and (not Path(imported).suffix or Path(imported).suffix in ts.EXTENSIONS):
            gaps.append(diagnostic(file, f"local import {imported} could not be read"))
    return {**inventory(snapshot, expected, entries, gaps, excluded),
            "calls": records, "dependencies": dependencies}
