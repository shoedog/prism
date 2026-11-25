"""Recognize TypeScript and JavaScript HTTP client call sites."""

from dataclasses import dataclass
from pathlib import Path

from tree_sitter import Node

from . import _ts_syntax as ts
from ._ts_imports import Import, Resolver
from ._ts_scope import Bindings
from ._ts_urls import pattern as pattern, url_pattern
from .config import ClientSpec, Config, MethodSpec


_HTTP_VERBS = {"get", "post", "put", "delete", "patch", "head", "options", "trace"}
_AXIOS_HELPERS = {"all", "create", "getUri", "isAxiosError", "isCancel", "spread", "toFormData"}
_GLOBAL_OBJECTS = {"globalThis", "self", "window"}


@dataclass(frozen=True)
class ClientCall:
    file: str
    line: int
    method: str | None
    url_text: str
    pattern: str | None
    url_class: str
    reason: str | None
    response_type: str | None
    enclosing_function: str | None


@dataclass(frozen=True)
class CallObservation:
    call: ClientCall
    start: int
    end: int
    end_line: int
    expression: str
    function_span: tuple[int, int] | None
    mapping: ClientSpec | None
    client: ClientSpec | None
    delegated: ClientSpec | None
    builtin: str | None


@dataclass(frozen=True)
class _Target:
    kind: str
    members: tuple[str | None, ...]
    spec: ClientSpec | None = None


@dataclass(frozen=True)
class _Request:
    method: str | None
    url: Node | None
    spec: ClientSpec | None = None
    reason: str | None = None


def calls(repo: Path, config: Config, root: str) -> list[ClientCall]:
    repo = repo.resolve()
    source_root = (repo / root).resolve()
    if not source_root.is_relative_to(repo) or not source_root.is_dir():
        raise ValueError(f"consumer root is not a directory under the repository: {root}")
    # Config paths are project-relative, e.g. src/utils/request.ts when scanning ui/src.
    base = source_root.parent if source_root.name == "src" and source_root != repo else source_root
    resolver = Resolver(repo, base, config)
    return [
        call for path in source_paths(source_root)
        for call in _File(path, resolver).calls(path.relative_to(repo).as_posix())
    ]


def source_paths(source_root: Path):
    """Yield the production JavaScript and TypeScript inputs of the adapter."""
    for path in sorted(source_root.rglob("*")):
        if path.suffix not in ts.EXTENSIONS or path.is_symlink() or not path.is_file():
            continue
        if any(marker in path.name for marker in (".test.", ".spec.")) or (
            "__tests__" in path.relative_to(source_root).parts
        ):
            continue
        yield path


def _method(rule: MethodSpec, args: list[Node]) -> str | None:
    if isinstance(rule, str):
        return rule.upper()
    if "method" in rule:
        return str(rule["method"]).upper()
    if "method_arg" in rule:
        node = ts.argument(args, int(rule["method_arg"]))
    else:
        options = ts.argument(args, int(rule.get("options_arg", 1)))
        node, _ = ts.property_value(options, str(rule["method_key"]))
    value = ts.literal(node)
    return value.upper() if value else None


def _client_request(target: _Target, args: list[Node]) -> _Request:
    spec = target.spec
    assert spec is not None
    url = ts.argument(args, spec.url_arg)
    if not target.members:
        rule = spec.call
        unknown = "unconfigured_client_form"
    else:
        path = ".".join(target.members) if all(part and "." not in part for part in target.members) else None
        rule = spec.methods.get(path) if path is not None else None
        unknown = "unknown_client_method"
        options_url, _ = ts.property_value(url, "url")
        if options_url is not None or (path is not None and any(m.startswith(path + ".") for m in spec.methods)):
            unknown = "unconfigured_client_form"
    if rule is None:
        return _Request(None, url, spec, unknown)
    verb = _method(rule, args)
    return _Request(verb, url, spec, None if verb else "dynamic_method")


def _request(target: _Target, args: list[Node]) -> _Request:
    url = ts.argument(args, 0)
    if target.kind == "client":
        return _client_request(target, args)
    if target.kind == "fetch":
        if len(args) < 2 and (not args or url is not None):
            return _Request("GET", url)
        node, _ = ts.property_value(ts.argument(args, 1), "method")
        method = ts.literal(node)
        return _Request(method.upper() if method else None, url, reason=None if method else "dynamic_method")
    if target.kind == "axios_factory":
        return _Request(None, url, reason="unconfigured_client_form")
    if not target.members or target.members == ("request",):
        options = ts.unwrap(url)
        if options is None or options.type != "object":
            return _Request(None, options, reason="unconfigured_client_form")
        method_node, known = ts.property_value(options, "method")
        method = ("GET" if known and method_node is None else ts.literal(method_node))
        url, _ = ts.property_value(options, "url")
        return _Request(method.upper() if method else None, url, reason=None if method else "dynamic_method")
    member = target.members[0]
    if len(target.members) == 1 and member in _HTTP_VERBS:
        return _Request(member.upper(), url)
    return _Request(None, url, reason="unknown_client_method")


class _File:
    def __init__(self, path: Path, resolver: Resolver):
        self.path = path
        self.resolver = resolver
        self.tree = ts.parse(path.read_bytes(), path)
        self.bindings = Bindings(self.tree)

    def _axios(self, imported: Import, members: tuple[str | None, ...]) -> _Target | None:
        if imported.export not in {"default", "*"}:
            members = (imported.export, *members)
        elif imported.export == "*" and members[:1] == ("default",):
            members = members[1:]
        if members[:1] and members[0] in _AXIOS_HELPERS:
            return None
        return _Target("axios", members)

    def _factory(self, node: Node | None) -> bool:
        node = ts.unwrap(node)
        if node is None or node.type != "call_expression":
            return False
        base, members = ts.member_chain(node.child_by_field_name("function"))
        if base is None or base.type != "identifier":
            return False
        binding = self.bindings.find(base)
        if binding is None:
            return ts.text(base) == "axios" and members == ("create",)
        if binding.written or binding.imported is None or binding.imported.module != "axios":
            return False
        export = binding.imported.export
        return (export in {"default", "*"} and members == ("create",)) or (export == "create" and not members)

    def target(self, callee: Node | None) -> _Target | None:
        root, members = ts.member_chain(callee)
        if root is None:
            return None
        if self._factory(root):
            return _Target("axios_factory", members)
        if root.type != "identifier":
            return None
        while (binding := self.bindings.find(root)) is not None:
            alias = ts.unwrap(binding.initializer)
            if (not binding.constant or binding.written or alias is None
                    or alias.type != "identifier" or alias.start_byte >= root.start_byte):
                break
            root = alias
        local = ts.text(root)
        if binding is None:
            if (local == "fetch" and not members) or (local in _GLOBAL_OBJECTS and members == ("fetch",)):
                return _Target("fetch", ())
            if local == "axios":
                return self._axios(Import("axios", "default"), members)
            hook = self.resolver.hooks.get(local)
            return _Target("hook", (), hook) if hook is not None and not members else None
        if binding.written:
            return None
        imported = binding.imported
        if imported is None:
            if self._factory(binding.initializer):
                return _Target("axios_factory", members)
            return None
        if imported.module == "axios":
            return self._axios(imported, members)

        if imported.export == "*" and members and members[0] is not None:
            imported = Import(imported.module, members[0])
            members = members[1:]
        spec = self.resolver.client(self.path, imported)
        if spec is not None:
            return _Target("client", members, spec)
        hook_name = imported.export
        if imported.export == "default":
            hook_name = "useSWR" if imported.module == "swr" else local
        hook = self.resolver.hooks.get(hook_name)
        return _Target("hook", (), hook) if hook is not None and not members else None

    def _hook(self, target: _Target, args: list[Node]) -> tuple[_Request, Node | None, ClientSpec | None]:
        hook = target.spec
        url = ts.argument(args, hook.url_arg)
        fetcher = ts.unwrap(ts.argument(args, hook.fetcher_arg))
        inline = fetcher if fetcher is not None and fetcher.type in ts.FUNCTIONS else None
        request = None
        if inline is not None:
            for result in _returned_calls(inline):
                called = self.target(result.child_by_field_name("function"))
                if called is None or called.kind == "hook":
                    continue
                delegated = _request(called, ts.children(result.child_by_field_name("arguments")))
                if request is not None and (request.method, request.spec, request.reason) != (
                    delegated.method, delegated.spec, delegated.reason
                ):
                    return _Request(None, url, hook, "dynamic_method"), inline, None
                request = delegated
        else:
            reference = self.target(fetcher)
            if reference is not None and reference.kind != "hook":
                request = _request(reference, [])
        if request is None:
            return _Request("GET", url, hook), inline, None
        spec = hook if hook.strip_prefix or hook.add_prefix else request.spec or hook
        return _Request(request.method, url, spec, request.reason), inline, request.spec

    def unsupported_aliases(self):
        for node in ts.walk(self.tree):
            if node.type != "call_expression":
                continue
            callee = node.child_by_field_name("function")
            root, _ = ts.member_chain(callee)
            if root is None or root.type != "identifier" or self.target(callee) is not None:
                continue
            binding = self.bindings.find(root)
            initializer = ts.unwrap(binding.initializer) if binding else None
            if initializer is not None and self.target(initializer) is not None:
                yield node.start_point.row + 1

    def calls(self, file: str) -> list[ClientCall]:
        return [observation.call for observation in self.observations(file)]

    def observations(self, file: str):
        fetchers = []
        for node in ts.walk(self.tree):
            if node.type != "call_expression":
                continue
            while fetchers and node.start_byte >= fetchers[-1].end_byte:
                fetchers.pop()
            if fetchers and node.start_byte >= fetchers[-1].start_byte:
                continue
            target = self.target(node.child_by_field_name("function"))
            if target is None:
                continue
            args = ts.children(node.child_by_field_name("arguments"))
            hook_key = target.kind == "hook"
            delegated = None
            if hook_key:
                request, inline, delegated = self._hook(target, args)
                if inline is not None:
                    fetchers.append(inline)
            else:
                request = _request(target, args)
            path, url_class, url_reason = url_pattern(
                request.url, self.bindings.constant, request.spec, hook_key=hook_key,
            )
            type_args = ts.children(node.child_by_field_name("type_arguments"))
            call = ClientCall(
                file, node.start_point.row + 1, request.method, ts.text(request.url), path, url_class,
                request.reason or url_reason, ts.text(type_args[0]) if type_args else None,
                self.bindings.enclosing_function(node),
            )
            function = node.parent
            while function is not None and function.type not in ts.FUNCTIONS:
                function = function.parent
            span = (function.start_byte, function.end_byte) if function is not None else None
            yield CallObservation(call, node.start_byte, node.end_byte, node.end_point.row + 1,
                                  ts.text(node), span, request.spec, target.spec, delegated,
                                  target.kind if target.spec is None else None)


def _returned_calls(function: Node):
    body = ts.unwrap(function.child_by_field_name("body"))
    if body is None:
        return
    if body.type == "call_expression":
        yield body
    elif body.type == "statement_block":
        pending = list(reversed(ts.children(body)))
        while pending:
            node = pending.pop()
            if node.type in ts.FUNCTIONS:
                continue
            if node.type == "return_statement":
                result = ts.unwrap(next(iter(ts.children(node)), None))
                if result is not None and result.type == "call_expression":
                    yield result
            else:
                pending.extend(reversed(ts.children(node)))
