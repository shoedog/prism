"""Configuration for the TypeScript and JavaScript consumer adapter."""

from dataclasses import dataclass, field
from pathlib import Path
import tomllib


MethodSpec = str | dict[str, str | int]


@dataclass(frozen=True)
class ClientSpec:
    file: str | None = None
    export: str = "default"
    methods: dict[str, MethodSpec] = field(default_factory=dict)
    call: MethodSpec | None = None
    hook: str | None = None
    fetcher_arg: int = 1
    url_arg: int = 0
    strip_prefix: str = ""
    add_prefix: str = ""
    response_root: str = ""


@dataclass(frozen=True)
class Config:
    aliases: dict[str, str] = field(default_factory=dict)
    clients: list[ClientSpec] = field(default_factory=list)


def load(path: str | Path) -> Config:
    with Path(path).open("rb") as stream:
        document = tomllib.load(stream)
    return Config(document.get("aliases", {}), [ClientSpec(**entry) for entry in document.get("client", [])])
