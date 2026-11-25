"""Client import, property spelling, and caller evidence for history replay."""

from collections import defaultdict
from functools import cache
from pathlib import Path

from . import _ts_syntax as ts
from ._replay_git import SourceLine
from ._ts_imports import Import, Resolver, imports
from ._ts_scope import Bindings
from .config import Config
from .consumer_ts import ClientCall, source_paths


def _property(node):
    if node.type in {"member_expression", "subscript_expression"}:
        prop = node.child_by_field_name("property") or node.child_by_field_name("index")
    elif node.type in {"pair", "pair_pattern", "property_signature", "method_definition",
                       "public_field_definition", "field_definition"}:
        prop = node.child_by_field_name("key") or node.child_by_field_name("name")
    elif node.type in {"shorthand_property_identifier", "shorthand_property_identifier_pattern"}:
        prop = node
    else:
        return None, None
    if prop is not None and prop.type == "computed_property_name":
        prop = next(iter(ts.children(prop)), None)
        return ts.literal(prop), prop
    if node.type == "subscript_expression":
        return ts.literal(prop), prop
    return ts.literal(prop) if prop is not None and prop.type == "string" else ts.text(prop), prop


def _export_names(statement):
    for clause in ts.children(statement):
        if clause.type != "export_clause":
            continue
        for item in ts.children(clause):
            name = item.child_by_field_name("name")
            alias = item.child_by_field_name("alias") or name
            yield ts.literal(alias) or ts.text(alias), ts.literal(name) or ts.text(name)


class ClientSources:
    def __init__(self, repo: Path, config: Config, root: str):
        repo = repo.resolve()
        self.repo = repo
        source_root = (repo / root).resolve()
        base = source_root.parent if source_root.name == "src" and source_root != repo else source_root
        self.resolver = Resolver(repo, base, config)
        self.trees = {}
        self.imports = {}
        self.sources = {}
        self.importers = defaultdict(lambda: defaultdict(set))
        self.properties = {}
        for path in source_paths(source_root):
            source = path.read_bytes()
            tree = ts.parse(source, path)
            self.trees[path] = tree
            self.sources[path] = source.decode().split("\n")
            direct = [pair for statement in ts.children(tree) if statement.type == "import_statement"
                      for pair in imports(statement)]
            self.imports[path] = direct
        self._local_exports = cache(self._local_exports)
        self._export_origins = cache(self._export_origins)
        for path, direct in self.imports.items():
            for _, imported in direct:
                target = self.resolver.file(path, imported.module)
                if target is not None and imported.export not in {"*", "default"}:
                    for origin in self._export_origins(target, imported.export):
                        self.importers[origin][path].add(imported)

    def _tree(self, path: Path):
        return self.trees.get(path) or ts.parse(path.read_bytes(), path)

    def _local_exports(self, path: Path) -> dict[str, str]:
        result = {}
        for statement in ts.children(self._tree(path)):
            if statement.type != "export_statement" or statement.child_by_field_name("source") is not None:
                continue
            if any(child.type in {"type", "default"} for child in statement.children):
                continue
            result.update(_export_names(statement))
            declaration = statement.child_by_field_name("declaration")
            if declaration is None:
                continue
            declarations = (ts.children(declaration) if declaration.type in
                            {"lexical_declaration", "variable_declaration"} else [declaration])
            for declared in declarations:
                name = declared.child_by_field_name("name")
                if name is not None:
                    result[ts.text(name)] = ts.text(name)
        return result

    def _export_origins(self, target: Path, export: str) -> set[tuple[Path, str]]:
        # Named wrappers can be forwarded through multiple barrels. The client
        # adapter's own import grammar still uses its original one-barrel rule.
        origins, visited = set(), set()
        pending = [(target, export)]
        while pending:
            path, name = pending.pop()
            if (path, name) in visited:
                continue
            visited.add((path, name))
            local = self._local_exports(path).get(name)
            if local is not None:
                origins.add((path, local))
                continue
            for remote, module in self.resolver.reexports(path, name):
                destination = self.resolver.file(path, module)
                if destination is not None:
                    pending.append((destination, remote))
        return origins if len(origins) == 1 else set()

    def _namespace_names(self, path: Path) -> set[str]:
        names = {export for _, export in self.resolver.clients if export != "default"}
        for statement in ts.children(self._tree(path)):
            if statement.type == "export_statement":
                names.update(alias for alias, _ in _export_names(statement))
        return names

    def has_client_import(self, client_paths: set[Path]) -> bool:
        for path, bindings in self.imports.items():
            for _, imported in bindings:
                target = self.resolver.file(path, imported.module)
                if target in client_paths:
                    return True
                names = {imported.export}
                if target and imported.export == "*":
                    names = self._namespace_names(target)
                for name in names:
                    spec = self.resolver.client(path, Import(imported.module, name))
                    if spec and (self.resolver.base / spec.file).resolve() in client_paths:
                        return True
        return False

    def _line(self, path: Path, row: int) -> SourceLine:
        return SourceLine(path.relative_to(self.repo).as_posix(), row + 1, self.sources[path][row])

    def property_lines(self, call: ClientCall, field: str) -> list[SourceLine]:
        path = self.repo / call.file
        files = {path} | self.importers[path, call.enclosing_function].keys()
        result = []
        for file in sorted(files):
            if file not in self.properties:
                found = defaultdict(set)
                for node in ts.walk(self.trees[file]):
                    name, prop = _property(node)
                    if prop is not None:
                        found[name].add(prop.start_point.row)
                self.properties[file] = found
            for row in sorted(self.properties[file].get(field, ())):
                result.append(self._line(file, row))
        return result

    def callers(self, file: str, function: str | None) -> list[SourceLine]:
        """Search direct calls by name and by resolved named import aliases."""
        if not function:
            return []
        target = self.repo / file
        importers = self.importers[target, function]
        result = []
        for path in sorted({target} | importers.keys()):
            if path not in self.trees:
                continue
            bindings = Bindings(self.trees[path])
            callers = set()
            for node in ts.walk(self.trees[path]):
                if node.type != "call_expression":
                    continue
                callee, members = ts.member_chain(node.child_by_field_name("function"))
                if callee is None or callee.type != "identifier" or members:
                    continue
                binding = bindings.find(callee)
                if binding is None or binding.written:
                    continue
                imported = binding.imported
                if imported is not None:
                    if imported not in importers.get(path, ()):
                        continue
                elif path != target or ts.text(callee) != function:
                    continue
                callers.add(node.start_point.row)
            result.extend(self._line(path, row) for row in sorted(callers))
        return result
