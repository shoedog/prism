from adoption.runner import build_claude_cmd, cache_key

def test_build_claude_cmd_sonnet_with_mcp():
    cmd = build_claude_cmd(prompt="hi", mcp_cfg="/m.json", model="sonnet")
    assert cmd[:6] == ["claude","-p","--output-format","stream-json","--verbose"][:5] + ["--model"]
    assert "--mcp-config" in cmd and "/m.json" in cmd and "--strict-mcp-config" in cmd
    assert cmd[-1] == "hi"

def test_cache_key_changes_with_skill_hash():
    a = cache_key(skill_bytes=b"v1", probe_id="p", trial=0, model="sonnet")
    b = cache_key(skill_bytes=b"v2", probe_id="p", trial=0, model="sonnet")
    assert a != b
    assert a == cache_key(skill_bytes=b"v1", probe_id="p", trial=0, model="sonnet")
