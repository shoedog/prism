import json
from pathlib import Path

import pytest

from tier_c.model import Variant
from tier_c.arm_runner import build_codex_cmd, prism_mcp_args, _prism_mcp_config


@pytest.mark.parametrize("runner", ["args", "claude", "codex"])
@pytest.mark.parametrize("cache_mode", ["default", "cached", "no-cache"])
def test_resident_baseline_args(tmp_path, runner, cache_mode):
    cache = str(tmp_path / "cache")
    no_cache = cache_mode == "no-cache"
    cache_dir = None if cache_mode == "default" else cache
    tail = ["--no-cache"] if no_cache else (["--cache-dir", cache] if cache_dir else [])
    expected = ["--repo", "/r", "--warm-at-startup", "--index-idle-secs", "0"] + tail
    if runner == "args":
        actual = prism_mcp_args("/r", no_cache=no_cache, cache_dir=cache_dir)
    elif runner == "claude":
        config = Path(_prism_mcp_config("/r", no_cache=no_cache, cache_dir=cache_dir))
        try:
            actual = json.loads(config.read_text())["mcpServers"]["prism"]["args"]
        finally:
            config.unlink()
    else:
        cmd = build_codex_cmd(Variant("gpt-5.5", True), repo="/r",
                              no_cache=no_cache, cache_dir=cache_dir)
        prefix = "mcp_servers.prism.args="
        args = [json.loads(arg[len(prefix):]) for arg in cmd if arg.startswith(prefix)]
        assert len(args) == 1
        actual = args[0]
    assert actual == expected


@pytest.mark.parametrize("no_cache,cache_dir", [(False, None), (False, "/cache"), (True, "/cache")])
def test_codex_off_ignores_index_settings(no_cache, cache_dir):
    off = build_codex_cmd(Variant("gpt-5.5", False), repo="/r",
                          no_cache=no_cache, cache_dir=cache_dir)
    assert off == build_codex_cmd(Variant("gpt-5.5", False), repo="/r")
