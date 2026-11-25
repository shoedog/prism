"""Operation and field identities shared by the feasibility tools."""

from collections.abc import Collection, Iterable
from enum import Enum
from itertools import product
import re
from urllib.parse import quote, urlsplit


_PARAMETER = re.compile(r"\{[^/{}]*\}|(?<=/):[\w-]+")
_SLASHES = re.compile(r"/+")
_BRACES = re.compile(r"\{([^/{}]+)\}")
_GIN_CAPTURE = re.compile(r":[\w-]+|\*[\w-]+$")
_EXPRESS_CAPTURE = re.compile(r":([\w]+)(\?)?|\*(?=/|$)")
_EXPRESS5_CAPTURE = re.compile(r'\{\*[\w]+\}|\*[\w]+|:(?:"[^"]+"|[\w]+)')
_HAPI_CAPTURE = re.compile(r"\{([^/{}*?]+)(\?|\*[1-9][0-9]*|\*)?\}")
_SERVER_VARIABLE = re.compile(r"\{([^{}]+)\}")


class Edge(Enum):
    ITEM = "[]"
    MAP_VALUE = "*"


def normalize_path(
    raw: str, base: str = "", *, dialect: str | None = None, labels: Collection[str] = (),
) -> str:
    if dialect is None:
        # Existing feasibility adapters use their original shared projection.
        path = _SLASHES.sub("/", f"/{base}/{raw}").rstrip("/") or "/"
        return _PARAMETER.sub("{}", path)
    path = raw if raw.startswith("/") else f"/{raw}"
    prefix = base if base.startswith("/") else f"/{base}"
    if dialect == "schema":
        path = _BRACES.sub(lambda match: ".{}" if match[1] in labels else "{}", path)
        return prefix + path if prefix != "/" else path

    path = prefix + path if prefix != "/" else path
    if dialect == "gin":
        return _GIN_CAPTURE.sub(lambda match: "{*0+}" if match[0].startswith("*") else "{}", path)
    if dialect == "express4":
        return _EXPRESS_CAPTURE.sub(
            lambda match: "{*0+}" if match[0] == "*" else "{?}" if match[2] else "{}", path,
        )
    if dialect == "express5":
        spelling = {"{": "{*0+}", "*": "{*}", ":": "{}"}
        return _EXPRESS5_CAPTURE.sub(lambda match: spelling[match[0][0]], path)
    if dialect == "hapi":
        def capture(match: re.Match) -> str:
            suffix = match[2] or ""
            return "{*0+}" if suffix == "*" else "{" + suffix + "}"

        return _HAPI_CAPTURE.sub(capture, path)
    raise ValueError(f"Unknown path dialect {dialect!r}")


def op_id(
    method: str, path: str, base: str = "", *, dialect: str | None = None, labels: Collection[str] = (),
) -> str:
    return f"op:{method.upper()} {normalize_path(path, base, dialect=dialect, labels=labels)}"


def field_id(root: str, pointer: Iterable[str | Edge]) -> str:
    parts = (
        part.value if isinstance(part, Edge) else quote(part.replace("~", "~0").replace("/", "~1"), safe="")
        for part in pointer
    )
    return f"field:{root}#/{'/'.join(parts)}"


def _resolve_relative_path(path: str, retrieval_path: str) -> str:
    if not path:
        return retrieval_path
    if not path.startswith("/"):
        path = f"{retrieval_path.rpartition('/')[0]}/{path}"
    segments = []
    for segment in path[1:].split("/"):
        if segment == "..":
            if segments:
                segments.pop()
        elif segment != ".":
            segments.append(segment)
    if path.endswith(("/.", "/..")):
        segments.append("")
    return "/" + "/".join(segments)


def server_paths(
    servers: list[dict], retrieval_url: str | None, warnings: set[str],
) -> set[str]:
    if not servers:
        return {"/"}
    paths = set()
    for server in servers:
        url = server["url"]
        names = dict.fromkeys(_SERVER_VARIABLE.findall(url))
        values = []
        for name in names:
            variable = server.get("variables", {}).get(name, {})
            if not variable.get("enum"):
                warnings.add(f"server_variable_unchecked {url} {name}")
            values.append(variable.get("enum") or ([variable["default"]] if "default" in variable else []))
        for combination in product(*values):
            selected = dict(zip(names, combination))
            expanded = _SERVER_VARIABLE.sub(lambda match: selected[match[1]], url)
            parts = urlsplit(expanded)
            if parts.scheme or parts.netloc:
                paths.add(parts.path or "/")
            elif retrieval_url:
                paths.add(_resolve_relative_path(parts.path, urlsplit(retrieval_url).path))
            elif parts.path.startswith("/"):
                paths.add(_resolve_relative_path(parts.path, "/"))
            else:
                warnings.add(f"server_base_unchecked {url}")
    return paths
