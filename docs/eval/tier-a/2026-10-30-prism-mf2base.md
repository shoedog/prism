# Tier-A run — prism-mf2base (2026-10-30)

- corpus: `prism-mf2base` @ `c89b878421ae`
- prism: `c89b878421ae` · oracle: rust-analyzer 1.94.0 (4a4ef493 2026-03-02) · seed: 42 · harness: `37b60d4469608ee4560196603275ce8855cd2e72`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 9.021, 'm1_oracle_inventory': 26.815, 'm2': 45.922, 'pinned': 8.718, 'matrix': 7.595, 'm3': 0.004, 'total': 105.264}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}
- oracle retries: 0 · restarts: 0

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.84–1.00] | 0.15 [0.10–0.21] | 1.00 [0.84–1.00] | 1.00 [0.84–1.00] | 1.00 [0.84–1.00] | 0.23 [0.16–0.33] | 20/0/0 | 117 | 0 |
| C-name | 1.00 [0.85–1.00] | 0.96 [0.79–0.99] | 1.00 [0.85–1.00] | 1.00 [0.85–1.00] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 22/0/0 | 1 | 0 |
| Q-scoped | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 8/0/0 | 0 | 0 |
| U-free | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 7/0/0 | 0 | 0 |
| U-method | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 17/0/0 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.84–1.00] | 0.15 [0.10–0.21] | 20/0/117 | 0 | 0 | 0 |
| C-name | 1.00 [0.85–1.00] | 0.96 [0.79–0.99] | 22/0/1 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 8/0/0 | 0 | 0 | 0 |
| U-free | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 7/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.77–1.00] | 0.76 [0.53–0.90] | 13/0/4 | 4 | 4 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 5/0/0 | 0 | 0 |
| C-name | 0.45 [0.21–0.72] | 0.83 [0.44–0.97] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 5/0/0 | 7 | 0 |
| Q-scoped | 0.67 [0.30–0.90] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 4/0/0 | 2 | 0 |
| U-free | 0.80 [0.68–0.89] | 1.00 [0.92–1.00] | 1.00 [0.92–1.00] | 1.00 [0.92–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 45/0/0 | 11 | 0 |
| U-method | 0.00 [0.00–0.66] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.56] | 0.00 [0.00–1.00] | 0/0/0 | 2 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.51–1.00] | 0.80 [0.38–0.96] | 4/0/1 | 1 | 1 | 0 |
| C-name | 0.83 [0.44–0.97] | 0.83 [0.44–0.97] | 5/1/1 | 5 | 0 | 5 |
| Q-scoped | 0.67 [0.30–0.90] | 1.00 [0.51–1.00] | 4/2/0 | 0 | 0 | 0 |
| U-free | 0.84 [0.72–0.92] | 0.96 [0.85–0.99] | 43/8/2 | 6 | 3 | 3 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 2 | 0 | 2 |

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
 "matched": 13087,
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
 "checked": [],
 "counts": {
  "alias_site": 0,
  "ambiguous": 0,
  "confirmed_fp": 0,
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
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/absence_slice.rs:541",
  "site_fingerprint": "1c2c2b036e4f6087"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/angle_slice.rs:157",
  "site_fingerprint": "17e7d11887483b2a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/callback_dispatcher_slice.rs:79",
  "site_fingerprint": "1e66e0786e7e8302"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/contract_slice.rs:162",
  "site_fingerprint": "ecca973cc4a495c4"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/contract_slice.rs:1341",
  "site_fingerprint": "64836fc31bf290a2"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/contract_slice.rs:1345",
  "site_fingerprint": "d18e92a077fd37a8"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/full_flow.rs:36",
  "site_fingerprint": "cbb7304f6d61956b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/full_flow.rs:45",
  "site_fingerprint": "48d8aa61f0e4b759"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/horizontal_slice.rs:52",
  "site_fingerprint": "1f04fda8f3fa92bd"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/left_flow.rs:312",
  "site_fingerprint": "cbb7304f6d61956b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/left_flow.rs:322",
  "site_fingerprint": "48d8aa61f0e4b759"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/parent_function.rs:26",
  "site_fingerprint": "cbb7304f6d61956b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/provenance_slice.rs:667",
  "site_fingerprint": "a4c4e6b3d35a10f0"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/quantum_slice.rs:47",
  "site_fingerprint": "ecca973cc4a495c4"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/relevant_slice.rs:66",
  "site_fingerprint": "d3c0b34746ae7cd5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/spiral_slice.rs:78",
  "site_fingerprint": "f12255440203c66a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/spiral_slice.rs:218",
  "site_fingerprint": "d1c63cb1cc61d003"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/symmetry_slice.rs:171",
  "site_fingerprint": "1f04fda8f3fa92bd"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/taint.rs:4873",
  "site_fingerprint": "812b6550f46351fb"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/thin_slice.rs:29",
  "site_fingerprint": "cbb7304f6d61956b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/algorithms/thin_slice.rs:38",
  "site_fingerprint": "48d8aa61f0e4b759"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/anon_enable_tests.rs:216",
  "site_fingerprint": "c9f01236ab2659b5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "src/output/review.rs:391",
  "site_fingerprint": "f4578ee8cc9e2931"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "tests/common/mod.rs:1170",
  "site_fingerprint": "da7ae0ab643fe5bd"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/ast/tree_access.rs:8",
  "site": "tests/lang/python/decorated_test.rs:215",
  "site_fingerprint": "5dee7f5446b03b34"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "src/cpg/exact_caller_read_tests.rs:606",
  "site_fingerprint": "12c98d0d1db4e18f"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "src/data_flow/tests/queries.rs:129",
  "site_fingerprint": "f8e8b9c09545e80b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "src/data_flow/tests/queries.rs:189",
  "site_fingerprint": "12a65b8122798c0c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "src/data_flow/tests/queries.rs:217",
  "site_fingerprint": "a3957d39e9011353"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "src/data_flow/tests/representatives.rs:112",
  "site_fingerprint": "759d48e15465fdfd"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "tests/integration/dfg_label_parity_test.rs:490",
  "site_fingerprint": "d41baaea2cbaf9a1"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/data_flow/queries.rs:44",
  "site": "tests/integration/dfg_label_parity_test.rs:523",
  "site_fingerprint": "c8b5e4448b83aff5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/executable_owner/integration/tests.rs:4",
  "site": "src/executable_owner/integration/tests.rs:405",
  "site_fingerprint": "92edeea683cac6f7"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_concurrency_tests.rs:105",
  "site_fingerprint": "045a0c9b6d930671"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_concurrency_tests.rs:135",
  "site_fingerprint": "7faea41347632ed3"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_isolation_tests.rs:56",
  "site_fingerprint": "29bff5fd0f29fc6d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_isolation_tests.rs:86",
  "site_fingerprint": "838552fa7ecef455"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_isolation_tests.rs:91",
  "site_fingerprint": "adb24659d2aee6c3"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_isolation_tests.rs:163",
  "site_fingerprint": "253dde041e024d6a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_panic_tests.rs:64",
  "site_fingerprint": "9f6c39872020c20c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_panic_tests.rs:162",
  "site_fingerprint": "d91e34e34f7f3688"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_panic_tests.rs:297",
  "site_fingerprint": "4c6bab1d4bc0eb29"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_panic_tests.rs:299",
  "site_fingerprint": "75dc9c9df7a3d40d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:200",
  "site_fingerprint": "253dde041e024d6a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:205",
  "site_fingerprint": "41600c9883163ba0"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:221",
  "site_fingerprint": "f9efac8f44789e3d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:235",
  "site_fingerprint": "8db600f96d7b5356"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:253",
  "site_fingerprint": "253dde041e024d6a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:261",
  "site_fingerprint": "41600c9883163ba0"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:272",
  "site_fingerprint": "5ef4fc032f3d52d7"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:283",
  "site_fingerprint": "4daeaa182e9b499d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:309",
  "site_fingerprint": "9f6c39872020c20c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:334",
  "site_fingerprint": "9f6c39872020c20c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_parity_tests.rs:356",
  "site_fingerprint": "69bd403631f39a7b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_ready_tests.rs:31",
  "site_fingerprint": "722769d23353f455"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_ready_tests.rs:39",
  "site_fingerprint": "3afb0e5f45d13ffe"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_ready_tests.rs:43",
  "site_fingerprint": "1d7d9e522d652b02"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:104",
  "site_fingerprint": "a56eac6a835cb30c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:110",
  "site_fingerprint": "6fc42b724341fc9e"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:117",
  "site_fingerprint": "26a764de33def659"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:124",
  "site_fingerprint": "740ac07c34618ff5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:153",
  "site_fingerprint": "253dde041e024d6a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_refresh_tests.rs:204",
  "site_fingerprint": "253dde041e024d6a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:104",
  "site_fingerprint": "37db7f3ea288deff"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:130",
  "site_fingerprint": "6105ba3eeabeeca0"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:139",
  "site_fingerprint": "3c0ad31edbcb75a8"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:141",
  "site_fingerprint": "f87f58f73b58f49b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:142",
  "site_fingerprint": "b1302e17baf7ed4e"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:170",
  "site_fingerprint": "8435cbe8e4c91909"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:181",
  "site_fingerprint": "dac850c921c3727b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:213",
  "site_fingerprint": "b23947f236afb395"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:221",
  "site_fingerprint": "dac850c921c3727b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:235",
  "site_fingerprint": "ef22f4d18bd59323"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:252",
  "site_fingerprint": "2e6f0dd8a5e74b51"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:356",
  "site_fingerprint": "3c5e57044a27d757"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:373",
  "site_fingerprint": "7d16a23be7c97c68"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:378",
  "site_fingerprint": "292929886ae3b4c1"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:394",
  "site_fingerprint": "dac850c921c3727b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:441",
  "site_fingerprint": "3afb0e5f45d13ffe"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:457",
  "site_fingerprint": "2e6f0dd8a5e74b51"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:485",
  "site_fingerprint": "722769d23353f455"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:500",
  "site_fingerprint": "82d82989dc56ef22"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:524",
  "site_fingerprint": "9f78f7ac96ab7278"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/connection_tests.rs:535",
  "site_fingerprint": "3a66a474714b8a2d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:24",
  "site_fingerprint": "7ecbe0373be066d4"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:28",
  "site_fingerprint": "ad98866b64ba071a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:217",
  "site_fingerprint": "4eb8b02915788849"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:252",
  "site_fingerprint": "6b62709bf34afdcf"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:265",
  "site_fingerprint": "7f5e9a96e441e71e"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_connection_tests.rs:346",
  "site_fingerprint": "4f8658660098cb90"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_event_tests.rs:132",
  "site_fingerprint": "66e21f220b243fce"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_event_tests.rs:215",
  "site_fingerprint": "2ce17b038d195908"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:28",
  "site_fingerprint": "b2508b2cca79ab06"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:34",
  "site_fingerprint": "a4c38b69808b721d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:47",
  "site_fingerprint": "165d351807df25f1"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:80",
  "site_fingerprint": "4a503a2e844cb3fe"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:84",
  "site_fingerprint": "ca56ae4f865e399d"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_reload_tests.rs:197",
  "site_fingerprint": "ea4bfa4ffab1a50c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_sink_tests.rs:50",
  "site_fingerprint": "ebc4505ff94c1643"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_sink_tests.rs:79",
  "site_fingerprint": "40dc878064a52fd3"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_tests.rs:58",
  "site_fingerprint": "35167027d68fd2bd"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_tests.rs:59",
  "site_fingerprint": "3fa4f5d15309406a"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_tests.rs:65",
  "site_fingerprint": "d0e19aca8c7003de"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_tests.rs:94",
  "site_fingerprint": "ff70695143e6adab"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_tests.rs:160",
  "site_fingerprint": "3537029c2677a949"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:44",
  "site_fingerprint": "4c13cf524a738d71"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:220",
  "site_fingerprint": "b7ad2e3d27388654"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:221",
  "site_fingerprint": "d0432de46f83236c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:225",
  "site_fingerprint": "1d85ed325a79d69c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:231",
  "site_fingerprint": "6eb5c8fdc4746018"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/idle_wire_logging_tests.rs:247",
  "site_fingerprint": "e3c04c2db8d730e2"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/server_accept_tests.rs:137",
  "site_fingerprint": "a8d941b5dbe3e2d8"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/server_control_tests.rs:172",
  "site_fingerprint": "73a3d15b221f3f80"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/server_control_tests.rs:175",
  "site_fingerprint": "e9c0cd2235ad3201"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/server_drain_tests.rs:88",
  "site_fingerprint": "5d06017a78c2b967"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/serving_event_tests.rs:320",
  "site_fingerprint": "5455c8b5f7f6f5d6"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/serving_event_tests.rs:321",
  "site_fingerprint": "2fb36be51678016b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/mcp/daemon/wire_support.rs:68",
  "site": "src/mcp/daemon/serving_event_tests.rs:322",
  "site_fingerprint": "2a05981ee3840852"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/ast.rs:2121",
  "site": "src/ast.rs:2125",
  "site_fingerprint": "f1d0a07bb3f07f6e",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/cpg/callback_identity_tests.rs:506",
  "site": "src/cpg/callback_identity_tests.rs:526",
  "site_fingerprint": "f7c15a616c231a29"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/cpg/callee_identity_tests.rs:19",
  "site": "src/cpg/callee_identity_tests.rs:24",
  "site_fingerprint": "b7665c1fa27127bb"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "src/cpg/callee_identity_tests.rs:19",
  "site": "src/cpg/callee_identity_tests.rs:25",
  "site_fingerprint": "9f5ac0b4dcf9f5c7",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/cpg/callee_identity_tests.rs:19",
  "site": "src/cpg/callee_identity_tests.rs:21",
  "site_fingerprint": "bf28fe6ed42a1905"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/executable_owner/integration/tests.rs:4",
  "site": "src/executable_owner/integration/tests.rs:11",
  "site_fingerprint": "66cdc61fa9670be0",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/executable_owner/integration/tests.rs:4",
  "site": "src/executable_owner/integration/tests.rs:15",
  "site_fingerprint": "86113a20583a5f9d",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/executable_owner/integration/tests.rs:4",
  "site": "src/executable_owner/integration/tests.rs:17",
  "site_fingerprint": "140e4f91dc5da89f",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/frameworks/python/flask.rs:226",
  "site": "src/frameworks/python/flask.rs:231",
  "site_fingerprint": "bc772da4b2236606",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:71",
  "site_fingerprint": "58d40ae92a8a6578"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:91",
  "site_fingerprint": "959dd81258cda5fc"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:122",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:152",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:182",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:195",
  "site_fingerprint": "d2e134d7522d07d0"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:201",
  "site_fingerprint": "56de19243e1be085"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:215",
  "site_fingerprint": "6b9368174190d6dd",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:259",
  "site_fingerprint": "f225238c223432c8",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:263",
  "site_fingerprint": "78497b142a04c2c6",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:54",
  "site": "src/main.rs:339",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "tests/lang/python/inherited_receiver_test.rs:45",
  "site": "tests/lang/python/inherited_receiver_test.rs:48",
  "site_fingerprint": "71399b0f5fc953f6"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "tests/mcp/daemon_process_test.rs:238",
  "site": "tests/mcp/daemon_process_test.rs:243",
  "site_fingerprint": "5c890703cf5e7a55",
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
   "src/algorithms/taint.rs:1768",
   "src/algorithms/taint.rs:4445",
   "src/algorithms/taint.rs:4455",
   "src/algorithms/taint.rs:4463",
   "src/algorithms/taint.rs:4475"
  ],
  "outcome": "flip_candidate",
  "prism_only": [],
  "prism_sites": [
   "src/algorithms/taint.rs:1768",
   "src/algorithms/taint.rs:4445",
   "src/algorithms/taint.rs:4455",
   "src/algorithms/taint.rs:4463",
   "src/algorithms/taint.rs:4475"
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
   "tests/navigation/module_graph_test.rs:93",
   "tests/navigation/module_graph_test.rs:142",
   "tests/navigation/module_graph_test.rs:152",
   "tests/navigation/module_graph_test.rs:169",
   "tests/navigation/module_graph_test.rs:219",
   "tests/navigation/module_graph_test.rs:236",
   "tests/navigation/module_graph_test.rs:253",
   "tests/navigation/module_graph_test.rs:281",
   "tests/navigation/module_graph_test.rs:307",
   "tests/navigation/module_graph_test.rs:332",
   "tests/navigation/module_graph_test.rs:353",
   "tests/navigation/module_graph_test.rs:455",
   "tests/name_resolution/build_wiring_test.rs:642",
   "tests/name_resolution/build_wiring_test.rs:643",
   "tests/navigation/cache_test.rs:331",
   "tests/navigation/cache_test.rs:345",
   "src/main.rs:194",
   "src/mcp/tools.rs:230",
   "tests/navigation/scoped_calls_test.rs:225"
  ],
  "outcome": "missing",
  "prism_only": [],
  "prism_sites": [
   "src/main.rs:194",
   "src/mcp/tools.rs:230",
   "tests/name_resolution/build_wiring_test.rs:642",
   "tests/name_resolution/build_wiring_test.rs:643",
   "tests/navigation/cache_test.rs:331",
   "tests/navigation/cache_test.rs:345",
   "tests/navigation/module_graph_test.rs:93",
   "tests/navigation/module_graph_test.rs:142",
   "tests/navigation/module_graph_test.rs:152",
   "tests/navigation/module_graph_test.rs:169",
   "tests/navigation/module_graph_test.rs:219",
   "tests/navigation/module_graph_test.rs:236",
   "tests/navigation/module_graph_test.rs:253",
   "tests/navigation/module_graph_test.rs:281",
   "tests/navigation/module_graph_test.rs:307",
   "tests/navigation/module_graph_test.rs:332",
   "tests/navigation/module_graph_test.rs:353",
   "tests/navigation/module_graph_test.rs:455",
   "tests/navigation/scoped_calls_test.rs:225"
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
   "tests/integration/resolution_test.rs:5299",
   "tests/integration/resolution_test.rs:5342",
   "tests/integration/resolution_test.rs:5368",
   "tests/integration/resolution_test.rs:5428",
   "src/repo_loader.rs:333",
   "src/repo_loader.rs:368",
   "src/repo_loader.rs:1185",
   "src/repo_loader.rs:1222",
   "src/repo_loader.rs:1241",
   "src/repo_loader.rs:1268",
   "src/repo_loader.rs:1306",
   "src/repo_loader.rs:1347",
   "src/repo_loader.rs:1385",
   "src/repo_loader.rs:1427",
   "src/repo_loader.rs:1485",
   "src/repo_loader.rs:1513",
   "src/repo_loader.rs:1572",
   "src/repo_loader.rs:1639",
   "src/repo_loader.rs:1675",
   "src/repo_loader.rs:1707",
   "src/repo_loader.rs:1732",
   "src/name_resolution/lookup_invalidation_tests.rs:165",
   "tests/navigation/func_value_test.rs:19",
   "examples/parameter_site_census.rs:219",
   "src/mcp/shared/cache_base_tests.rs:276",
   "tests/reasoning/member_only_param_test.rs:23",
   "tests/integration/js_paths_test.rs:58",
   "tests/integration/js_paths_test.rs:268",
   "tests/integration/js_paths_test.rs:297",
   "tests/integration/js_paths_test.rs:320",
   "tests/integration/js_paths_test.rs:336",
   "tests/integration/js_paths_test.rs:350",
   "tests/integration/js_paths_test.rs:396",
   "tests/integration/js_paths_test.rs:442",
   "tests/integration/js_paths_test.rs:465",
   "tests/integration/js_paths_test.rs:490",
   "tests/lang/go/receiver_origin_prereq_test.rs:36",
   "tests/integration/js_alias_rule_test.rs:206",
   "examples/project_membership_census.rs:63",
   "tests/navigation/seed_test.rs:11",
   "tests/integration/js_paths_common.rs:11",
   "tests/navigation/index_test.rs:13",
   "tests/navigation/index_test.rs:31",
   "src/executable_owner/acquisition.rs:127",
   "src/build_pool/boundary_tests.rs:275",
   "src/reasoning/seeds.rs:468",
   "tests/reasoning/taint_reaches_test.rs:25",
   "tests/lang/go/test_support.rs:23",
   "tests/integration/js_paths_r2b_test.rs:29",
   "src/cpg/tests.rs:2542",
   "src/api/nav.rs:26",
   "src/executable_owner/integration/tests.rs:22",
   "src/executable_owner/integration/tests.rs:79",
   "tests/navigation/module_graph_test.rs:17",
   "tests/navigation/ego_test.rs:9",
   "src/navigation/cache_lifetime_tests.rs:96",
   "src/navigation/cache_lifetime_tests.rs:124",
   "tests/integration/rust_self_closed_rule_test.rs:503",
   "tests/integration/rust_self_closed_rule_test.rs:536",
   "tests/navigation/callees_test.rs:11",
   "tests/integration/js_paths_r3_test.rs:32",
   "examples/dfg_census.rs:359",
   "src/cpg_cache.rs:840",
   "tests/navigation/property_access_test.rs:19",
   "tests/navigation/glob_stats_scope_test.rs:46",
   "tests/integration/js_paths_cap_test.rs:431",
   "tests/integration/js_paths_cap_test.rs:444",
   "tests/integration/js_paths_cap_test.rs:446",
   "tests/integration/js_paths_cap_test.rs:414",
   "tests/integration/js_paths_cap_test.rs:416",
   "tests/integration/js_paths_cap_test.rs:423",
   "tests/integration/js_paths_cap_test.rs:442",
   "tests/integration/js_paths_repair_test.rs:363",
   "tests/integration/js_paths_repair_test.rs:365",
   "tests/integration/js_paths_repair_test.rs:350",
   "tests/integration/js_paths_repair_test.rs:356",
   "src/mcp/freshness.rs:429",
   "tests/navigation/symbol_spans_test.rs:21",
   "tests/cli/nav_compat_test.rs:329",
   "tests/cli/nav_compat_test.rs:819",
   "tests/integration/js_paths_p2_test.rs:28",
   "tests/integration/js_paths_p2_test.rs:818",
   "tests/lang/go/loader_hygiene_test.rs:143",
   "tests/lang/go/loader_hygiene_test.rs:163",
   "tests/lang/go/loader_hygiene_test.rs:186",
   "tests/lang/go/loader_hygiene_test.rs:201",
   "tests/lang/go/loader_hygiene_test.rs:213",
   "tests/lang/go/loader_hygiene_test.rs:230",
   "tests/lang/go/loader_hygiene_test.rs:232",
   "tests/lang/go/loader_hygiene_test.rs:246",
   "tests/lang/go/loader_hygiene_test.rs:248",
   "tests/lang/go/loader_hygiene_test.rs:267",
   "tests/lang/go/loader_hygiene_test.rs:314",
   "tests/lang/go/loader_hygiene_test.rs:325",
   "tests/lang/go/loader_hygiene_test.rs:372",
   "src/navigation/anon_guard_tests.rs:19",
   "src/executable_owner/admission.rs:120",
   "tests/name_resolution/build_wiring_test.rs:72",
   "tests/name_resolution/build_wiring_test.rs:127",
   "tests/name_resolution/build_wiring_test.rs:169",
   "tests/name_resolution/build_wiring_test.rs:198",
   "tests/name_resolution/build_wiring_test.rs:233",
   "tests/name_resolution/build_wiring_test.rs:323",
   "tests/name_resolution/build_wiring_test.rs:469",
   "tests/name_resolution/build_wiring_test.rs:528",
   "tests/name_resolution/build_wiring_test.rs:576",
   "tests/name_resolution/build_wiring_test.rs:628",
   "src/navigation/mod.rs:640",
   "src/navigation/mod.rs:655",
   "src/navigation/mod.rs:678",
   "src/navigation/mod.rs:722",
   "tests/integration/js_paths_r2c_test.rs:43",
   "src/executable_owner/audit_tests.rs:310",
   "src/executable_owner/audit_tests.rs:322",
   "src/executable_owner/audit_tests.rs:333",
   "tests/navigation/nodes_at_test.rs:17",
   "tests/navigation/framework_entry_test.rs:20",
   "src/cpg/callback_identity_tests.rs:776",
   "src/navigation/call_edge_cache.rs:780",
   "tests/navigation/callers_test.rs:11",
   "tests/navigation/callers_test.rs:134",
   "tests/navigation/callers_test.rs:166",
   "tests/integration/js_paths_repair_test.rs:7",
   "tests/integration/js_paths_repair_test.rs:24",
   "tests/integration/js_paths_repair_test.rs:37",
   "tests/integration/js_paths_repair_test.rs:139",
   "src/mcp/session.rs:415",
   "src/mcp/session.rs:435",
   "src/mcp/session.rs:472",
   "tests/navigation/scoped_calls_test.rs:20",
   "tests/integration/js_paths_cap_test.rs:14",
   "tests/integration/js_paths_cap_test.rs:204",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/navigation/go_concrete_cache_test.rs:90",
   "tests/navigation/go_concrete_cache_test.rs:188",
   "tests/navigation/go_concrete_cache_test.rs:387",
   "tests/integration/dfg_label_parity_test.rs:30",
   "tests/ast/cpg_cache_test.rs:559",
   "src/navigation/inventory.rs:16",
   "src/mcp/freshness_policy_tests.rs:393",
   "src/mcp/freshness_capture_tests.rs:46",
   "src/mcp/freshness_capture_tests.rs:60",
   "src/mcp/freshness_capture_tests.rs:75",
   "src/mcp/freshness_capture_tests.rs:102",
   "src/mcp/freshness_capture_tests.rs:119",
   "src/mcp/freshness_capture_tests.rs:122",
   "src/mcp/freshness_capture_tests.rs:146",
   "src/mcp/freshness_capture_tests.rs:159",
   "src/mcp/freshness_capture_tests.rs:181",
   "src/mcp/freshness_capture_tests.rs:185",
   "src/mcp/freshness_capture_tests.rs:205",
   "src/mcp/freshness_capture_tests.rs:233",
   "src/mcp/freshness_capture_tests.rs:253",
   "src/mcp/freshness_capture_tests.rs:268",
   "src/mcp/freshness_capture_tests.rs:301",
   "tests/integration/js_paths_r1_test.rs:18",
   "tests/integration/js_paths_r1_test.rs:40",
   "tests/integration/js_paths_r1_test.rs:61",
   "tests/integration/js_paths_r1_test.rs:110",
   "tests/integration/js_paths_r1_test.rs:160",
   "tests/integration/js_paths_r1_test.rs:162",
   "tests/integration/js_paths_r1_test.rs:169",
   "tests/integration/js_paths_r1_test.rs:181",
   "tests/integration/js_paths_r1_test.rs:215",
   "tests/integration/js_alias_callers_test.rs:23",
   "src/js_exports/memo/pipeline_tests.rs:59",
   "src/js_exports/memo/pipeline_tests.rs:82",
   "src/js_exports/memo/pipeline_tests.rs:115",
   "src/executable_owner/integration.rs:156",
   "tests/navigation/loader_test.rs:13",
   "tests/navigation/loader_test.rs:32",
   "tests/navigation/loader_test.rs:41",
   "tests/navigation/loader_test.rs:59",
   "tests/navigation/loader_test.rs:86",
   "tests/navigation/loader_test.rs:104",
   "tests/navigation/loader_test.rs:106",
   "tests/navigation/loader_test.rs:108",
   "tests/navigation/loader_test.rs:110",
   "tests/navigation/loader_test.rs:112",
   "tests/navigation/loader_test.rs:134",
   "tests/navigation/loader_test.rs:168",
   "tests/navigation/loader_test.rs:174",
   "tests/navigation/loader_test.rs:176",
   "tests/navigation/loader_test.rs:179",
   "tests/navigation/loader_test.rs:181",
   "src/call_graph.rs:7313",
   "src/call_graph.rs:7326",
   "src/cpg/fallback_lifetime_tests.rs:139",
   "src/cpg/fallback_lifetime_tests.rs:148",
   "src/cpg/fallback_lifetime_tests.rs:166",
   "src/cpg/fallback_lifetime_tests.rs:173",
   "src/cpg/fallback_lifetime_tests.rs:199",
   "src/cpg/fallback_lifetime_tests.rs:206",
   "src/cpg/fallback_lifetime_tests.rs:224",
   "src/cpg/fallback_lifetime_tests.rs:233",
   "src/cpg/fallback_lifetime_tests.rs:263",
   "src/cpg/fallback_lifetime_tests.rs:278",
   "src/cpg/fallback_lifetime_tests.rs:286",
   "src/cpg/fallback_lifetime_tests.rs:336",
   "src/cpg/fallback_lifetime_tests.rs:339",
   "src/cpg/fallback_lifetime_tests.rs:369",
   "tests/integration/js_returned_callers_test.rs:21",
   "tests/integration/js_paths_r2d_test.rs:28",
   "tests/navigation/cache_test.rs:78",
   "tests/navigation/cache_test.rs:96",
   "tests/navigation/cache_test.rs:139",
   "tests/navigation/cache_test.rs:209",
   "tests/navigation/cache_test.rs:253",
   "tests/navigation/cache_test.rs:292",
   "tests/navigation/cache_test.rs:323",
   "tests/navigation/cache_test.rs:383",
   "tests/navigation/cache_test.rs:424",
   "tests/navigation/cache_test.rs:459",
   "tests/navigation/cache_test.rs:499",
   "tests/navigation/cache_test.rs:501",
   "tests/navigation/cache_test.rs:510",
   "tests/navigation/cache_test.rs:541",
   "tests/navigation/cache_test.rs:542",
   "tests/navigation/cache_test.rs:592",
   "tests/navigation/onboarding_test.rs:7"
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
   "docs/superpowers/plans/2026-10-06-js-param-defs/probes/return_inputs.rs:7",
   "src/api/nav.rs:26",
   "src/build_pool/boundary_tests.rs:275",
   "src/call_graph.rs:7313",
   "src/call_graph.rs:7326",
   "src/cpg/callback_identity_tests.rs:776",
   "src/cpg/fallback_lifetime_tests.rs:139",
   "src/cpg/fallback_lifetime_tests.rs:148",
   "src/cpg/fallback_lifetime_tests.rs:166",
   "src/cpg/fallback_lifetime_tests.rs:173",
   "src/cpg/fallback_lifetime_tests.rs:199",
   "src/cpg/fallback_lifetime_tests.rs:206",
   "src/cpg/fallback_lifetime_tests.rs:224",
   "src/cpg/fallback_lifetime_tests.rs:233",
   "src/cpg/fallback_lifetime_tests.rs:263",
   "src/cpg/fallback_lifetime_tests.rs:278",
   "src/cpg/fallback_lifetime_tests.rs:286",
   "src/cpg/fallback_lifetime_tests.rs:336",
   "src/cpg/fallback_lifetime_tests.rs:339",
   "src/cpg/fallback_lifetime_tests.rs:369",
   "src/cpg/tests.rs:2542",
   "src/cpg_cache.rs:840",
   "src/executable_owner/acquisition.rs:127",
   "src/executable_owner/admission.rs:120",
   "src/executable_owner/audit_tests.rs:310",
   "src/executable_owner/audit_tests.rs:322",
   "src/executable_owner/audit_tests.rs:333",
   "src/executable_owner/integration.rs:156",
   "src/executable_owner/integration/tests.rs:22",
   "src/executable_owner/integration/tests.rs:79",
   "src/js_exports/memo/pipeline_tests.rs:59",
   "src/js_exports/memo/pipeline_tests.rs:115",
   "src/mcp/freshness.rs:429",
   "src/mcp/freshness_capture_tests.rs:46",
   "src/mcp/freshness_capture_tests.rs:60",
   "src/mcp/freshness_capture_tests.rs:75",
   "src/mcp/freshness_capture_tests.rs:102",
   "src/mcp/freshness_capture_tests.rs:119",
   "src/mcp/freshness_capture_tests.rs:122",
   "src/mcp/freshness_capture_tests.rs:146",
   "src/mcp/freshness_capture_tests.rs:159",
   "src/mcp/freshness_capture_tests.rs:181",
   "src/mcp/freshness_capture_tests.rs:185",
   "src/mcp/freshness_capture_tests.rs:205",
   "src/mcp/freshness_capture_tests.rs:233",
   "src/mcp/freshness_capture_tests.rs:253",
   "src/mcp/freshness_capture_tests.rs:268",
   "src/mcp/freshness_capture_tests.rs:301",
   "src/mcp/freshness_policy_tests.rs:393",
   "src/mcp/session.rs:415",
   "src/mcp/session.rs:435",
   "src/mcp/session.rs:472",
   "src/mcp/shared/cache_base_tests.rs:276",
   "src/name_resolution/lookup_invalidation_tests.rs:165",
   "src/navigation/anon_guard_tests.rs:19",
   "src/navigation/cache_lifetime_tests.rs:96",
   "src/navigation/cache_lifetime_tests.rs:124",
   "src/navigation/call_edge_cache.rs:780",
   "src/navigation/inventory.rs:16",
   "src/navigation/mod.rs:640",
   "src/navigation/mod.rs:655",
   "src/navigation/mod.rs:678",
   "src/navigation/mod.rs:722",
   "src/reasoning/seeds.rs:468",
   "src/repo_loader.rs:333",
   "src/repo_loader.rs:368",
   "src/repo_loader.rs:1185",
   "src/repo_loader.rs:1222",
   "src/repo_loader.rs:1241",
   "src/repo_loader.rs:1268",
   "src/repo_loader.rs:1306",
   "src/repo_loader.rs:1347",
   "src/repo_loader.rs:1385",
   "src/repo_loader.rs:1427",
   "src/repo_loader.rs:1485",
   "src/repo_loader.rs:1513",
   "src/repo_loader.rs:1572",
   "src/repo_loader.rs:1639",
   "src/repo_loader.rs:1675",
   "src/repo_loader.rs:1707",
   "src/repo_loader.rs:1732",
   "tests/ast/cpg_cache_test.rs:559",
   "tests/cli/nav_compat_test.rs:329",
   "tests/cli/nav_compat_test.rs:819",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/integration/dfg_label_parity_test.rs:30",
   "tests/integration/js_alias_callers_test.rs:23",
   "tests/integration/js_alias_rule_test.rs:206",
   "tests/integration/js_paths_cap_test.rs:14",
   "tests/integration/js_paths_cap_test.rs:204",
   "tests/integration/js_paths_cap_test.rs:414",
   "tests/integration/js_paths_cap_test.rs:416",
   "tests/integration/js_paths_cap_test.rs:423",
   "tests/integration/js_paths_cap_test.rs:431",
   "tests/integration/js_paths_cap_test.rs:442",
   "tests/integration/js_paths_cap_test.rs:444",
   "tests/integration/js_paths_cap_test.rs:446",
   "tests/integration/js_paths_common.rs:11",
   "tests/integration/js_paths_p2_test.rs:28",
   "tests/integration/js_paths_p2_test.rs:818",
   "tests/integration/js_paths_r1_test.rs:18",
   "tests/integration/js_paths_r1_test.rs:40",
   "tests/integration/js_paths_r1_test.rs:61",
   "tests/integration/js_paths_r1_test.rs:110",
   "tests/integration/js_paths_r1_test.rs:160",
   "tests/integration/js_paths_r1_test.rs:162",
   "tests/integration/js_paths_r1_test.rs:169",
   "tests/integration/js_paths_r1_test.rs:181",
   "tests/integration/js_paths_r1_test.rs:215",
   "tests/integration/js_paths_r2b_test.rs:29",
   "tests/integration/js_paths_r2c_test.rs:43",
   "tests/integration/js_paths_r2d_test.rs:28",
   "tests/integration/js_paths_r3_test.rs:32",
   "tests/integration/js_paths_repair_test.rs:7",
   "tests/integration/js_paths_repair_test.rs:24",
   "tests/integration/js_paths_repair_test.rs:37",
   "tests/integration/js_paths_repair_test.rs:139",
   "tests/integration/js_paths_repair_test.rs:350",
   "tests/integration/js_paths_repair_test.rs:356",
   "tests/integration/js_paths_repair_test.rs:363",
   "tests/integration/js_paths_repair_test.rs:365",
   "tests/integration/js_paths_test.rs:58",
   "tests/integration/js_paths_test.rs:268",
   "tests/integration/js_paths_test.rs:297",
   "tests/integration/js_paths_test.rs:320",
   "tests/integration/js_paths_test.rs:336",
   "tests/integration/js_paths_test.rs:350",
   "tests/integration/js_paths_test.rs:396",
   "tests/integration/js_paths_test.rs:442",
   "tests/integration/js_paths_test.rs:465",
   "tests/integration/js_paths_test.rs:490",
   "tests/integration/js_returned_callers_test.rs:21",
   "tests/integration/resolution_test.rs:5342",
   "tests/integration/resolution_test.rs:5428",
   "tests/integration/rust_self_closed_rule_test.rs:503",
   "tests/integration/rust_self_closed_rule_test.rs:536",
   "tests/lang/go/loader_hygiene_test.rs:143",
   "tests/lang/go/loader_hygiene_test.rs:163",
   "tests/lang/go/loader_hygiene_test.rs:186",
   "tests/lang/go/loader_hygiene_test.rs:201",
   "tests/lang/go/loader_hygiene_test.rs:213",
   "tests/lang/go/loader_hygiene_test.rs:230",
   "tests/lang/go/loader_hygiene_test.rs:232",
   "tests/lang/go/loader_hygiene_test.rs:246",
   "tests/lang/go/loader_hygiene_test.rs:248",
   "tests/lang/go/loader_hygiene_test.rs:267",
   "tests/lang/go/loader_hygiene_test.rs:314",
   "tests/lang/go/loader_hygiene_test.rs:325",
   "tests/lang/go/loader_hygiene_test.rs:372",
   "tests/lang/go/receiver_origin_prereq_test.rs:36",
   "tests/lang/go/test_support.rs:23",
   "tests/name_resolution/build_wiring_test.rs:72",
   "tests/name_resolution/build_wiring_test.rs:127",
   "tests/name_resolution/build_wiring_test.rs:169",
   "tests/name_resolution/build_wiring_test.rs:198",
   "tests/name_resolution/build_wiring_test.rs:233",
   "tests/name_resolution/build_wiring_test.rs:323",
   "tests/name_resolution/build_wiring_test.rs:469",
   "tests/name_resolution/build_wiring_test.rs:528",
   "tests/name_resolution/build_wiring_test.rs:576",
   "tests/name_resolution/build_wiring_test.rs:628",
   "tests/navigation/cache_test.rs:78",
   "tests/navigation/cache_test.rs:96",
   "tests/navigation/cache_test.rs:139",
   "tests/navigation/cache_test.rs:209",
   "tests/navigation/cache_test.rs:253",
   "tests/navigation/cache_test.rs:292",
   "tests/navigation/cache_test.rs:323",
   "tests/navigation/cache_test.rs:383",
   "tests/navigation/cache_test.rs:424",
   "tests/navigation/cache_test.rs:459",
   "tests/navigation/cache_test.rs:499",
   "tests/navigation/cache_test.rs:501",
   "tests/navigation/cache_test.rs:510",
   "tests/navigation/cache_test.rs:541",
   "tests/navigation/cache_test.rs:542",
   "tests/navigation/cache_test.rs:592",
   "tests/navigation/callees_test.rs:11",
   "tests/navigation/callers_test.rs:11",
   "tests/navigation/callers_test.rs:134",
   "tests/navigation/callers_test.rs:166",
   "tests/navigation/ego_test.rs:9",
   "tests/navigation/framework_entry_test.rs:20",
   "tests/navigation/func_value_test.rs:19",
   "tests/navigation/glob_stats_scope_test.rs:46",
   "tests/navigation/go_concrete_cache_test.rs:90",
   "tests/navigation/go_concrete_cache_test.rs:188",
   "tests/navigation/go_concrete_cache_test.rs:387",
   "tests/navigation/index_test.rs:13",
   "tests/navigation/index_test.rs:31",
   "tests/navigation/loader_test.rs:13",
   "tests/navigation/loader_test.rs:32",
   "tests/navigation/loader_test.rs:41",
   "tests/navigation/loader_test.rs:59",
   "tests/navigation/loader_test.rs:86",
   "tests/navigation/loader_test.rs:104",
   "tests/navigation/loader_test.rs:106",
   "tests/navigation/loader_test.rs:108",
   "tests/navigation/loader_test.rs:110",
   "tests/navigation/loader_test.rs:112",
   "tests/navigation/loader_test.rs:134",
   "tests/navigation/loader_test.rs:168",
   "tests/navigation/loader_test.rs:174",
   "tests/navigation/loader_test.rs:176",
   "tests/navigation/loader_test.rs:179",
   "tests/navigation/loader_test.rs:181",
   "tests/navigation/module_graph_test.rs:17",
   "tests/navigation/nodes_at_test.rs:17",
   "tests/navigation/onboarding_test.rs:7",
   "tests/navigation/property_access_test.rs:19",
   "tests/navigation/scoped_calls_test.rs:20",
   "tests/navigation/seed_test.rs:11",
   "tests/navigation/symbol_spans_test.rs:21",
   "tests/reasoning/member_only_param_test.rs:23",
   "tests/reasoning/taint_reaches_test.rs:25"
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
