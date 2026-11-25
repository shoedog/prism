"""Resolve client imports by file and export, following one barrel."""

from dataclasses import dataclass
from functools import cache
from pathlib import Path

from tree_sitter import Node

from . import _ts_syntax as ts
from .config import ClientSpec, Config


@dataclass(frozen=True)
class Import:
    module: str
    export: str


def imports(statement: Node):
    """Yield local names and their value imports from an import statement."""
    if any(child.type == "type" for child in statement.children):
        return
    for clause in ts.children(statement):
        if clause.type == "import_require_clause":
            name = ts.children(clause)[0]
            source = ts.literal(clause.child_by_field_name("source"))
            if source is not None:
                yield ts.text(name), Import(source, "default")
        if clause.type != "import_clause":
            continue
        source = ts.literal(statement.child_by_field_name("source"))
        if source is None:
            continue
        for entry in ts.children(clause):
            if entry.type == "identifier":
                yield ts.text(entry), Import(source, "default")
            elif entry.type == "namespace_import":
                yield ts.text(ts.children(entry)[0]), Import(source, "*")
            elif entry.type == "named_imports":
                for item in ts.children(entry):
                    if any(child.type == "type" for child in item.children):
                        continue
                    name = item.child_by_field_name("name")
                    local = item.child_by_field_name("alias") or name
                    yield ts.text(local), Import(source, ts.literal(name) or ts.text(name))


class Resolver:
    def __init__(self, repo: Path, base: Path, config: Config):
        self.repo = repo
        self.base = base
        self.aliases = sorted(config.aliases.items(), key=lambda entry: -len(entry[0]))
        self.clients = {
            ((base / client.file).resolve(), client.export): client
            for client in config.clients if client.file is not None
        }
        self.hooks = {client.hook: client for client in config.clients if client.hook is not None}
        self.file = cache(self.file)
        self.client = cache(self.client)
        self.reaches_client = cache(self.reaches_client)
        self.reexports = cache(self.reexports)

    def file(self, importer: Path, module: str) -> Path | None:
        if module.startswith(("./", "../")):
            candidate = importer.parent / module
        else:
            for prefix, directory in self.aliases:
                if module.startswith(prefix):
                    candidate = self.base / directory / module[len(prefix):]
                    break
            else:
                return None
        if candidate.suffix in ts.EXTENSIONS:
            candidates = [candidate]
        else:
            candidates = [Path(str(candidate) + suffix) for suffix in ts.EXTENSIONS]
            candidates.extend(candidate / ("index" + suffix) for suffix in ts.EXTENSIONS)
        for path in candidates:
            resolved = path.resolve()
            if resolved.is_relative_to(self.repo) and resolved.is_file():
                return resolved
        return None

    def client(self, importer: Path, imported: Import) -> ClientSpec | None:
        path = self.file(importer, imported.module)
        if path is None:
            return None
        direct = self.clients.get((path, imported.export))
        if direct is not None:
            return direct

        for name, source in self.reexports(path, imported.export):
            target = self.file(path, source)
            spec = self.clients.get((target, name))
            if spec is not None:
                return spec
        return None

    def reaches_client(self, importer: Path, imported: Import) -> bool:
        """Find configured bindings even when the call extractor cannot follow the import chain."""
        pending = [(self.file(importer, imported.module), imported.export)]
        seen = set()
        while pending:
            path, export = pending.pop()
            if path is None or (path, export) in seen:
                continue
            seen.add((path, export))
            if (path, export) in self.clients or (
                export == "*" and any(candidate == path for candidate, _ in self.clients)
            ):
                return True
            pending.extend((self.file(path, source), name) for name, source in self.reexports(path, export))
        return False

    def reexports(self, path: Path, export: str) -> tuple[tuple[str, str], ...]:
        tree = ts.parse(path.read_bytes(), path)
        result = []
        for statement in ts.children(tree):
            if statement.type != "export_statement" or any(c.type == "type" for c in statement.children):
                continue
            source = ts.literal(statement.child_by_field_name("source"))
            if source is None:
                continue
            if export != "default" and any(child.type == "*" for child in statement.children):
                result.append((export, source))
            for clause in ts.children(statement):
                if clause.type != "export_clause":
                    continue
                for item in ts.children(clause):
                    if any(child.type == "type" for child in item.children):
                        continue
                    name = item.child_by_field_name("name")
                    alias = item.child_by_field_name("alias") or name
                    if (ts.literal(alias) or ts.text(alias)) == export:
                        result.append((ts.literal(name) or ts.text(name), source))
        return tuple(result)
