import os, subprocess, json
from tier_c.lspshim import make_lsp_deny_shim, DENIED

def test_shim_dir_has_stub_for_each_denied(tmp_path):
    log = str(tmp_path / "shim.jsonl")
    d = make_lsp_deny_shim(log)
    for tool in DENIED:
        assert os.access(os.path.join(d, tool), os.X_OK)

def test_stub_logs_and_fails(tmp_path):
    log = str(tmp_path / "shim.jsonl")
    d = make_lsp_deny_shim(log)
    env = {**os.environ, "PATH": d + os.pathsep + os.environ["PATH"]}
    r = subprocess.run(["pyright", "foo.py"], capture_output=True, text=True, env=env)
    assert r.returncode != 0
    assert "disabled" in r.stderr.lower()
    rec = json.loads(open(log).read().splitlines()[0])
    assert rec["tool"] == "pyright"
