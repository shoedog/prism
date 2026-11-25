# subscript_recall: independent inventory for lane 10's C3 audit

Measurement-only tool (no product code). It walks pinned source and implements DESIGN sections 4 to 5 of
`docs/superpowers/plans/2026-11-20-lanes/lane10-keyed-subscripts/` independently of any product pass: element writes W,
whole-container reads R, per-pair route verdicts, raw-pair work, the three caps and the cost flags of section 6.

- Python files use the stdlib `ast`. JS, TS, TSX and Go use the Python `tree-sitter` bindings, which are NOT a
  dependency of the repository or of `eval/`: install them in a throwaway environment (do not touch `eval/uv.lock`):
  `uv run --with tree-sitter==0.23.2 --with tree-sitter-go==0.23.4 --with tree-sitter-javascript==0.23.1
  --with tree-sitter-typescript==0.23.2 python3 -B -m unittest` from this directory (these are the versions the
  recorded run used; they differ from the grammars prism vendors, a deliberate independence).
- Tests: `python3 -B -m unittest` from this directory (42 tests).
- Entry point: `inventory.py` (see `VERIFICATION.md` for the run recorded for the stage M audit).
