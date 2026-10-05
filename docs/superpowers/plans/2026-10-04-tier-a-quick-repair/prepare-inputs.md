# Offline quick inputs

No Git writes, clones, fetches, or installs are needed. Excalidraw is the owner's
existing public archive at `~/prism-evidence/inputs/excalidraw-0642e72c/source`.
Its adjacent `source-file-manifest.sha256` is pinned to
`1a6c4397bfacafc0dc2651d1f65fe4dd7afed60a0fa21f8f928061b9e20968f0`.

The recovered Prism source is an archive of the **original** baseline commit,
not a new baseline. This recipe was executed locally (864 files, 20,639,908 bytes).
Use an empty destination; do not overwrite an existing snapshot with differing
bytes. The manifest is outside `source`, so it does not include itself.

```bash
task_snapshot="$HOME/.local/share/prism/corpora/prism-20c8490591a3"
mkdir -p "$task_snapshot/source"
git -C /Users/wesleyjinks/code/prism-tiera archive \
  20c8490591a379227b70c1d5ec4c75ff4b64ff01 | tar -x -C "$task_snapshot/source"
```

With Python 3.12, before starting rust-analyzer or generating `target` files:

```python
import hashlib
from pathlib import Path
root = Path.home() / ".local/share/prism/corpora/prism-20c8490591a3"
source = root / "source"
files = sorted(p for p in source.rglob("*") if p.is_file())
body = "".join(hashlib.sha256(p.read_bytes()).hexdigest() + "  "
               + p.relative_to(source).as_posix() + "\n" for p in files)
assert hashlib.sha256(body.encode()).hexdigest() == (
    "9c0598b8a737df30dd329ef2175498c8ce12e5ce2ecb82e5fbf20b8003e4b4f8")
(root / "source-file-manifest.sha256").write_text(body)
```

The runner verifies the manifest and complete source contents before and after
each corpus. Generated `target`, `node_modules`, `.venv`, and `.git` trees are
outside this source census. Neither quick identity nor validity is overridden by
`--allow-stale-sut`; that switch only accepts the explicitly rebuilt dirty SUT.
