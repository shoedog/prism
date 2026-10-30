# Tier-A run — prism-mf5cand (2026-10-30)

- corpus: `prism-mf5cand` @ `db066ef3762e`
- prism: `3b31a5036b85` · oracle: rust-analyzer 1.94.0 (4a4ef493 2026-03-02) · seed: 42 · harness: `3b31a5036b85e2f5ba47059f31301f9f5c43dfff`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 8.945, 'm1_oracle_inventory': 21.281, 'm2': 57.237, 'pinned': 8.894, 'matrix': 7.696, 'm3': 0.018, 'total': 179.93}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}
- oracle retries: 40 · restarts: 1

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.34–1.00] | 0.18 [0.05–0.48] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.25 [0.07–0.59] | 2/0/0 | 9 | 0 |
| C-name | 0.33 [0.14–0.61] | 0.80 [0.38–0.96] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.36 [0.15–0.65] | 0.80 [0.38–0.96] | 4/0/0 | 9 | 0 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| U-free | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 2/0/0 | 0 | 0 |
| U-method | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 6/0/0 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.34–1.00] | 0.18 [0.05–0.48] | 2/0/9 | 0 | 0 | 0 |
| C-name | 1.00 [0.51–1.00] | 0.80 [0.38–0.96] | 4/0/1 | 8 | 0 | 8 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| U-free | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 6/0/0 | 0 | 0 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.75 [0.30–0.95] | 0.50 [0.19–0.81] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 3/0/0 | 4 | 0 |
| C-name | 0.82 [0.52–0.95] | 0.82 [0.52–0.95] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 9/0/0 | 4 | 0 |
| Q-scoped | 0.78 [0.45–0.94] | 0.70 [0.40–0.89] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 7/0/0 | 5 | 0 |
| U-free | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 1/0/0 | 1 | 0 |
| U-method | 0.94 [0.73–0.99] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 16/0/0 | 1 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.34–1.00] | 0.33 [0.10–0.70] | 2/0/4 | 2 | 1 | 1 |
| C-name | 1.00 [0.57–1.00] | 0.45 [0.21–0.72] | 5/0/6 | 6 | 4 | 2 |
| Q-scoped | 0.78 [0.45–0.94] | 0.70 [0.40–0.89] | 7/2/3 | 0 | 0 | 0 |
| U-free | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 1/1/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 16/0/0 | 2 | 1 | 1 |

## Oracle and SUT failures (excluded from accuracy, retained for validity)

```json
{}
```

## Incoming sites excluded by static definition binding

```json
[]
```

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 0,
 "matched": 13197,
 "prism_extra": 0,
 "prism_missing": 86,
 "shared_oracle_selection_lines": 0,
 "shared_sut_start_lines": 0,
 "snapshot_prism_missing": 86
}
```

## M3 spot-check

```json
{
 "cap": 10,
 "checked": [
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/anon_enable_tests.rs:133",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/anon_enable_tests.rs:161",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/anon_enable_tests.rs:194",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/anon_eval_tests.rs:9",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/anon_eval_tests.rs:206",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "src/call_graph_anon_authority_tests.rs:257",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "tests/integration/js_alias_follow_test.rs:414",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:tests/name_resolution/anchor_glob_precision_test.rs:39",
   "site": "tests/integration/js_alias_follow_test.rs:415",
   "verdict": "confirmed_fp"
  }
 ],
 "counts": {
  "alias_site": 0,
  "ambiguous": 6,
  "confirmed_fp": 2,
  "confirmed_tp": 0
 }
}
```

## Capability matrix

```json
[
 {
  "capability": "chain_in_repo_exact",
  "expected": [
   [
    "main.rs",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    12
   ]
  ],
  "got_kinds": {
   "main.rs:12": "return_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/chain_in_repo_exact",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "closure_call",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/closure_call",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "common_name_collision",
  "expected": [
   [
    "main.rs",
    5
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    5
   ]
  ],
  "got_kinds": {
   "main.rs:5": "stem_single"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/common_name_collision",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "crate_glob_caller",
  "expected": [
   [
    "main.rs",
    9
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    9
   ]
  ],
  "got_kinds": {
   "main.rs:9": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/crate_glob_caller",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "cross_module_no_collision",
  "expected": [
   [
    "main.rs",
    5
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    5
   ]
  ],
  "got_kinds": {
   "main.rs:5": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/cross_module_no_collision",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "dfg_reaching_alias_conservative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.rs:3:q.x",
    "to": "main.rs:4:q.x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:1:q",
    "kill_line": null,
    "to": "main.rs:2:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:1:q",
    "kill_line": 2,
    "to": "main.rs:4:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:p",
    "kill_line": null,
    "to": "main.rs:3:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:2:q",
    "kill_line": null,
    "to": "main.rs:1:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.rs:2:q",
    "kill_line": null,
    "to": "main.rs:4:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.rs:3:q.x",
    "kill_line": null,
    "to": "main.rs:4:q.x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_alias_conservative",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_capture_timing",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:2:x",
    "to": "main.rs:3:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:3:thunk",
    "kill_line": null,
    "to": "main.rs:5:thunk"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:3:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_capture_timing",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_branch_arm_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "to": "main.rs:6:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:1:c",
    "kill_line": null,
    "to": "main.rs:3:c"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:6:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_cfg_branch_arm_negative",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_killed_def",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 3,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "exact",
    "from": "main.rs:3:x",
    "to": "main.rs:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 3,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:3:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:3:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_killed_def",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.rs:5:x",
    "to": "main.rs:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:5:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:5:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_loop_carried",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.rs:5:x",
    "to": "main.rs:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:5:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.rs:5:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_loop_carried_negative",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_rust_let_shadow",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 3,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "exact",
    "from": "main.rs:3:x",
    "to": "main.rs:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 3,
    "to": "main.rs:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:3:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:3:x",
    "kill_line": null,
    "to": "main.rs:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_rust_let_shadow",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner",
  "expected": [
   {
    "from": "main.rs:2:x",
    "present": false,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "exact",
    "from": "main.rs:2:x",
    "to": "main.rs:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:4:x",
    "kill_line": 6,
    "to": "main.rs:7:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_shadowed_inner",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner_negative",
  "expected": [
   {
    "from": "main.rs:2:x",
    "present": false,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 7,
    "to": "main.rs:8:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:2:x",
    "kill_line": null,
    "to": "main.rs:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:2:x",
    "kill_line": 7,
    "to": "main.rs:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:4:x",
    "kill_line": null,
    "to": "main.rs:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:4:x",
    "kill_line": 6,
    "to": "main.rs:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.rs:4:x",
    "kill_line": 6,
    "to": "main.rs:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.rs:7:x",
    "kill_line": null,
    "to": "main.rs:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.rs:7:x",
    "kill_line": null,
    "to": "main.rs:8:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/dfg_reaching_shadowed_inner_negative",
  "language": "rust",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "extension_trait_method",
  "expected": [
   [
    "main.rs",
    10
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    10
   ]
  ],
  "got_kinds": {
   "main.rs:10": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/extension_trait_method",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "external_chain_unchanged",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/external_chain_unchanged",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "field_receiver_method",
  "expected": [
   [
    "main.rs",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    12
   ]
  ],
  "got_kinds": {
   "main.rs:12": "field_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/field_receiver_method",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "field_typed_recovery",
  "expected": [
   [
    "main.rs",
    13
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    13
   ]
  ],
  "got_kinds": {
   "main.rs:13": "field_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/field_typed_recovery",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "free_fn_cross_file_use",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "free_single"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/free_fn_cross_file_use",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "free_fn_same_file",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/free_fn_same_file",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "inherent_method_same_file",
  "expected": [
   [
    "main.rs",
    8
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    8
   ]
  ],
  "got_kinds": {
   "main.rs:8": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/inherent_method_same_file",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "inrepo_then_external_unchanged",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/inrepo_then_external_unchanged",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "macro_arg_call",
  "expected": [
   [
    "main.rs",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    6
   ]
  ],
  "got_kinds": {
   "main.rs:6": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/macro_arg_call",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "macro_arg_ctor_guard",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/macro_arg_ctor_guard",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "macro_arg_nested",
  "expected": [
   [
    "main.rs",
    7
   ],
   [
    "main.rs",
    8
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    7
   ],
   [
    "main.rs",
    8
   ]
  ],
  "got_kinds": {
   "main.rs:7": "local_def",
   "main.rs:8": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/macro_arg_nested",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "macro_arg_nontransparent_guard",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/macro_arg_nontransparent_guard",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "macro_arg_qualified",
  "expected": [
   [
    "main.rs",
    2
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    2
   ]
  ],
  "got_kinds": {
   "main.rs:2": "stem_single"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/macro_arg_qualified",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "method_cross_file_type_ne_stem",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/method_cross_file_type_ne_stem",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "mod_qualified_free_fn",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "stem_single"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/mod_qualified_free_fn",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "module_edges_basic",
  "expected": "util.rs",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "util.rs",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/module_edges_basic",
  "language": "rust",
  "outcome": "ok",
  "probe": "module_deps"
 },
 {
  "capability": "nested_test_module_glob_gap",
  "expected": [
   [
    "main.rs",
    11
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    11
   ]
  ],
  "got_kinds": {
   "main.rs:11": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/nested_test_module_glob_gap",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "p6_typed_param_recovery",
  "expected": [
   [
    "m.rs",
    2
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "m.rs",
    2
   ]
  ],
  "got_kinds": {
   "m.rs:2": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/p6_typed_param_recovery",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "r6_multi_owner_drop",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/r6_multi_owner_drop",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "r6_single_owner_demote",
  "expected": [
   [
    "m.rs",
    3
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "m.rs",
    3
   ]
  ],
  "got_kinds": {
   "m.rs:3": "r6_single_owner"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/r6_single_owner_demote",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "receiver_method_cross_file_stem_eq",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/receiver_method_cross_file_stem_eq",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "return_typed_recovery",
  "expected": [
   [
    "main.rs",
    13
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    13
   ]
  ],
  "got_kinds": {
   "main.rs:13": "return_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/return_typed_recovery",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "super_super_glob_caller",
  "expected": [
   [
    "main.rs",
    10
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    10
   ]
  ],
  "got_kinds": {
   "main.rs:10": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/super_super_glob_caller",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "trait_dyn_dispatch",
  "expected": [
   [
    "main.rs",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    12
   ]
  ],
  "got_kinds": {
   "main.rs:12": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/trait_dyn_dispatch",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "trait_static_dispatch",
  "expected": [
   [
    "main.rs",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    12
   ]
  ],
  "got_kinds": {
   "main.rs:12": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/trait_static_dispatch",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "type_method_qualified",
  "expected": [
   [
    "main.rs",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.rs",
    4
   ]
  ],
  "got_kinds": {
   "main.rs:4": "qualified_owner"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/rust/type_method_qualified",
  "language": "rust",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "build_suffix_partition",
  "expected": [
   [
    "use_linux.go",
    3
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "use_linux.go",
    3
   ]
  ],
  "got_kinds": {
   "use_linux.go:3": "same_package"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/build_suffix_partition",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "closure",
  "expected": [
   [
    "main.go",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    6
   ]
  ],
  "got_kinds": {
   "main.go:6": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/closure",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "common_name_collision",
  "expected": [
   [
    "main.go",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    6
   ]
  ],
  "got_kinds": {
   "main.go:6": "import_qualified"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/common_name_collision",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "cross_pkg_qualified",
  "expected": [
   [
    "main.go",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    6
   ]
  ],
  "got_kinds": {
   "main.go:6": "import_qualified"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/cross_pkg_qualified",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "dfg_reaching_alias_conservative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:4:q.x",
    "to": "main.go:5:q.x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:2:q",
    "kill_line": null,
    "to": "main.go:3:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:2:q",
    "kill_line": null,
    "to": "main.go:5:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:p",
    "kill_line": null,
    "to": "main.go:4:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:q",
    "kill_line": null,
    "to": "main.go:2:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:3:q",
    "kill_line": null,
    "to": "main.go:5:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:4:q.x",
    "kill_line": null,
    "to": "main.go:5:q.x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_alias_conservative",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_alias_unstable",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:4:q.x",
    "to": "main.go:5:q.x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:2:q",
    "kill_line": null,
    "to": "main.go:3:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:2:q",
    "kill_line": null,
    "to": "main.go:5:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:2:q",
    "kill_line": null,
    "to": "main.go:6:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:p",
    "kill_line": null,
    "to": "main.go:4:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:p",
    "kill_line": null,
    "to": "main.go:6:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:q",
    "kill_line": null,
    "to": "main.go:2:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:3:q",
    "kill_line": null,
    "to": "main.go:5:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:3:q",
    "kill_line": null,
    "to": "main.go:6:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "main.go:4:q.x",
    "kill_line": null,
    "to": "main.go:5:q.x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:p",
    "kill_line": null,
    "to": "main.go:3:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:p",
    "kill_line": null,
    "to": "main.go:4:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:q",
    "kill_line": null,
    "to": "main.go:2:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:q",
    "kill_line": null,
    "to": "main.go:3:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:q",
    "kill_line": null,
    "to": "main.go:5:q"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_alias_unstable",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_call_nameonly_named_param",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "call_nameonly",
    "from": "main.go:8:x",
    "to": "main.go:3:p"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:p",
    "kill_line": null,
    "to": "main.go:3:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:6:c",
    "kill_line": null,
    "to": "main.go:8:c"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:7:x",
    "kill_line": null,
    "to": "main.go:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "call_nameonly",
    "from": "main.go:8:x",
    "kill_line": null,
    "to": "main.go:3:p"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_call_nameonly_named_param",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_capture_timing",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:x",
    "to": "main.go:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_capture_timing",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_go_defer_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "to": "main.go:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_cfg_go_defer_negative",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_defer_argument_now",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.go:3:x",
    "to": "main.go:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_defer_argument_now",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_defer_argument_now_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:x",
    "to": "main.go:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_defer_argument_now_negative",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_go_short_var_if",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.go:4:v",
    "to": "main.go:5:v"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:v",
    "kill_line": 4,
    "to": "main.go:5:v"
   },
   {
    "confidence": "exact",
    "from": "main.go:3:v",
    "to": "main.go:7:v"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:4:v",
    "kill_line": 6,
    "to": "main.go:7:v"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:v",
    "kill_line": null,
    "to": "main.go:4:v"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:v",
    "kill_line": 4,
    "to": "main.go:5:v"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:v",
    "kill_line": null,
    "to": "main.go:7:v"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:4:v",
    "kill_line": null,
    "to": "main.go:3:v"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:4:v",
    "kill_line": null,
    "to": "main.go:5:v"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:4:v",
    "kill_line": 6,
    "to": "main.go:7:v"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_go_short_var_if",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_interproc_exact",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.go:5:x",
    "to": "main.go:2:p"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:2:p",
    "kill_line": null,
    "to": "main.go:2:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:4:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:2:p"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_interproc_exact",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_interproc_nameonly",
  "expected": [
   {
    "from": "main.go:5:x",
    "present": false,
    "to": "main.go:2:p"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:4:x",
    "kill_line": null,
    "to": "main.go:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_interproc_nameonly",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_killed_def",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:x",
    "kill_line": 4,
    "to": "main.go:5:x"
   },
   {
    "confidence": "exact",
    "from": "main.go:4:x",
    "to": "main.go:5:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:x",
    "kill_line": 4,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:4:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:4:x",
    "kill_line": null,
    "to": "main.go:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_killed_def",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.go:6:x",
    "to": "main.go:5:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:6:x",
    "kill_line": null,
    "to": "main.go:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_loop_carried",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.go:6:x",
    "to": "main.go:5:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:6:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.go:6:x",
    "kill_line": null,
    "to": "main.go:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_loop_carried_negative",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner",
  "expected": [
   {
    "from": "main.go:3:x",
    "present": false,
    "to": "main.go:6:x"
   },
   {
    "confidence": "exact",
    "from": "main.go:3:x",
    "to": "main.go:8:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:5:x",
    "kill_line": 7,
    "to": "main.go:8:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_shadowed_inner",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner_negative",
  "expected": [
   {
    "from": "main.go:3:x",
    "present": false,
    "to": "main.go:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:x",
    "kill_line": 8,
    "to": "main.go:9:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:3:x",
    "kill_line": null,
    "to": "main.go:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:3:x",
    "kill_line": 8,
    "to": "main.go:9:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:5:x",
    "kill_line": null,
    "to": "main.go:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:5:x",
    "kill_line": 7,
    "to": "main.go:8:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.go:5:x",
    "kill_line": 7,
    "to": "main.go:9:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.go:8:x",
    "kill_line": null,
    "to": "main.go:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.go:8:x",
    "kill_line": null,
    "to": "main.go:9:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/dfg_reaching_shadowed_inner_negative",
  "language": "go",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "embedded_interface_struct",
  "expected": [
   [
    "main.go",
    16
   ]
  ],
  "expected_resolution_kind": "interface_dispatch",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    16
   ]
  ],
  "got_kinds": {
   "main.go:16": "interface_dispatch"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/embedded_interface_struct",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "embedded_method",
  "expected": [
   [
    "main.go",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    12
   ]
  ],
  "got_kinds": {
   "main.go:12": "embedded_promotion"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/embedded_method",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "field_typed_depth_guard",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/field_typed_depth_guard",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "field_typed_recovery",
  "expected": [
   [
    "main.go",
    12
   ]
  ],
  "expected_resolution_kind": "field_typed",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    12
   ]
  ],
  "got_kinds": {
   "main.go:12": "field_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/field_typed_recovery",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "func_value_fanout",
  "expected": [
   [
    "main.go",
    12
   ]
  ],
  "expected_resolution_kind": "callback_registration",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    12
   ]
  ],
  "got_kinds": {
   "main.go:12": "callback_registration"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/func_value_fanout",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "func_value_invocation",
  "expected": [
   [
    "main.go",
    14
   ]
  ],
  "expected_resolution_kind": "func_value_field",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    10
   ],
   [
    "main.go",
    14
   ]
  ],
  "got_kinds": {
   "main.go:10": "callback_registration",
   "main.go:14": "func_value_field"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/func_value_invocation",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "func_value_registration",
  "expected": [
   [
    "main.go",
    10
   ]
  ],
  "expected_resolution_kind": "callback_registration",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    10
   ]
  ],
  "got_kinds": {
   "main.go:10": "callback_registration"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/func_value_registration",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "gobuild_expr_partition",
  "expected": [
   [
    "use_fast.go",
    5
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "use_fast.go",
    5
   ]
  ],
  "got_kinds": {
   "use_fast.go:5": "same_package"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/gobuild_expr_partition",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "import_package_path",
  "expected": [
   [
    "main.go",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    6
   ]
  ],
  "got_kinds": {
   "main.go:6": "import_qualified"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/import_package_path",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "interface_dispatch",
  "expected": [
   [
    "main.go",
    12
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    12
   ]
  ],
  "got_kinds": {
   "main.go:12": "interface_dispatch"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/interface_dispatch",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "interface_dispatch_assert",
  "expected": [
   [
    "main.go",
    14
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    14
   ]
  ],
  "got_kinds": {
   "main.go:14": "interface_dispatch"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/interface_dispatch_assert",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "interface_dispatch_var",
  "expected": [
   [
    "main.go",
    15
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    15
   ]
  ],
  "got_kinds": {
   "main.go:15": "interface_dispatch"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/interface_dispatch_var",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "package_var_receiver",
  "expected": [
   [
    "main.go",
    14
   ]
  ],
  "expected_resolution_kind": "interface_dispatch",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    14
   ]
  ],
  "got_kinds": {
   "main.go:14": "interface_dispatch"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/package_var_receiver",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "pkg_clause_partition",
  "expected": [
   [
    "global_test.go",
    3
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "global_test.go",
    3
   ]
  ],
  "got_kinds": {
   "global_test.go:3": "same_package"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/pkg_clause_partition",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "return_typed_multi_return_guard",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/return_typed_multi_return_guard",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "return_typed_recovery",
  "expected": [
   [
    "main.go",
    13
   ]
  ],
  "expected_resolution_kind": "return_typed",
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    13
   ]
  ],
  "got_kinds": {
   "main.go:13": "return_typed"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/return_typed_recovery",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "same_pkg_free_fn",
  "expected": [
   [
    "main.go",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    6
   ]
  ],
  "got_kinds": {
   "main.go:6": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/same_pkg_free_fn",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "struct_method_cross_file",
  "expected": [
   [
    "main.go",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    4
   ]
  ],
  "got_kinds": {
   "main.go:4": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/struct_method_cross_file",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "struct_method_same_file",
  "expected": [
   [
    "main.go",
    8
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "main.go",
    8
   ]
  ],
  "got_kinds": {
   "main.go:8": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/go/struct_method_same_file",
  "language": "go",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "class_method_same_file",
  "expected": [
   [
    "app.py",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    6
   ]
  ],
  "got_kinds": {
   "app.py:6": "r6_single_owner"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/class_method_same_file",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "closure",
  "expected": [
   [
    "app.py",
    6
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    6
   ]
  ],
  "got_kinds": {
   "app.py:6": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/closure",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "common_name_collision",
  "expected": [
   [
    "app.py",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    4
   ]
  ],
  "got_kinds": {
   "app.py:4": "import_qualified"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/common_name_collision",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "decorator_wrapped",
  "expected": [
   [
    "app.py",
    8
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    8
   ]
  ],
  "got_kinds": {
   "app.py:8": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/decorator_wrapped",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "dfg_reaching_alias_conservative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:3:q.x",
    "to": "a.py:4:q.x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:q",
    "kill_line": null,
    "to": "a.py:2:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:q",
    "kill_line": null,
    "to": "a.py:4:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:p",
    "kill_line": null,
    "to": "a.py:3:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:q",
    "kill_line": null,
    "to": "a.py:1:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:2:q",
    "kill_line": null,
    "to": "a.py:4:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:3:q.x",
    "kill_line": null,
    "to": "a.py:4:q.x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_alias_conservative",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_alias_unstable",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:3:q.x",
    "to": "a.py:4:q.x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:q",
    "kill_line": null,
    "to": "a.py:2:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:q",
    "kill_line": null,
    "to": "a.py:4:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:q",
    "kill_line": null,
    "to": "a.py:5:q"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:p",
    "kill_line": null,
    "to": "a.py:3:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:p",
    "kill_line": null,
    "to": "a.py:5:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:q",
    "kill_line": null,
    "to": "a.py:1:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:2:q",
    "kill_line": null,
    "to": "a.py:4:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:2:q",
    "kill_line": null,
    "to": "a.py:5:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "alias_unstable",
    "from": "a.py:3:q.x",
    "kill_line": null,
    "to": "a.py:4:q.x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:p",
    "kill_line": null,
    "to": "a.py:2:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:p",
    "kill_line": null,
    "to": "a.py:3:p"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:q",
    "kill_line": null,
    "to": "a.py:1:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:q",
    "kill_line": null,
    "to": "a.py:2:q"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:q",
    "kill_line": null,
    "to": "a.py:4:q"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_alias_unstable",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_capture_timing",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:x",
    "to": "a.py:3:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:3:thunk",
    "kill_line": null,
    "to": "a.py:5:thunk"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:3:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_capture_timing",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_branch_arm_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "to": "a.py:6:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:1:c",
    "kill_line": null,
    "to": "a.py:3:c"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:6:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:6:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_cfg_branch_arm_negative",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_gap",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "to": "a.py:5:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_cfg_gap",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_try_header_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "to": "a.py:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:4:x",
    "kill_line": null,
    "to": "a.py:7:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_cfg_try_header_negative",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_interproc_exact",
  "expected": [
   {
    "confidence": "exact",
    "from": "a.py:6:x",
    "to": "a.py:1:p"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:1:p",
    "kill_line": null,
    "to": "a.py:2:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:5:x",
    "kill_line": null,
    "to": "a.py:6:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:6:x",
    "kill_line": null,
    "to": "a.py:1:p"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_interproc_exact",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_interproc_nameonly",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "call_nameonly",
    "from": "a.py:7:x",
    "to": "a.py:2:p"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:2:p",
    "kill_line": null,
    "to": "a.py:3:p"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:6:x",
    "kill_line": null,
    "to": "a.py:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "call_nameonly",
    "from": "a.py:7:x",
    "kill_line": null,
    "to": "a.py:2:p"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_interproc_nameonly",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_killed_def",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "a.py:2:x",
    "kill_line": 3,
    "to": "a.py:4:x"
   },
   {
    "confidence": "exact",
    "from": "a.py:3:x",
    "to": "a.py:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "a.py:2:x",
    "kill_line": 3,
    "to": "a.py:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:3:x",
    "kill_line": null,
    "to": "a.py:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:3:x",
    "kill_line": null,
    "to": "a.py:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_killed_def",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried",
  "expected": {
   "edges": [
    {
     "confidence": "exact",
     "from": "a.py:5:x",
     "to": "a.py:4:x"
    }
   ],
   "stats": {
    "dfg_label_loop_carried_min": 1
   }
  },
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": {
   "edges": [
    {
     "confidence": "exact",
     "doubt": null,
     "from": "a.py:2:x",
     "kill_line": null,
     "to": "a.py:4:x"
    },
    {
     "confidence": "exact",
     "doubt": null,
     "from": "a.py:2:x",
     "kill_line": null,
     "to": "a.py:5:x"
    },
    {
     "confidence": "nameonly",
     "doubt": "cfg_incomplete",
     "from": "a.py:5:x",
     "kill_line": null,
     "to": "a.py:2:x"
    },
    {
     "confidence": "exact",
     "doubt": null,
     "from": "a.py:5:x",
     "kill_line": null,
     "to": "a.py:4:x"
    }
   ],
   "stats": {
    "dfg_label_exact": 3,
    "dfg_label_loop_carried": 1,
    "dfg_label_nameonly_alias_unstable": 0,
    "dfg_label_nameonly_call": 0,
    "dfg_label_nameonly_cfg_incomplete": 1,
    "dfg_label_nameonly_killed": 0,
    "dfg_label_nameonly_sameline": 0,
    "dfg_rd_functions_over_cap": 0,
    "dfg_rd_functions_without_cfg": 0
   }
  },
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_loop_carried",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:5:x",
    "to": "a.py:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:2:x",
    "kill_line": null,
    "to": "a.py:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:x",
    "kill_line": null,
    "to": "a.py:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:5:x",
    "kill_line": null,
    "to": "a.py:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_loop_carried_negative",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_nonlocal_global",
  "expected": [
   {
    "from": "a.py:1:x",
    "present": false,
    "to": "a.py:8:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "a.py:5:x",
    "kill_line": null,
    "to": "a.py:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_nonlocal_global",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_same_line",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:2:a",
    "to": "a.py:3:a"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:2:a",
    "kill_line": null,
    "to": "a.py:3:a"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/dfg_reaching_same_line",
  "language": "python",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "from_import_alias",
  "expected": [
   [
    "app.py",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    4
   ]
  ],
  "got_kinds": {
   "app.py:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/from_import_alias",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "import_module_call",
  "expected": [
   [
    "app.py",
    4
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    4
   ]
  ],
  "got_kinds": {
   "app.py:4": "import_qualified"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/import_module_call",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "inherited_direct_base_typed",
  "expected": [
   [
    "app.py",
    13
   ]
  ],
  "expected_resolution_kind": "typed_param",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    13
   ]
  ],
  "got_kinds": {
   "app.py:13": "typed_param"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/inherited_direct_base_typed",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "inherited_override",
  "expected": [
   [
    "app.py",
    10
   ]
  ],
  "expected_resolution_kind": "r6_multi_owner_candidate",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    10
   ]
  ],
  "got_kinds": {
   "app.py:10": "r6_multi_owner_candidate"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/inherited_override",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "instance_method_cross_file",
  "expected": [
   [
    "app.py",
    5
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    5
   ]
  ],
  "got_kinds": {
   "app.py:5": "constructor_local"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/instance_method_cross_file",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "module_edges_basic",
  "expected": "b.py",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "a.py,b.py",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/module_edges_basic",
  "language": "python",
  "outcome": "ok",
  "probe": "module_deps"
 },
 {
  "capability": "module_fn",
  "expected": [
   [
    "app.py",
    5
   ]
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    5
   ]
  ],
  "got_kinds": {
   "app.py:5": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/module_fn",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "multi_owner_candidate",
  "expected": [
   [
    "app.py",
    12
   ]
  ],
  "expected_resolution_kind": "r6_multi_owner_candidate",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    12
   ]
  ],
  "got_kinds": {
   "app.py:12": "r6_multi_owner_candidate"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/multi_owner_candidate",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "multi_owner_over_cap",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/multi_owner_over_cap",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "property_access",
  "expected": [
   [
    "app.py",
    8
   ]
  ],
  "expected_resolution_kind": "property_access",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    8
   ]
  ],
  "got_kinds": {
   "app.py:8": "property_access"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/property_access",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "property_fanout",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/property_fanout",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "property_self_access",
  "expected": [
   [
    "app.py",
    7
   ]
  ],
  "expected_resolution_kind": "property_access",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    7
   ]
  ],
  "got_kinds": {
   "app.py:7": "property_access"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/property_self_access",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "property_setter_guard",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/property_setter_guard",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "framework_entry",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "framework_entry",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/route_decorator_guard",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "framework_entry",
  "expected": [
   [
    "app.py",
    6
   ]
  ],
  "expected_resolution_kind": "framework_entry",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    6
   ]
  ],
  "got_kinds": {
   "app.py:6": "framework_entry"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/route_handler_entry",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "framework_entry",
  "expected": [
   [
    "app.py",
    6
   ]
  ],
  "expected_resolution_kind": "framework_entry",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.py",
    6
   ]
  ],
  "got_kinds": {
   "app.py:6": "framework_entry"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/route_handler_entry_fastapi",
  "language": "python",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "taint_boundary_negative",
  "expected": "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=any",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=false",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_boundary_negative",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_cross_function_positive",
  "expected": "Reached|warnings=none|sanitizers=any",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "Reached|warnings=none|sanitizers=false",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_cross_function_positive",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_descent_depth_bound",
  "expected": "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=any",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=false",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_descent_depth_bound",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_frontier_only",
  "expected": "None|warnings=none|sanitizers=any|frontier=>=1",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "None|warnings=none|sanitizers=false|frontier=2",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_frontier_only",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_reach_positive",
  "expected": "Reached|warnings=none|sanitizers=any",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "Reached|warnings=OrderingUnavailable|sanitizers=false",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_reach_positive",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_sanitized_current",
  "expected": "Sanitized|warnings=Cleansed|sanitizers=true",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "Sanitized|warnings=Cleansed|sanitizers=true",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_sanitized_current",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "taint_sanitizer_bypass",
  "expected": "Reached|warnings=Cleansed|sanitizers=true",
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": "Reached|warnings=Cleansed|sanitizers=true",
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/python/taint_sanitizer_bypass",
  "language": "python",
  "outcome": "ok",
  "probe": "taint"
 },
 {
  "capability": "commonjs_export_deferred",
  "expected": [
   [
    "app.js",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.js",
    4
   ]
  ],
  "got_kinds": {
   "app.js:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/commonjs_export_deferred",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "dfg_reaching_capture_timing",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:2:x",
    "to": "main.js:3:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:3:thunk",
    "kill_line": null,
    "to": "main.js:5:thunk"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:3:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_capture_timing",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_try_header_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "to": "main.js:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:7:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_cfg_try_header_negative",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_js_var_hoisting",
  "expected": [
   {
    "confidence": "nameonly",
    "from": "main.js:3:v",
    "to": "main.js:2:v"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:3:v",
    "kill_line": null,
    "to": "main.js:2:v"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_js_var_hoisting",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_killed_def",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.js:2:x",
    "kill_line": 3,
    "to": "main.js:4:x"
   },
   {
    "confidence": "exact",
    "from": "main.js:3:x",
    "to": "main.js:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.js:2:x",
    "kill_line": 3,
    "to": "main.js:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:3:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:3:x",
    "kill_line": null,
    "to": "main.js:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_killed_def",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.js:5:x",
    "to": "main.js:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:5:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:5:x",
    "kill_line": null,
    "to": "main.js:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_loop_carried",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:5:x",
    "to": "main.js:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:5:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:5:x",
    "kill_line": null,
    "to": "main.js:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_loop_carried_negative",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_same_line",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:2:a",
    "to": "main.js:3:a"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:2:a",
    "kill_line": null,
    "to": "main.js:3:a"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_same_line",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner",
  "expected": [
   {
    "from": "main.js:2:x",
    "present": false,
    "to": "main.js:5:x"
   },
   {
    "confidence": "exact",
    "from": "main.js:2:x",
    "to": "main.js:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:7:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_shadowed_inner",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner_negative",
  "expected": [
   {
    "from": "main.js:2:x",
    "present": false,
    "to": "main.js:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.js:2:x",
    "kill_line": 7,
    "to": "main.js:8:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:2:x",
    "kill_line": null,
    "to": "main.js:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.js:2:x",
    "kill_line": 7,
    "to": "main.js:8:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:7:x",
    "kill_line": null,
    "to": "main.js:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.js:7:x",
    "kill_line": null,
    "to": "main.js:8:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/dfg_reaching_shadowed_inner_negative",
  "language": "javascript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "named_import_alias",
  "expected": [
   [
    "app.js",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.js",
    4
   ]
  ],
  "got_kinds": {
   "app.js:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/named_import_alias",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "non_exported_function_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/non_exported_function_deferred",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "framework_entry",
  "expected": [
   [
    "app.js",
    6
   ]
  ],
  "expected_resolution_kind": "framework_entry",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.js",
    6
   ]
  ],
  "got_kinds": {
   "app.js:6": "framework_entry"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/route_handler_entry",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "framework_entry",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "framework_entry",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/route_receiver_guard",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_js_export_hop",
  "expected": [
   [
    "app.jsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.jsx",
    2
   ]
  ],
  "got_kinds": {
   "app.jsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_js_export_hop",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_js_export_hop_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_js_export_hop_refusal",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_nonrelative_hop",
  "expected": [
   [
    "app.jsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.jsx",
    2
   ]
  ],
  "got_kinds": {
   "app.jsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_nonrelative_hop",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_nonrelative_hop_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_nonrelative_hop_refusal",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_positive",
  "expected": [
   [
    "app.jsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.jsx",
    2
   ]
  ],
  "got_kinds": {
   "app.jsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_positive",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/javascript/tsconfig_paths_refusal",
  "language": "javascript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "arrow_const_export_deferred",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/arrow_const_export_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "barrel_reexport_three_hop_fails",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/barrel_reexport_three_hop_fails",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "barrel_reexport_two_hop_resolves",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/barrel_reexport_two_hop_resolves",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "catch_shadow_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/catch_shadow_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "common_name_collision",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/common_name_collision",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "default_export_function_deferred",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/default_export_function_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "default_export_named_import_mismatch",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/default_export_named_import_mismatch",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "default_import_deferred",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/default_import_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "default_import_named_export_mismatch",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/default_import_named_export_mismatch",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "dfg_reaching_capture_timing",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:2:x",
    "to": "main.ts:3:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:3:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:3:thunk",
    "kill_line": null,
    "to": "main.ts:5:thunk"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:3:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_capture_timing",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_gap",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.ts:3:x",
    "to": "main.ts:5:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:3:x",
    "kill_line": null,
    "to": "main.ts:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_cfg_gap",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_cfg_try_header_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "to": "main.ts:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:7:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_cfg_try_header_negative",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_killed_def",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.ts:2:x",
    "kill_line": 3,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "exact",
    "from": "main.ts:3:x",
    "to": "main.ts:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:3:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.ts:2:x",
    "kill_line": 3,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:3:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:3:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_killed_def",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried",
  "expected": [
   {
    "confidence": "exact",
    "from": "main.ts:5:x",
    "to": "main.ts:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:5:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:5:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_loop_carried",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_loop_carried_negative",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:5:x",
    "to": "main.ts:4:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:5:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:5:x",
    "kill_line": null,
    "to": "main.ts:4:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_loop_carried_negative",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_same_line",
  "expected": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:2:a",
    "to": "main.ts:3:a"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:2:a",
    "kill_line": null,
    "to": "main.ts:3:a"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_same_line",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner",
  "expected": [
   {
    "from": "main.ts:2:x",
    "present": false,
    "to": "main.ts:5:x"
   },
   {
    "confidence": "exact",
    "from": "main.ts:2:x",
    "to": "main.ts:7:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:7:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:5:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_shadowed_inner",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "dfg_reaching_shadowed_inner_negative",
  "expected": [
   {
    "from": "main.ts:2:x",
    "present": false,
    "to": "main.ts:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.ts:2:x",
    "kill_line": 7,
    "to": "main.ts:8:x"
   }
  ],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:2:x",
    "kill_line": null,
    "to": "main.ts:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.ts:2:x",
    "kill_line": 7,
    "to": "main.ts:8:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:5:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:7:x",
    "kill_line": null,
    "to": "main.ts:2:x"
   },
   {
    "confidence": "exact",
    "doubt": null,
    "from": "main.ts:7:x",
    "kill_line": null,
    "to": "main.ts:8:x"
   }
  ],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/dfg_reaching_shadowed_inner_negative",
  "language": "typescript",
  "outcome": "ok",
  "probe": "dfg"
 },
 {
  "capability": "duplicate_import_local",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/duplicate_import_local",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "forwardref_ident_arg_nested_samename_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [
   [
    "lib.tsx",
    7
   ]
  ],
  "got_kinds": {
   "lib.tsx:7": "local_def"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/forwardref_ident_arg_nested_samename_refused",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "forwardref_named_export",
  "expected": [
   [
    "app.tsx",
    3
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.tsx",
    3
   ]
  ],
  "got_kinds": {
   "app.tsx:3": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/forwardref_named_export",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "index_module",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/index_module",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "local_shadow_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/local_shadow_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "named_import_alias",
  "expected": [
   [
    "app.ts",
    4
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.ts",
    4
   ]
  ],
  "got_kinds": {
   "app.ts:4": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/named_import_alias",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "non_exported_function_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/non_exported_function_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "param_shadow_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/param_shadow_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "react_default_member_write_out_of_model",
  "expected": [
   [
    "app.tsx",
    3
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.tsx",
    3
   ]
  ],
  "got_kinds": {
   "app.tsx:3": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/react_default_member_write_out_of_model",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "rebound_import_local",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/rebound_import_local",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "s1b_jsx_intrinsic_tag_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "free_single",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/s1b_jsx_intrinsic_tag_refused",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "s1b_list_export_nested_decoy_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/s1b_list_export_nested_decoy_refused",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "s1b_namespace_nested_decoy_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_qualified",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/s1b_namespace_nested_decoy_refused",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "s1b_param_shadow_local_def_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "local_def",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/s1b_param_shadow_local_def_refused",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_js_export_hop",
  "expected": [
   [
    "app.tsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.tsx",
    2
   ]
  ],
  "got_kinds": {
   "app.tsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_js_export_hop",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_js_export_hop_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_js_export_hop_refusal",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_nonrelative_hop",
  "expected": [
   [
    "app.tsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.tsx",
    2
   ]
  ],
  "got_kinds": {
   "app.tsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_nonrelative_hop",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_nonrelative_hop_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_nonrelative_hop_refusal",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_positive",
  "expected": [
   [
    "app.tsx",
    2
   ]
  ],
  "expected_resolution_kind": "import_member",
  "forbid_resolution_kind": null,
  "got": [
   [
    "app.tsx",
    2
   ]
  ],
  "got_kinds": {
   "app.tsx:2": "import_member"
  },
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_positive",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "tsconfig_paths_refusal",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": null,
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/tsconfig_paths_refusal",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "type_only_import_deferred",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/type_only_import_deferred",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 },
 {
  "capability": "wrong_directory_same_stem",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "import_member",
  "got": [],
  "got_kinds": {},
  "id": "/Users/wesleyjinks/code/slicing-tiera/eval/fixtures/typescript/wrong_directory_same_stem",
  "language": "typescript",
  "outcome": "ok",
  "probe": "callers"
 }
]
```

## Pending triage

```json
[
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index.rs:163",
  "site_fingerprint": "6bd1c71f613ebf85"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers.rs:149",
  "site_fingerprint": "7218c721523c70c5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:445",
  "site_fingerprint": "0e9e3734a51594a5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:497",
  "site_fingerprint": "e3a3d44f81093447"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:501",
  "site_fingerprint": "66694f689bbc0881"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:584",
  "site_fingerprint": "a3f9e9824ecc81e0"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:648",
  "site_fingerprint": "022f9d73199c7e2b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers/tests.rs:650",
  "site_fingerprint": "7bde8bcb66475508"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/compact/views.rs:99",
  "site": "src/data_flow/tests/representatives.rs:227",
  "site_fingerprint": "d9680b179575c9fc"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/go_type_alias/canon.rs:439",
  "site": "src/go_type_alias/canon.rs:445",
  "site_fingerprint": "ed44a316f632bced"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/anon_enable_tests.rs:133",
  "site_fingerprint": "a2def8272ce07ad4",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/anon_enable_tests.rs:161",
  "site_fingerprint": "8c8487c869ef8e1e",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/anon_enable_tests.rs:194",
  "site_fingerprint": "cc30b4cb85d04548",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/anon_eval_tests.rs:9",
  "site_fingerprint": "5f58940b596344b1",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/anon_eval_tests.rs:206",
  "site_fingerprint": "147cab9d0ce55253",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "src/call_graph_anon_authority_tests.rs:257",
  "site_fingerprint": "6081e44eaf9adecd",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "tests/integration/js_alias_follow_test.rs:414",
  "site_fingerprint": "a88affe13363cdb1",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "tests/name_resolution/anchor_glob_precision_test.rs:39",
  "site": "tests/integration/js_alias_follow_test.rs:415",
  "site_fingerprint": "fa9574eb3e6600a8",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/ast/member_reference_index/identifiers.rs:114",
  "site": "src/ast/member_reference_index/identifiers.rs:123",
  "site_fingerprint": "b01734bb0ad3d39d"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/cpg/callback_identity_tests.rs:360",
  "site": "src/cpg/callback_identity_tests.rs:366",
  "site_fingerprint": "49a02a7f7eb082c9"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/cpg/callback_identity_tests.rs:360",
  "site": "src/cpg/callback_identity_tests.rs:369",
  "site_fingerprint": "8d96aa0aafedf4a2"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/cpg/callback_identity_tests.rs:360",
  "site": "src/cpg/callback_identity_tests.rs:364",
  "site_fingerprint": "440f84b06726379d"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/data_flow/compact/views.rs:99",
  "site": "src/data_flow/compact/views.rs:102",
  "site_fingerprint": "e884b6b5f84b5223",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/data_flow/compact/views.rs:99",
  "site": "src/data_flow/compact/views.rs:105",
  "site_fingerprint": "66b87ee779c45a90"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/data_flow/compact/views.rs:99",
  "site": "src/data_flow/compact/views.rs:106",
  "site_fingerprint": "e741170a92231e96"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/framework_entries/mod.rs:96",
  "site": "src/framework_entries/mod.rs:198",
  "site_fingerprint": "962d17b47a4d0500",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/framework_entries/mod.rs:96",
  "site": "src/framework_entries/mod.rs:219",
  "site_fingerprint": "5e763da3b341d4a6"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/framework_entries/mod.rs:96",
  "site": "src/framework_entries/mod.rs:220",
  "site_fingerprint": "9e951bfdfdf087b3"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/go_type_alias/canon.rs:439",
  "site": "src/go_type_alias/canon.rs:443",
  "site_fingerprint": "1b570ade09557164",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/main.rs:1034",
  "site": "src/main.rs:1036",
  "site_fingerprint": "2f1a9bab070a6e7d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/mcp/mem_sample.rs:234",
  "site": "src/mcp/mem_sample.rs:237",
  "site_fingerprint": "4257ee085691c577"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/mcp/mem_sample.rs:234",
  "site": "src/mcp/mem_sample.rs:242",
  "site_fingerprint": "c429940c180c0c58"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/resolution.rs:1928",
  "site": "src/resolution.rs:1940",
  "site_fingerprint": "89670125acc0dc02",
  "tier": "candidate"
 }
]
```

## Pinned probes

```json
[
 {
  "exact_supplementary": {
   "fn": 0,
   "fp": 0,
   "precision": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "recall": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "tp": 5
  },
  "expected": "known_fail",
  "id": "target-c-method",
  "known_fail": false,
  "oracle_only": [],
  "oracle_sites": [
   [
    "src/algorithms/taint.rs",
    1768,
    1768
   ],
   [
    "src/algorithms/taint.rs",
    4445,
    4445
   ],
   [
    "src/algorithms/taint.rs",
    4455,
    4455
   ],
   [
    "src/algorithms/taint.rs",
    4463,
    4463
   ],
   [
    "src/algorithms/taint.rs",
    4475,
    4475
   ]
  ],
  "outcome": "flip_candidate",
  "prism_only": [],
  "prism_sites": [
   [
    "src/algorithms/taint.rs",
    1768,
    1768
   ],
   [
    "src/algorithms/taint.rs",
    4445,
    4445
   ],
   [
    "src/algorithms/taint.rs",
    4455,
    4455
   ],
   [
    "src/algorithms/taint.rs",
    4463,
    4463
   ],
   [
    "src/algorithms/taint.rs",
    4475,
    4475
   ]
  ],
  "raw": {
   "fn": 0,
   "fp": 0,
   "precision": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "recall": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "tp": 5
  }
 },
 {
  "expected": "oracle_miss_site",
  "id": "module-deps-feature-gated",
  "oracle_miss_site_found": false,
  "oracle_only": [],
  "oracle_sites": [
   [
    "tests/navigation/module_graph_test.rs",
    93,
    93
   ],
   [
    "tests/navigation/module_graph_test.rs",
    142,
    142
   ],
   [
    "tests/navigation/module_graph_test.rs",
    152,
    152
   ],
   [
    "tests/navigation/module_graph_test.rs",
    169,
    169
   ],
   [
    "tests/navigation/module_graph_test.rs",
    219,
    219
   ],
   [
    "tests/navigation/module_graph_test.rs",
    236,
    236
   ],
   [
    "tests/navigation/module_graph_test.rs",
    253,
    253
   ],
   [
    "tests/navigation/module_graph_test.rs",
    281,
    281
   ],
   [
    "tests/navigation/module_graph_test.rs",
    307,
    307
   ],
   [
    "tests/navigation/module_graph_test.rs",
    332,
    332
   ],
   [
    "tests/navigation/module_graph_test.rs",
    353,
    353
   ],
   [
    "tests/navigation/module_graph_test.rs",
    455,
    455
   ],
   [
    "src/mcp/tools.rs",
    230,
    230
   ],
   [
    "tests/navigation/cache_test.rs",
    345,
    345
   ],
   [
    "tests/navigation/cache_test.rs",
    331,
    331
   ],
   [
    "src/main.rs",
    194,
    194
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    642,
    642
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    643,
    643
   ],
   [
    "tests/navigation/scoped_calls_test.rs",
    225,
    225
   ]
  ],
  "outcome": "missing",
  "prism_only": [],
  "prism_sites": [
   [
    "src/main.rs",
    194,
    194
   ],
   [
    "src/mcp/tools.rs",
    230,
    230
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    642,
    642
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    643,
    643
   ],
   [
    "tests/navigation/cache_test.rs",
    331,
    331
   ],
   [
    "tests/navigation/cache_test.rs",
    345,
    345
   ],
   [
    "tests/navigation/module_graph_test.rs",
    93,
    93
   ],
   [
    "tests/navigation/module_graph_test.rs",
    142,
    142
   ],
   [
    "tests/navigation/module_graph_test.rs",
    152,
    152
   ],
   [
    "tests/navigation/module_graph_test.rs",
    169,
    169
   ],
   [
    "tests/navigation/module_graph_test.rs",
    219,
    219
   ],
   [
    "tests/navigation/module_graph_test.rs",
    236,
    236
   ],
   [
    "tests/navigation/module_graph_test.rs",
    253,
    253
   ],
   [
    "tests/navigation/module_graph_test.rs",
    281,
    281
   ],
   [
    "tests/navigation/module_graph_test.rs",
    307,
    307
   ],
   [
    "tests/navigation/module_graph_test.rs",
    332,
    332
   ],
   [
    "tests/navigation/module_graph_test.rs",
    353,
    353
   ],
   [
    "tests/navigation/module_graph_test.rs",
    455,
    455
   ],
   [
    "tests/navigation/scoped_calls_test.rs",
    225,
    225
   ]
  ],
  "raw": {
   "fn": 0,
   "fp": 0,
   "precision": [
    1.0,
    0.8318207737404363,
    0.9999999999999999
   ],
   "recall": [
    1.0,
    0.8318207737404363,
    0.9999999999999999
   ],
   "tp": 19
  }
 },
 {
  "expected": "oracle_miss_site",
  "id": "load-repo-feature-gated",
  "oracle_miss_site_found": false,
  "oracle_only": [
   "examples/dfg_census.rs:359",
   "examples/parameter_site_census.rs:219",
   "examples/project_membership_census.rs:63",
   "src/js_exports/memo/pipeline_tests.rs:82",
   "tests/integration/resolution_test.rs:5299",
   "tests/integration/resolution_test.rs:5368"
  ],
  "oracle_sites": [
   [
    "tests/navigation/nodes_at_test.rs",
    17,
    17
   ],
   [
    "tests/navigation/loader_test.rs",
    32,
    32
   ],
   [
    "tests/navigation/framework_entry_test.rs",
    20,
    20
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    72,
    72
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    127,
    127
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    169,
    169
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    198,
    198
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    233,
    233
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    323,
    323
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    469,
    469
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    528,
    528
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    576,
    576
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    628,
    628
   ],
   [
    "tests/navigation/callers_test.rs",
    11,
    11
   ],
   [
    "tests/navigation/callers_test.rs",
    134,
    134
   ],
   [
    "tests/navigation/callers_test.rs",
    166,
    166
   ],
   [
    "tests/integration/rust_self_closed_rule_test.rs",
    503,
    503
   ],
   [
    "tests/integration/rust_self_closed_rule_test.rs",
    536,
    536
   ],
   [
    "tests/integration/js_paths_r3_test.rs",
    32,
    32
   ],
   [
    "src/call_graph.rs",
    7313,
    7313
   ],
   [
    "src/call_graph.rs",
    7326,
    7326
   ],
   [
    "tests/navigation/scoped_calls_test.rs",
    20,
    20
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    139,
    139
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    148,
    148
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    166,
    166
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    173,
    173
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    199,
    199
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    206,
    206
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    224,
    224
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    233,
    233
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    263,
    263
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    278,
    278
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    286,
    286
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    336,
    336
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    339,
    339
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    369,
    369
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    90,
    90
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    188,
    188
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    387,
    387
   ],
   [
    "tests/cli/nav_compat_test.rs",
    329,
    329
   ],
   [
    "src/navigation/anon_guard_tests.rs",
    19,
    19
   ],
   [
    "tests/ast/cpg_cache_test.rs",
    559,
    559
   ],
   [
    "src/executable_owner/acquisition.rs",
    127,
    127
   ],
   [
    "src/navigation/mod.rs",
    640,
    640
   ],
   [
    "src/navigation/mod.rs",
    655,
    655
   ],
   [
    "src/navigation/mod.rs",
    678,
    678
   ],
   [
    "src/navigation/mod.rs",
    722,
    722
   ],
   [
    "tests/integration/js_paths_p2_test.rs",
    28,
    28
   ],
   [
    "tests/integration/js_paths_p2_test.rs",
    818,
    818
   ],
   [
    "tests/infra/parallel_equality_test.rs",
    18,
    18
   ],
   [
    "tests/infra/parallel_equality_test.rs",
    22,
    22
   ],
   [
    "tests/navigation/loader_test.rs",
    13,
    13
   ],
   [
    "tests/navigation/loader_test.rs",
    41,
    41
   ],
   [
    "tests/navigation/loader_test.rs",
    59,
    59
   ],
   [
    "tests/navigation/loader_test.rs",
    86,
    86
   ],
   [
    "tests/navigation/loader_test.rs",
    104,
    104
   ],
   [
    "tests/navigation/loader_test.rs",
    106,
    106
   ],
   [
    "tests/navigation/loader_test.rs",
    108,
    108
   ],
   [
    "tests/navigation/loader_test.rs",
    110,
    110
   ],
   [
    "tests/navigation/loader_test.rs",
    112,
    112
   ],
   [
    "tests/navigation/loader_test.rs",
    134,
    134
   ],
   [
    "tests/navigation/loader_test.rs",
    168,
    168
   ],
   [
    "tests/navigation/loader_test.rs",
    174,
    174
   ],
   [
    "tests/navigation/loader_test.rs",
    176,
    176
   ],
   [
    "tests/navigation/loader_test.rs",
    179,
    179
   ],
   [
    "tests/navigation/loader_test.rs",
    181,
    181
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    143,
    143
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    163,
    163
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    186,
    186
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    201,
    201
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    213,
    213
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    230,
    230
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    232,
    232
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    246,
    246
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    248,
    248
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    267,
    267
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    314,
    314
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    325,
    325
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    372,
    372
   ],
   [
    "tests/reasoning/member_only_param_test.rs",
    23,
    23
   ],
   [
    "src/cpg/tests.rs",
    2542,
    2542
   ],
   [
    "src/executable_owner/integration/tests.rs",
    22,
    22
   ],
   [
    "src/executable_owner/integration/tests.rs",
    79,
    79
   ],
   [
    "tests/navigation/cache_test.rs",
    78,
    78
   ],
   [
    "tests/navigation/cache_test.rs",
    96,
    96
   ],
   [
    "tests/navigation/cache_test.rs",
    139,
    139
   ],
   [
    "tests/navigation/cache_test.rs",
    209,
    209
   ],
   [
    "tests/navigation/cache_test.rs",
    253,
    253
   ],
   [
    "tests/navigation/cache_test.rs",
    292,
    292
   ],
   [
    "tests/navigation/cache_test.rs",
    323,
    323
   ],
   [
    "tests/navigation/cache_test.rs",
    383,
    383
   ],
   [
    "tests/navigation/cache_test.rs",
    424,
    424
   ],
   [
    "tests/navigation/cache_test.rs",
    459,
    459
   ],
   [
    "tests/navigation/cache_test.rs",
    499,
    499
   ],
   [
    "tests/navigation/cache_test.rs",
    501,
    501
   ],
   [
    "tests/navigation/cache_test.rs",
    510,
    510
   ],
   [
    "tests/navigation/cache_test.rs",
    541,
    541
   ],
   [
    "tests/navigation/cache_test.rs",
    542,
    542
   ],
   [
    "tests/navigation/cache_test.rs",
    592,
    592
   ],
   [
    "tests/integration/js_paths_r2c_test.rs",
    43,
    43
   ],
   [
    "src/api/nav.rs",
    26,
    26
   ],
   [
    "src/navigation/call_edge_cache.rs",
    780,
    780
   ],
   [
    "src/build_pool/boundary_tests.rs",
    275,
    275
   ],
   [
    "tests/navigation/onboarding_test.rs",
    7,
    7
   ],
   [
    "src/mcp/freshness.rs",
    429,
    429
   ],
   [
    "tests/reasoning/taint_reaches_test.rs",
    25,
    25
   ],
   [
    "tests/cli/nav_compat_test.rs",
    819,
    819
   ],
   [
    "examples/dfg_census.rs",
    359,
    359
   ],
   [
    "tests/navigation/func_value_test.rs",
    19,
    19
   ],
   [
    "src/cpg_cache.rs",
    840,
    840
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    431,
    431
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    444,
    444
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    446,
    446
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    414,
    414
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    416,
    416
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    423,
    423
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    442,
    442
   ],
   [
    "src/navigation/inventory.rs",
    16,
    16
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    7,
    7
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    24,
    24
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    37,
    37
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    139,
    139
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    350,
    350
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    356,
    356
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    363,
    363
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    365,
    365
   ],
   [
    "examples/project_membership_census.rs",
    63,
    63
   ],
   [
    "tests/navigation/seed_test.rs",
    11,
    11
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    14,
    14
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    204,
    204
   ],
   [
    "tests/navigation/index_test.rs",
    13,
    13
   ],
   [
    "tests/navigation/index_test.rs",
    31,
    31
   ],
   [
    "tests/integration/dfg_label_parity_test.rs",
    30,
    30
   ],
   [
    "src/executable_owner/admission.rs",
    120,
    120
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    18,
    18
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    40,
    40
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    61,
    61
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    110,
    110
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    160,
    160
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    162,
    162
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    169,
    169
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    181,
    181
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    215,
    215
   ],
   [
    "tests/integration/js_alias_callers_test.rs",
    23,
    23
   ],
   [
    "tests/navigation/module_graph_test.rs",
    17,
    17
   ],
   [
    "src/mcp/session.rs",
    415,
    415
   ],
   [
    "src/mcp/session.rs",
    435,
    435
   ],
   [
    "src/mcp/session.rs",
    472,
    472
   ],
   [
    "examples/parameter_site_census.rs",
    219,
    219
   ],
   [
    "tests/navigation/ego_test.rs",
    9,
    9
   ],
   [
    "src/repo_loader.rs",
    333,
    333
   ],
   [
    "src/repo_loader.rs",
    368,
    368
   ],
   [
    "src/repo_loader.rs",
    1185,
    1185
   ],
   [
    "src/repo_loader.rs",
    1222,
    1222
   ],
   [
    "src/repo_loader.rs",
    1241,
    1241
   ],
   [
    "src/repo_loader.rs",
    1268,
    1268
   ],
   [
    "src/repo_loader.rs",
    1306,
    1306
   ],
   [
    "src/repo_loader.rs",
    1347,
    1347
   ],
   [
    "src/repo_loader.rs",
    1385,
    1385
   ],
   [
    "src/repo_loader.rs",
    1427,
    1427
   ],
   [
    "src/repo_loader.rs",
    1485,
    1485
   ],
   [
    "src/repo_loader.rs",
    1513,
    1513
   ],
   [
    "src/repo_loader.rs",
    1572,
    1572
   ],
   [
    "src/repo_loader.rs",
    1639,
    1639
   ],
   [
    "src/repo_loader.rs",
    1675,
    1675
   ],
   [
    "src/repo_loader.rs",
    1707,
    1707
   ],
   [
    "src/repo_loader.rs",
    1732,
    1732
   ],
   [
    "tests/integration/js_returned_callers_test.rs",
    21,
    21
   ],
   [
    "src/executable_owner/audit_tests.rs",
    310,
    310
   ],
   [
    "src/executable_owner/audit_tests.rs",
    322,
    322
   ],
   [
    "src/executable_owner/audit_tests.rs",
    333,
    333
   ],
   [
    "tests/navigation/callees_test.rs",
    11,
    11
   ],
   [
    "tests/integration/js_paths_r2d_test.rs",
    28,
    28
   ],
   [
    "tests/navigation/property_access_test.rs",
    19,
    19
   ],
   [
    "src/mcp/freshness_policy_tests.rs",
    394,
    394
   ],
   [
    "tests/integration/resolution_test.rs",
    5299,
    5299
   ],
   [
    "tests/integration/resolution_test.rs",
    5342,
    5342
   ],
   [
    "tests/integration/resolution_test.rs",
    5368,
    5368
   ],
   [
    "tests/integration/resolution_test.rs",
    5428,
    5428
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    46,
    46
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    60,
    60
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    75,
    75
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    102,
    102
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    119,
    119
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    122,
    122
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    146,
    146
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    159,
    159
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    181,
    181
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    185,
    185
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    205,
    205
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    233,
    233
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    253,
    253
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    268,
    268
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    301,
    301
   ],
   [
    "tests/navigation/glob_stats_scope_test.rs",
    46,
    46
   ],
   [
    "src/js_exports/memo/pipeline_tests.rs",
    59,
    59
   ],
   [
    "src/js_exports/memo/pipeline_tests.rs",
    82,
    82
   ],
   [
    "src/js_exports/memo/pipeline_tests.rs",
    115,
    115
   ],
   [
    "src/reasoning/seeds.rs",
    468,
    468
   ],
   [
    "tests/integration/js_paths_test.rs",
    58,
    58
   ],
   [
    "tests/integration/js_paths_test.rs",
    268,
    268
   ],
   [
    "tests/integration/js_paths_test.rs",
    297,
    297
   ],
   [
    "tests/integration/js_paths_test.rs",
    320,
    320
   ],
   [
    "tests/integration/js_paths_test.rs",
    336,
    336
   ],
   [
    "tests/integration/js_paths_test.rs",
    350,
    350
   ],
   [
    "tests/integration/js_paths_test.rs",
    396,
    396
   ],
   [
    "tests/integration/js_paths_test.rs",
    442,
    442
   ],
   [
    "tests/integration/js_paths_test.rs",
    465,
    465
   ],
   [
    "tests/integration/js_paths_test.rs",
    490,
    490
   ],
   [
    "tests/integration/js_alias_rule_test.rs",
    206,
    206
   ],
   [
    "tests/navigation/symbol_spans_test.rs",
    21,
    21
   ],
   [
    "tests/lang/go/receiver_origin_prereq_test.rs",
    36,
    36
   ],
   [
    "tests/integration/js_paths_common.rs",
    11,
    11
   ],
   [
    "src/cpg/callback_identity_tests.rs",
    776,
    776
   ],
   [
    "src/name_resolution/lookup_invalidation_tests.rs",
    165,
    165
   ],
   [
    "src/executable_owner/integration.rs",
    156,
    156
   ],
   [
    "tests/lang/go/test_support.rs",
    23,
    23
   ],
   [
    "src/mcp/shared/cache_base_tests.rs",
    276,
    276
   ],
   [
    "src/navigation/cache_lifetime_tests.rs",
    96,
    96
   ],
   [
    "src/navigation/cache_lifetime_tests.rs",
    124,
    124
   ],
   [
    "tests/integration/js_paths_r2b_test.rs",
    29,
    29
   ]
  ],
  "outcome": "missing",
  "prism_only": [
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/gap-driver.rs:13",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:12",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:251",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:280",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:303",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:319",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:332",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:369",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:392",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:417",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:439",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:461",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs:482",
   "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/dump_imports.rs:4",
   "docs/superpowers/plans/2026-10-06-js-param-defs/probes/byte_dump.rs:8",
   "docs/superpowers/plans/2026-10-06-js-param-defs/probes/census_b.rs:40",
   "docs/superpowers/plans/2026-10-06-js-param-defs/probes/dfg_time.rs:5",
   "docs/superpowers/plans/2026-10-06-js-param-defs/probes/return_inputs.rs:7"
  ],
  "prism_sites": [
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/p2-probes/gap-driver.rs",
    13,
    13
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    12,
    12
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    251,
    251
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    280,
    280
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    303,
    303
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    319,
    319
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    332,
    332
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    369,
    369
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    392,
    392
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    417,
    417
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    439,
    439
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    461,
    461
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/P1-tests.rs",
    482,
    482
   ],
   [
    "docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/dump_imports.rs",
    4,
    4
   ],
   [
    "docs/superpowers/plans/2026-10-06-js-param-defs/probes/byte_dump.rs",
    8,
    8
   ],
   [
    "docs/superpowers/plans/2026-10-06-js-param-defs/probes/census_b.rs",
    40,
    40
   ],
   [
    "docs/superpowers/plans/2026-10-06-js-param-defs/probes/dfg_time.rs",
    5,
    5
   ],
   [
    "docs/superpowers/plans/2026-10-06-js-param-defs/probes/return_inputs.rs",
    7,
    7
   ],
   [
    "src/api/nav.rs",
    26,
    26
   ],
   [
    "src/build_pool/boundary_tests.rs",
    275,
    275
   ],
   [
    "src/call_graph.rs",
    7313,
    7313
   ],
   [
    "src/call_graph.rs",
    7326,
    7326
   ],
   [
    "src/cpg/callback_identity_tests.rs",
    776,
    776
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    139,
    139
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    148,
    148
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    166,
    166
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    173,
    173
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    199,
    199
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    206,
    206
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    224,
    224
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    233,
    233
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    263,
    263
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    278,
    278
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    286,
    286
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    336,
    336
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    339,
    339
   ],
   [
    "src/cpg/fallback_lifetime_tests.rs",
    369,
    369
   ],
   [
    "src/cpg/tests.rs",
    2542,
    2542
   ],
   [
    "src/cpg_cache.rs",
    840,
    840
   ],
   [
    "src/executable_owner/acquisition.rs",
    127,
    127
   ],
   [
    "src/executable_owner/admission.rs",
    120,
    120
   ],
   [
    "src/executable_owner/audit_tests.rs",
    310,
    310
   ],
   [
    "src/executable_owner/audit_tests.rs",
    322,
    322
   ],
   [
    "src/executable_owner/audit_tests.rs",
    333,
    333
   ],
   [
    "src/executable_owner/integration.rs",
    156,
    156
   ],
   [
    "src/executable_owner/integration/tests.rs",
    22,
    22
   ],
   [
    "src/executable_owner/integration/tests.rs",
    79,
    79
   ],
   [
    "src/js_exports/memo/pipeline_tests.rs",
    59,
    59
   ],
   [
    "src/js_exports/memo/pipeline_tests.rs",
    115,
    115
   ],
   [
    "src/mcp/freshness.rs",
    429,
    429
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    46,
    46
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    60,
    60
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    75,
    75
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    102,
    102
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    119,
    119
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    122,
    122
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    146,
    146
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    159,
    159
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    181,
    181
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    185,
    185
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    205,
    205
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    233,
    233
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    253,
    253
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    268,
    268
   ],
   [
    "src/mcp/freshness_capture_tests.rs",
    301,
    301
   ],
   [
    "src/mcp/freshness_policy_tests.rs",
    394,
    394
   ],
   [
    "src/mcp/session.rs",
    415,
    415
   ],
   [
    "src/mcp/session.rs",
    435,
    435
   ],
   [
    "src/mcp/session.rs",
    472,
    472
   ],
   [
    "src/mcp/shared/cache_base_tests.rs",
    276,
    276
   ],
   [
    "src/name_resolution/lookup_invalidation_tests.rs",
    165,
    165
   ],
   [
    "src/navigation/anon_guard_tests.rs",
    19,
    19
   ],
   [
    "src/navigation/cache_lifetime_tests.rs",
    96,
    96
   ],
   [
    "src/navigation/cache_lifetime_tests.rs",
    124,
    124
   ],
   [
    "src/navigation/call_edge_cache.rs",
    780,
    780
   ],
   [
    "src/navigation/inventory.rs",
    16,
    16
   ],
   [
    "src/navigation/mod.rs",
    640,
    640
   ],
   [
    "src/navigation/mod.rs",
    655,
    655
   ],
   [
    "src/navigation/mod.rs",
    678,
    678
   ],
   [
    "src/navigation/mod.rs",
    722,
    722
   ],
   [
    "src/reasoning/seeds.rs",
    468,
    468
   ],
   [
    "src/repo_loader.rs",
    333,
    333
   ],
   [
    "src/repo_loader.rs",
    368,
    368
   ],
   [
    "src/repo_loader.rs",
    1185,
    1185
   ],
   [
    "src/repo_loader.rs",
    1222,
    1222
   ],
   [
    "src/repo_loader.rs",
    1241,
    1241
   ],
   [
    "src/repo_loader.rs",
    1268,
    1268
   ],
   [
    "src/repo_loader.rs",
    1306,
    1306
   ],
   [
    "src/repo_loader.rs",
    1347,
    1347
   ],
   [
    "src/repo_loader.rs",
    1385,
    1385
   ],
   [
    "src/repo_loader.rs",
    1427,
    1427
   ],
   [
    "src/repo_loader.rs",
    1485,
    1485
   ],
   [
    "src/repo_loader.rs",
    1513,
    1513
   ],
   [
    "src/repo_loader.rs",
    1572,
    1572
   ],
   [
    "src/repo_loader.rs",
    1639,
    1639
   ],
   [
    "src/repo_loader.rs",
    1675,
    1675
   ],
   [
    "src/repo_loader.rs",
    1707,
    1707
   ],
   [
    "src/repo_loader.rs",
    1732,
    1732
   ],
   [
    "tests/ast/cpg_cache_test.rs",
    559,
    559
   ],
   [
    "tests/cli/nav_compat_test.rs",
    329,
    329
   ],
   [
    "tests/cli/nav_compat_test.rs",
    819,
    819
   ],
   [
    "tests/infra/parallel_equality_test.rs",
    18,
    18
   ],
   [
    "tests/infra/parallel_equality_test.rs",
    22,
    22
   ],
   [
    "tests/integration/dfg_label_parity_test.rs",
    30,
    30
   ],
   [
    "tests/integration/js_alias_callers_test.rs",
    23,
    23
   ],
   [
    "tests/integration/js_alias_rule_test.rs",
    206,
    206
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    14,
    14
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    204,
    204
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    414,
    414
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    416,
    416
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    423,
    423
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    431,
    431
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    442,
    442
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    444,
    444
   ],
   [
    "tests/integration/js_paths_cap_test.rs",
    446,
    446
   ],
   [
    "tests/integration/js_paths_common.rs",
    11,
    11
   ],
   [
    "tests/integration/js_paths_p2_test.rs",
    28,
    28
   ],
   [
    "tests/integration/js_paths_p2_test.rs",
    818,
    818
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    18,
    18
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    40,
    40
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    61,
    61
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    110,
    110
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    160,
    160
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    162,
    162
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    169,
    169
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    181,
    181
   ],
   [
    "tests/integration/js_paths_r1_test.rs",
    215,
    215
   ],
   [
    "tests/integration/js_paths_r2b_test.rs",
    29,
    29
   ],
   [
    "tests/integration/js_paths_r2c_test.rs",
    43,
    43
   ],
   [
    "tests/integration/js_paths_r2d_test.rs",
    28,
    28
   ],
   [
    "tests/integration/js_paths_r3_test.rs",
    32,
    32
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    7,
    7
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    24,
    24
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    37,
    37
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    139,
    139
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    350,
    350
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    356,
    356
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    363,
    363
   ],
   [
    "tests/integration/js_paths_repair_test.rs",
    365,
    365
   ],
   [
    "tests/integration/js_paths_test.rs",
    58,
    58
   ],
   [
    "tests/integration/js_paths_test.rs",
    268,
    268
   ],
   [
    "tests/integration/js_paths_test.rs",
    297,
    297
   ],
   [
    "tests/integration/js_paths_test.rs",
    320,
    320
   ],
   [
    "tests/integration/js_paths_test.rs",
    336,
    336
   ],
   [
    "tests/integration/js_paths_test.rs",
    350,
    350
   ],
   [
    "tests/integration/js_paths_test.rs",
    396,
    396
   ],
   [
    "tests/integration/js_paths_test.rs",
    442,
    442
   ],
   [
    "tests/integration/js_paths_test.rs",
    465,
    465
   ],
   [
    "tests/integration/js_paths_test.rs",
    490,
    490
   ],
   [
    "tests/integration/js_returned_callers_test.rs",
    21,
    21
   ],
   [
    "tests/integration/resolution_test.rs",
    5342,
    5342
   ],
   [
    "tests/integration/resolution_test.rs",
    5428,
    5428
   ],
   [
    "tests/integration/rust_self_closed_rule_test.rs",
    503,
    503
   ],
   [
    "tests/integration/rust_self_closed_rule_test.rs",
    536,
    536
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    143,
    143
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    163,
    163
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    186,
    186
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    201,
    201
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    213,
    213
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    230,
    230
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    232,
    232
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    246,
    246
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    248,
    248
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    267,
    267
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    314,
    314
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    325,
    325
   ],
   [
    "tests/lang/go/loader_hygiene_test.rs",
    372,
    372
   ],
   [
    "tests/lang/go/receiver_origin_prereq_test.rs",
    36,
    36
   ],
   [
    "tests/lang/go/test_support.rs",
    23,
    23
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    72,
    72
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    127,
    127
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    169,
    169
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    198,
    198
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    233,
    233
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    323,
    323
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    469,
    469
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    528,
    528
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    576,
    576
   ],
   [
    "tests/name_resolution/build_wiring_test.rs",
    628,
    628
   ],
   [
    "tests/navigation/cache_test.rs",
    78,
    78
   ],
   [
    "tests/navigation/cache_test.rs",
    96,
    96
   ],
   [
    "tests/navigation/cache_test.rs",
    139,
    139
   ],
   [
    "tests/navigation/cache_test.rs",
    209,
    209
   ],
   [
    "tests/navigation/cache_test.rs",
    253,
    253
   ],
   [
    "tests/navigation/cache_test.rs",
    292,
    292
   ],
   [
    "tests/navigation/cache_test.rs",
    323,
    323
   ],
   [
    "tests/navigation/cache_test.rs",
    383,
    383
   ],
   [
    "tests/navigation/cache_test.rs",
    424,
    424
   ],
   [
    "tests/navigation/cache_test.rs",
    459,
    459
   ],
   [
    "tests/navigation/cache_test.rs",
    499,
    499
   ],
   [
    "tests/navigation/cache_test.rs",
    501,
    501
   ],
   [
    "tests/navigation/cache_test.rs",
    510,
    510
   ],
   [
    "tests/navigation/cache_test.rs",
    541,
    541
   ],
   [
    "tests/navigation/cache_test.rs",
    542,
    542
   ],
   [
    "tests/navigation/cache_test.rs",
    592,
    592
   ],
   [
    "tests/navigation/callees_test.rs",
    11,
    11
   ],
   [
    "tests/navigation/callers_test.rs",
    11,
    11
   ],
   [
    "tests/navigation/callers_test.rs",
    134,
    134
   ],
   [
    "tests/navigation/callers_test.rs",
    166,
    166
   ],
   [
    "tests/navigation/ego_test.rs",
    9,
    9
   ],
   [
    "tests/navigation/framework_entry_test.rs",
    20,
    20
   ],
   [
    "tests/navigation/func_value_test.rs",
    19,
    19
   ],
   [
    "tests/navigation/glob_stats_scope_test.rs",
    46,
    46
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    90,
    90
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    188,
    188
   ],
   [
    "tests/navigation/go_concrete_cache_test.rs",
    387,
    387
   ],
   [
    "tests/navigation/index_test.rs",
    13,
    13
   ],
   [
    "tests/navigation/index_test.rs",
    31,
    31
   ],
   [
    "tests/navigation/loader_test.rs",
    13,
    13
   ],
   [
    "tests/navigation/loader_test.rs",
    32,
    32
   ],
   [
    "tests/navigation/loader_test.rs",
    41,
    41
   ],
   [
    "tests/navigation/loader_test.rs",
    59,
    59
   ],
   [
    "tests/navigation/loader_test.rs",
    86,
    86
   ],
   [
    "tests/navigation/loader_test.rs",
    104,
    104
   ],
   [
    "tests/navigation/loader_test.rs",
    106,
    106
   ],
   [
    "tests/navigation/loader_test.rs",
    108,
    108
   ],
   [
    "tests/navigation/loader_test.rs",
    110,
    110
   ],
   [
    "tests/navigation/loader_test.rs",
    112,
    112
   ],
   [
    "tests/navigation/loader_test.rs",
    134,
    134
   ],
   [
    "tests/navigation/loader_test.rs",
    168,
    168
   ],
   [
    "tests/navigation/loader_test.rs",
    174,
    174
   ],
   [
    "tests/navigation/loader_test.rs",
    176,
    176
   ],
   [
    "tests/navigation/loader_test.rs",
    179,
    179
   ],
   [
    "tests/navigation/loader_test.rs",
    181,
    181
   ],
   [
    "tests/navigation/module_graph_test.rs",
    17,
    17
   ],
   [
    "tests/navigation/nodes_at_test.rs",
    17,
    17
   ],
   [
    "tests/navigation/onboarding_test.rs",
    7,
    7
   ],
   [
    "tests/navigation/property_access_test.rs",
    19,
    19
   ],
   [
    "tests/navigation/scoped_calls_test.rs",
    20,
    20
   ],
   [
    "tests/navigation/seed_test.rs",
    11,
    11
   ],
   [
    "tests/navigation/symbol_spans_test.rs",
    21,
    21
   ],
   [
    "tests/reasoning/member_only_param_test.rs",
    23,
    23
   ],
   [
    "tests/reasoning/taint_reaches_test.rs",
    25,
    25
   ]
  ],
  "raw": {
   "fn": 6,
   "fp": 18,
   "precision": [
    0.9227467811158798,
    0.8812024553007224,
    0.9505775925444296
   ],
   "recall": [
    0.9728506787330317,
    0.9420447679739598,
    0.9874991028045189
   ],
   "tp": 215
  }
 },
 {
  "ambiguous_ok": true,
  "expected": "ambiguous_symbol_error",
  "id": "ambiguous-symbol-contract",
  "oracle_sites": [],
  "outcome": "ok",
  "prism_sites": []
 }
]
```
