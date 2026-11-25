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
from .report_proof import diff


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


def diagnostic(file, reason, line=None, *, status="partial", check="source inventory"):
    return {"file": file, "line": line, "status": status, "reason": reason, "check": check}


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
    registrations, outside, regions = None, None, []
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
        regions = [dict(row, file=(Path(service_root) / row["file"]).as_posix())
                   for row in extraction.registration_regions]
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
            "outside_registration": outside, "registration_regions": regions}


def account_changed_route_regions(previous, current, paths, previous_sources, current_sources):
    prior, following = defaultdict(list), defaultdict(list)
    for snapshot, index in ((previous, prior), (current, following)):
        for region in snapshot["registration_regions"]:
            index[region["file"]].append(region)
    outside = Counter((row["file"], row["text"]) for row in previous["outside_registration"] or [])
    status = {row["path"]: row for row in current["file_status"]}

    def gap(file, region, reason):
        current["unaccounted_scopes"].append(diagnostic(file, reason, region["line"], check="registration region"))
        status[file]["entries"].append(dict(region, kind="unrecognized route", status="partial", reason=reason))
        status[file].update(status="partial", reason=reason)
        if current["schema"]["source"].get("derived"):
            current["content_hash"] = current["schema"]["content_hash"] = None

    for region in current["outside_registration"] or []:
        file, text = region["file"], region["text"]
        if file in paths and not outside[file, text]:
            gap(file, region, "a changed route-like entry is outside the configured registration call")
        else:
            outside[file, text] -= 1
    for file in sorted(set(prior) & set(paths) & status.keys()):
        if status[file]["status"] != "complete":
            continue
        _, after, opcodes = diff(previous_sources.get(file), current_sources.get(file))
        covered = {line for region in following[file] for line in range(region["line"], region["end_line"] + 1)}
        for tag, lo, hi, start, stop in opcodes:
            if tag != "replace" or not any(lo < row["end_line"] and row["line"] - 1 < hi for row in prior[file]):
                continue
            unaccounted = next((i + 1 for i in range(start, stop)
                                if i + 1 not in covered and after[i].strip()
                                and not after[i].lstrip().startswith(("//", "/*", "*"))), None)
            if unaccounted is not None:
                gap(file, {"file": file, "line": unaccounted},
                    "replacement source is outside the configured registration call")
                break


class RecordingResolver(Resolver):
    def __init__(self, repo, base, config, readable):
        self.readable = readable
        self.dependencies = {}
        self.parse_errors = {}
        super().__init__(repo, base, config)

    def file(self, importer, module):
        resolved = super().file(importer, module)
        if resolved is not None or module.startswith(("./", "../", *(prefix for prefix, _ in self.aliases))):
            path = resolved.relative_to(self.repo).as_posix() if resolved is not None else None
            reason = None
            if path is None:
                reason = "the local import could not be resolved to a readable file"
            elif self.readable.get(path) is None:
                reason = "the imported file could not be read"
            elif self.parse_errors.setdefault(path, None) is None:
                self.parse_errors[path] = ts.parse(self.readable[path].encode(), resolved).has_error
            if path and self.parse_errors.get(path):
                reason = "the imported source could not be parsed"
            self.dependencies[importer.relative_to(self.repo).as_posix(), module] = path, reason
            if reason:
                return None
        return resolved


def _test_file(file, root):
    path = Path(file)
    return (any(marker in path.name for marker in (".test.", ".spec."))
            or "__tests__" in path.relative_to(root).parts)


def function_inventory(tree):
    def identity(node):
        if node.type in ts.FUNCTIONS | {"class_declaration", "class", "namespace_declaration"}:
            name = node.child_by_field_name("name")
            if name is None and node.parent.type == "variable_declarator":
                name = node.parent.child_by_field_name("name")
            if name is None:
                return None
            return f"{node.type}:{ts.text(name)}"
        return ""

    for node in ts.walk(tree):
        if node.type not in ts.FUNCTIONS:
            continue
        chain = []
        ancestor = node
        while ancestor is not None:
            part = identity(ancestor)
            if part is None:
                break
            if part:
                chain.append(part)
            ancestor = ancestor.parent
        body = node.child_by_field_name("body")
        if ancestor is None and body is not None:
            yield {"span": [node.start_byte, node.end_byte], "identity": chain[::-1],
                   "header_lines": [node.start_point.row + 1, body.start_point.row + 1]}


def client_calls(snapshot, root, client_root, selected, config, config_source):
    candidates = [file for file in selected if in_root(file, client_root)]
    excluded = [file for file in candidates if _test_file(file, client_root)]
    expected = sorted(set(candidates) - set(excluded))
    gaps, entries, records = [], [], []
    if not any(in_root(file, client_root) for file in snapshot["tree_files"]):
        gaps.append(diagnostic(client_root, f"source root for {snapshot['rev'][:8]} is missing"))
    if config is None:
        failed_config = (config_source or {}).get("error") or "client configuration is missing"
        gaps.append(diagnostic((config_source or {}).get("file") or client_root, failed_config,
                               status="unknown", check="client configuration"))
        gaps.extend(diagnostic(file, failed_config, status="unknown", check="client configuration")
                    for file in expected)
        return {**inventory(snapshot, expected, entries, gaps, excluded),
                "calls": records, "dependencies": [], "functions": {}}
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
        gaps.extend(diagnostic(file, "a client alias could not be resolved to a stable request target", line,
                               check="client call inventory") for line in scan.unsupported_aliases())
        observations = list(scan.observations(file))
        same_line = Counter(row.call.line for row in observations)
        for occurrence, observation in enumerate(observations, start=1):
            call = observation.call
            source = site(snapshot, file, call.line, end_line=observation.end_line,
                          occurrence=occurrence, same_line_occurrences=same_line[call.line], byte_span=[observation.start, observation.end],
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
    for (file, imported), (resolved, reason) in sorted(resolver.dependencies.items()):
        dependencies.append({"importer": file, "import": imported, "path": resolved,
                             "blob": snapshot["tree_files"][resolved]["blob"] if resolved else None,
                             "status": "unknown" if reason else "complete", "reason": reason})
        if reason and (resolved or not Path(imported).suffix or Path(imported).suffix in ts.EXTENSIONS):
            gaps.append(diagnostic(resolved or file, f"{reason} (import {imported} from {file})", check="client import"))
    return {**inventory(snapshot, expected, entries, gaps, excluded),
            "calls": records, "dependencies": dependencies,
            "functions": {file: list(function_inventory(scan.tree)) for file, scan in scans.items()}}
