"""Run-artifact store (spec §4): persists a run under runs/<run-id>/ for deterministic replay +
audit. JSON, diff-able. gitignored. Replay (Phase-1d-replay) consumes this."""
from __future__ import annotations
import json, os


class RunStore:
    def __init__(self, root: str, run_id: str, manifest: dict):
        self.dir = os.path.join(root, run_id)
        self.manifest = manifest

    def ensure_new(self, force: bool = False):
        if os.path.exists(self.dir) and not force:
            raise FileExistsError(f"run-id dir exists: {self.dir} (use --force-new)")
        os.makedirs(os.path.join(self.dir, "stages"), exist_ok=True)

    def _write(self, rel: str, obj):
        p = os.path.join(self.dir, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as f:
            json.dump(obj, f, indent=1, default=str)

    def write_manifest(self):
        os.makedirs(self.dir, exist_ok=True)
        self._write("manifest.json", self.manifest)

    def write_stage_artifact(self, stage: str, name: str, obj):
        self._write(os.path.join("stages", stage, f"{name}.json"), obj)

    def write_root_artifact(self, name: str, obj):
        """Write <run-dir>/<name>.json (report, detectability, etc.)."""
        self._write(f"{name}.json", obj)
