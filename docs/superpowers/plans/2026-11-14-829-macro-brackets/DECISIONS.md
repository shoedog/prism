# #829 owner decisions (2026-11-14)

Design phase (DESIGN.md §9):
- OD-1 (argument split, commit B): in scope. OD-2 (top-level `;`): deferred to #831. OD-3 (external block-macro list): include.
- Tests first; serial enumeration chains (A: terra→Sonnet→sol→Opus, B: Sonnet→terra→Opus→sol); enumerators state required behaviour, no fixes.
- Boundary items (D2 external macros, #813 include resolution): placed per item later (in this PR, #813, split, or pinned until #813).

Test form (TEST-FORM.md, Fable consult; all defaults accepted):
- OD-1/2: stop hand enumeration; adopt the expansion-arm differential (clang `-dump-tokens` expansion run through the same prism binary) as row oracle and corpus gate, as an authoring/gate script whose results are frozen as test data (Rust tests need no clang).
- OD-3: STRICT truth gate — no dropped pair may be CONTRADICTED by the oracle; no per-case waivers. If unconditional-wrapper drops matter for recall, a separate rule-D slice (see through unconditional nested blocks in plain code) recovers them automatically.
- OD-4: recall is a reported guide (ledger); control rows stop being assertions except where they guard test validity.
- OD-5: ~80 tentative-design tests (census facts, name list, digest) `#[ignore = "tentative design"]`, not deleted.
- OD-6: BOUNDARY computed (verdict flips with declared config, or clang opens an in-repo file the census did not read); A-ext disclosed (listed-name exceptions by the design), A-inputs to #813 as pinned holes (assert today's drop), A-config/dialect/encoding disclosed.
- OD-7: redesign aims at a positive certificate, chosen after L4 design scoring.
- OD-8: generator bound 3 fragments, 2 macros, one offline run of a few hours at most.
- OD-9: oracle-confirmed "kept" rows (b6, b8, a3–a6, w7, g2s07) become guide rows; non-compiling rows (b7, b9, mf3) marked "no truth basis".
- OD-10: baseline gaps from the same mismatch are out of scope, disclosed.
- #831: re-check its premise (Fable's oracle found no U/D drop in its probes) and amend the issue.
