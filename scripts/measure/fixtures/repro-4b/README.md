# R4B fixture and retained capture

M-3 generated the 115 measured source files in `repo/` from the alias-kill
test tables at `1f2ad1cc`. The source files and the independent
`truth.jsonl` are copied from that generator's output. The table supplies
139 rows, including 14 with `base_inferred`. The 42 rows in
`truth-extra.jsonl` come from the dumps and carry their separate provenance.

The two fixture edge streams and head stderr under `capture/` retain the
M-3 bytes inside gzip transport. The base stderr was empty. The capture
used `4f9e3a3eb8d2` and `1f2ad1cc49e2` with byte bounds on both arms and
`PRISM_RD_CENSUS_TRACE=1` on the head. The tests assemble synthetic S1-shaped
receipts over these bytes and bind each edited trace anew. The test does
not launch either binary or establish historical corpus counter parity.

`python3 controls.py OUTPUT_DIR` writes five trace variants. Use a new
directory. Each variant targets a different join or attribution failure.
The relocated record uses an unchanged endpoint on the same destination
line from the fixture's `prefix_write.js`.
