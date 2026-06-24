import json, os, pytest
from tier_c.store import RunStore

def test_store_writes_manifest_and_stage_and_rejects_collision(tmp_path):
    root = str(tmp_path / "runs")
    s = RunStore(root, run_id="r1", manifest={"models": ["opus-4.8"], "prism_sha": "abc"})
    s.write_manifest()
    assert json.load(open(os.path.join(root, "r1", "manifest.json")))["prism_sha"] == "abc"
    s.write_stage_artifact("spec", "prompt", {"text": "P", "upstream": ""})
    assert json.load(open(os.path.join(root, "r1", "stages", "spec", "prompt.json")))["text"] == "P"
    s.write_stage_artifact("spec", "seeds", {"shuffle": "spec|x", "tiebreak": "spec|x"})
    with pytest.raises(FileExistsError):
        RunStore(root, run_id="r1", manifest={}).ensure_new()
