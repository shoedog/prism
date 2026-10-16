"""Overlay only read-only capture support on a freshly archived main tree."""
from pathlib import Path
import shutil
import sys

source = Path(__file__).resolve().parents[2]
main = Path(sys.argv[1]).resolve()
assert main != source, "capture must use an independent main archive"
adapter = "src/cpg_cache/dfg_observation.rs"
assert not (main / adapter).exists(), "use a fresh archive, not an existing overlay"
manifest = main / "Cargo.toml"
assert 'name = "dfg_capture"' not in manifest.read_text(), "already overlaid"
for name in [adapter, "tests/dfg_compaction/cases.rs", "tests/dfg_compaction/capture.rs"]:
    target = main / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source / name, target)
facade = main / "src/cpg.rs"
text = facade.read_text()
text += '\n#[cfg(feature = "dfg-oracle")]\n#[path = "cpg_cache/dfg_observation.rs"]\npub mod dfg_compaction_support;\n'
facade.write_text(text)
text = manifest.read_text()
if 'dfg-oracle = []' not in text:
    text = text.replace("[features]", "[features]\ndfg-oracle = []")
text += '\n[[test]]\nname = "dfg_capture"\npath = "tests/dfg_compaction/capture_main.rs"\nrequired-features = ["dfg-oracle"]\n'
manifest.write_text(text)
(main / "tests/dfg_compaction/capture_main.rs").write_text(
    '#[path = "../common/mod.rs"]\nmod common;\nmod cases;\nmod capture;\n'
)
