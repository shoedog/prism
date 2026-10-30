"""Suite-wide hermetic doubles for host tool version probes."""
from __future__ import annotations

import pytest

from tests import helpers
from tier_a import bootstrap, commands, launch


@pytest.fixture(autouse=True)
def hermetic_host_tool_versions(monkeypatch):
    launch.authorize(
        {
            "isolated": True,
            "prefix_fresh": True,
            "protocol": launch.PROTOCOL,
        }
    )
    real_commands_version = commands._version
    real_bootstrap_run = bootstrap._run

    versions = {
        ("rust-analyzer", "--version"): helpers.FakeOracle.VERSION,
        ("rustc", "--version"): helpers.FAKE_RUSTC_VERSION,
        ("cargo", "--version"): helpers.FAKE_CARGO_VERSION,
    }

    def command_version(*command: str) -> str:
        return versions.get(tuple(command)) or real_commands_version(*command)

    def bootstrap_run(*command: str, env=None) -> str:
        return versions.get(tuple(command)) or real_bootstrap_run(*command, env=env)

    monkeypatch.setattr(commands, "_version", command_version)
    monkeypatch.setattr(
        commands,
        "_toolchain",
        lambda: {
            "rustc": helpers.FAKE_RUSTC_VERSION,
            "cargo": helpers.FAKE_CARGO_VERSION,
            "python": commands.sys.version.split()[0],
        },
    )
    monkeypatch.setattr(bootstrap, "_run", bootstrap_run)
    monkeypatch.setattr(
        helpers,
        "_command_version",
        lambda *command: versions[tuple(command)],
    )
