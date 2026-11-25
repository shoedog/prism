"""Operation and field identities shared by the feasibility tools."""

import re


_PARAMETER = re.compile(r"\{[^/{}]*\}|(?<=/):[\w-]+")
_SLASHES = re.compile(r"/+")


def normalize_path(raw: str, base: str = "") -> str:
    path = _SLASHES.sub("/", f"/{base}/{raw}").rstrip("/") or "/"
    return _PARAMETER.sub("{}", path)


def op_id(method: str, path: str, base: str = "") -> str:
    return f"op:{method.upper()} {normalize_path(path, base)}"


def field_id(root: str, pointer: list[str]) -> str:
    return f"field:{root}#/{'/'.join(pointer)}"
