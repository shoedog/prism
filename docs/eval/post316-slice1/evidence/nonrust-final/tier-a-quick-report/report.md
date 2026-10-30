# Tier-A run — prism (2026-09-16-slice1-07ed5ceb)

- corpus: `prism` @ `07ed5ceb6098`
- prism: `07ed5ceb6098` · oracle: rust-analyzer 1.94.0 (4a4ef493 2026-03-02) · seed: 42 · harness: `07ed5ceb6098`
- oracle_error_rate: 0.067 · sut_error_rate: 0.000 · baseline_invalid: True · oracle_not_quiescent: False
- wall (s): {'oracle_start': 36.463, 'm1_oracle_inventory': 16.062, 'm2': 161.809, 'pinned': 5.81, 'matrix': 6.7, 'm3': 0.008}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.57–1.00] | 0.28 [0.12–0.51] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.29 [0.13–0.53] | 5/0/0 | 13 | 0 |
| C-name | 0.76 [0.53–0.90] | 1.00 [0.77–1.00] | 1.00 [0.77–1.00] | 1.00 [0.77–1.00] | 0.80 [0.55–0.93] | 1.00 [0.76–1.00] | 13/0/0 | 4 | 0 |
| Q-scoped | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 7/0/0 | 0 | 0 |
| U-free | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 2/0/0 | 0 | 0 |
| U-method | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 4/0/0 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.51–1.00] | 0.22 [0.09–0.45] | 4/0/14 | 1 | 1 | 0 |
| C-name | 0.87 [0.62–0.96] | 1.00 [0.77–1.00] | 13/2/0 | 2 | 0 | 2 |
| Q-scoped | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 7/0/0 | 0 | 0 | 0 |
| U-free | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.44–1.00] | 0.75 [0.30–0.95] | 3/0/1 | 1 | 1 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 9/0/0 | 0 | 0 |
| C-name | 0.50 [0.24–0.76] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 5/0/0 | 5 | 0 |
| Q-scoped | 1.00 [0.65–1.00] | 0.78 [0.45–0.94] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 7/0/0 | 2 | 0 |
| U-free | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 1/0/0 | 2 | 0 |
| U-method | 1.00 [0.76–1.00] | 1.00 [0.76–1.00] | 1.00 [0.76–1.00] | 1.00 [0.76–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 12/0/0 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 9/0/0 | 0 | 0 | 0 |
| C-name | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 5/0/0 | 5 | 1 | 4 |
| Q-scoped | 1.00 [0.65–1.00] | 0.78 [0.45–0.94] | 7/0/2 | 2 | 2 | 0 |
| U-free | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 1/2/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.76–1.00] | 1.00 [0.76–1.00] | 12/0/0 | 0 | 0 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 0,
 "matched": 8654,
 "prism_extra": 0,
 "prism_missing": 45,
 "snapshot_prism_missing": 45
}
```

## M3 spot-check

```json
{
 "cap": 10,
 "checked": [
  {
   "probe": "callers:src/cpg/multiline_call_arg_tests.rs:216",
   "site": "src/ast_required_parameter_tests.rs:141",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:src/cpg/multiline_call_arg_tests.rs:216",
   "site": "src/ast_required_parameter_tests.rs:158",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:src/go_build_profile.rs:498",
   "site": "src/ast.rs:439",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/go_build_profile.rs:498",
   "site": "src/queries.rs:315",
   "verdict": "ambiguous"
  }
 ],
 "counts": {
  "alias_site": 0,
  "ambiguous": 2,
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
    "to": "main.go:8:x"
   }
  ],
  "got_kinds": {},
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
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "a.py:2:a",
    "kill_line": null,
    "to": "a.py:3:a"
   }
  ],
  "got_kinds": {},
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
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.js:2:a",
    "kill_line": null,
    "to": "main.js:3:a"
   }
  ],
  "got_kinds": {},
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
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:2:x"
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
    "doubt": "killed",
    "from": "main.js:4:x",
    "kill_line": 6,
    "to": "main.js:7:x"
   }
  ],
  "got_kinds": {},
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
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.js:4:x",
    "kill_line": null,
    "to": "main.js:2:x"
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
    "doubt": "killed",
    "from": "main.js:4:x",
    "kill_line": 6,
    "to": "main.js:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.js:4:x",
    "kill_line": 6,
    "to": "main.js:8:x"
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
   },
   {
    "confidence": "nameonly",
    "doubt": "sameline",
    "from": "main.ts:2:a",
    "kill_line": null,
    "to": "main.ts:3:a"
   }
  ],
  "got_kinds": {},
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
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:2:x"
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
    "doubt": "killed",
    "from": "main.ts:4:x",
    "kill_line": 6,
    "to": "main.ts:7:x"
   }
  ],
  "got_kinds": {},
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
    "confidence": "nameonly",
    "doubt": "cfg_incomplete",
    "from": "main.ts:4:x",
    "kill_line": null,
    "to": "main.ts:2:x"
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
    "doubt": "killed",
    "from": "main.ts:4:x",
    "kill_line": 6,
    "to": "main.ts:7:x"
   },
   {
    "confidence": "nameonly",
    "doubt": "killed",
    "from": "main.ts:4:x",
    "kill_line": 6,
    "to": "main.ts:8:x"
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
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/ast/cpg_cache_test.rs:1391",
  "site_fingerprint": "7ee819c81a50f34b"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/ast/cpg_cache_test.rs:1437",
  "site_fingerprint": "fa63983c663a829c"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/ast/cpg_cache_test.rs:1488",
  "site_fingerprint": "a9228fdfd9257760"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/call_graph_test.rs:1825",
  "site_fingerprint": "702a5b100351cb26"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/import_binding_test.rs:936",
  "site_fingerprint": "2f5037262fec47b5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/imported_object_alias_test.rs:175",
  "site_fingerprint": "075f2c753e860e62"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/imported_object_alias_test.rs:342",
  "site_fingerprint": "1515476570b929e9"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/imported_object_alias_test.rs:352",
  "site_fingerprint": "1515476570b929e9"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/js_export_reexport_test.rs:505",
  "site_fingerprint": "c72108cf566a1bca"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/integration/resolution_test.rs:4077",
  "site_fingerprint": "06668ffbc9efdbe5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/lang/typescript/typed_receiver_test.rs:476",
  "site_fingerprint": "de5063379c1989c5"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/lang/typescript/typed_receiver_test.rs:724",
  "site_fingerprint": "30112a47932ad368"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:2238",
  "site": "tests/name_resolution/build_wiring_test.rs:332",
  "site_fingerprint": "2db8c76430fb6ac0"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "src/cpg/multiline_call_arg_tests.rs:216",
  "site": "src/ast_required_parameter_tests.rs:141",
  "site_fingerprint": "36f5de991093b17d",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "src/cpg/multiline_call_arg_tests.rs:216",
  "site": "src/ast_required_parameter_tests.rs:158",
  "site_fingerprint": "f928f8ffbe578ec2",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callers",
  "seed_def": "src/go_build_profile.rs:498",
  "site": "src/ast.rs:439",
  "site_fingerprint": "d8760e09b9a17349"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callers",
  "seed_def": "src/go_build_profile.rs:498",
  "site": "src/queries.rs:315",
  "site_fingerprint": "db7ca80582fe1a87"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/go_build_profile.rs:498",
  "site": "src/go_build_profile.rs:499",
  "site_fingerprint": "5ca8559ddaa517ac",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/go_type_alias/canon.rs:392",
  "site": "src/go_type_alias/canon.rs:400",
  "site_fingerprint": "61cdc8b89714c8e8"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/go_type_alias/canon.rs:392",
  "site": "src/go_type_alias/canon.rs:401",
  "site_fingerprint": "a931368d46e7a085",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/go_type_alias/canon.rs:392",
  "site": "src/go_type_alias/canon.rs:406",
  "site_fingerprint": "bed3b9a0e6e4d8d5",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/go_type_alias/canon.rs:392",
  "site": "src/go_type_alias/canon.rs:411",
  "site_fingerprint": "16d4a32dcea9b80b",
  "tier": "candidate"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/main.rs:1019",
  "site": "src/main.rs:1020",
  "site_fingerprint": "0d68f9a5d956eb01"
 },
 {
  "corpus": "prism",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/main.rs:1019",
  "site": "src/main.rs:1021",
  "site_fingerprint": "ab211e34e9133547"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/lang/bash/bash_test.rs:209",
  "site": "tests/lang/bash/bash_test.rs:226",
  "site_fingerprint": "c35c54daae30a929"
 },
 {
  "corpus": "prism",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/lang/java/algo_test.rs:371",
  "site": "tests/lang/java/algo_test.rs:418",
  "site_fingerprint": "89d3c387b0d49fa9"
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
   "tests/navigation/module_graph_test.rs:152",
   "tests/navigation/module_graph_test.rs:142",
   "src/main.rs:193",
   "tests/navigation/cache_test.rs:331",
   "tests/navigation/cache_test.rs:345",
   "tests/navigation/module_graph_test.rs:93",
   "tests/navigation/module_graph_test.rs:169",
   "tests/navigation/module_graph_test.rs:219",
   "tests/navigation/module_graph_test.rs:236",
   "tests/navigation/module_graph_test.rs:253",
   "tests/navigation/module_graph_test.rs:281",
   "tests/navigation/module_graph_test.rs:307",
   "tests/navigation/module_graph_test.rs:332",
   "tests/navigation/module_graph_test.rs:353",
   "tests/name_resolution/build_wiring_test.rs:642",
   "tests/name_resolution/build_wiring_test.rs:643",
   "tests/navigation/scoped_calls_test.rs:223"
  ],
  "outcome": "missing",
  "prism_only": [
   "src/mcp/tools.rs:230"
  ],
  "prism_sites": [
   "src/main.rs:193",
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
   "tests/navigation/scoped_calls_test.rs:223"
  ],
  "raw": {
   "fn": 0,
   "fp": 1,
   "precision": [
    0.9444444444444444,
    0.7424269900391908,
    0.990124809131223
   ],
   "recall": [
    1.0,
    0.8156818626193445,
    1.0
   ],
   "tp": 17
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
   "tests/integration/resolution_test.rs:5299",
   "tests/integration/resolution_test.rs:5368"
  ],
  "oracle_sites": [
   "src/cpg/tests.rs:2542",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/navigation/property_access_test.rs:19",
   "tests/navigation/loader_test.rs:13",
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
   "examples/parameter_site_census.rs:219",
   "tests/navigation/loader_test.rs:32",
   "src/executable_owner/acquisition.rs:127",
   "tests/navigation/glob_stats_scope_test.rs:46",
   "src/navigation/inventory.rs:16",
   "examples/project_membership_census.rs:63",
   "src/cpg_cache.rs:766",
   "tests/navigation/callers_test.rs:11",
   "tests/navigation/callers_test.rs:134",
   "tests/navigation/callers_test.rs:166",
   "tests/reasoning/taint_reaches_test.rs:25",
   "tests/navigation/symbol_spans_test.rs:21",
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
   "tests/navigation/onboarding_test.rs:7",
   "examples/dfg_census.rs:359",
   "tests/ast/cpg_cache_test.rs:559",
   "src/repo_loader.rs:993",
   "src/repo_loader.rs:1030",
   "src/repo_loader.rs:1049",
   "src/repo_loader.rs:1076",
   "src/repo_loader.rs:1114",
   "src/repo_loader.rs:1155",
   "src/repo_loader.rs:1193",
   "src/repo_loader.rs:1235",
   "src/repo_loader.rs:1293",
   "src/repo_loader.rs:1321",
   "src/repo_loader.rs:1380",
   "src/repo_loader.rs:1447",
   "src/repo_loader.rs:1483",
   "src/repo_loader.rs:1515",
   "src/repo_loader.rs:1540",
   "tests/navigation/func_value_test.rs:19",
   "src/api/nav.rs:26",
   "tests/cli/nav_compat_test.rs:819",
   "tests/navigation/callees_test.rs:11",
   "src/navigation/mod.rs:569",
   "src/navigation/mod.rs:584",
   "src/navigation/mod.rs:607",
   "src/navigation/mod.rs:651",
   "tests/lang/go/receiver_origin_prereq_test.rs:36",
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
   "tests/navigation/seed_test.rs:11",
   "tests/navigation/nodes_at_test.rs:17",
   "src/executable_owner/integration.rs:156",
   "tests/navigation/index_test.rs:13",
   "tests/navigation/index_test.rs:31",
   "tests/navigation/framework_entry_test.rs:20",
   "tests/lang/go/test_support.rs:23",
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
   "src/call_graph.rs:7063",
   "src/call_graph.rs:7076",
   "tests/navigation/scoped_calls_test.rs:20",
   "tests/cli/nav_compat_test.rs:329",
   "tests/navigation/module_graph_test.rs:17",
   "tests/integration/dfg_label_parity_test.rs:30",
   "src/executable_owner/admission.rs:120",
   "tests/navigation/go_concrete_cache_test.rs:90",
   "tests/navigation/go_concrete_cache_test.rs:188",
   "tests/navigation/go_concrete_cache_test.rs:387",
   "tests/integration/resolution_test.rs:5299",
   "tests/integration/resolution_test.rs:5342",
   "tests/integration/resolution_test.rs:5368",
   "tests/integration/resolution_test.rs:5428",
   "tests/navigation/ego_test.rs:9",
   "src/navigation/call_edge_cache.rs:736"
  ],
  "outcome": "missing",
  "prism_only": [
   "src/executable_owner/audit_tests.rs:310",
   "src/executable_owner/audit_tests.rs:322",
   "src/executable_owner/audit_tests.rs:333",
   "src/executable_owner/integration/tests.rs:22",
   "src/executable_owner/integration/tests.rs:79",
   "src/mcp/freshness.rs:401",
   "src/mcp/session.rs:359",
   "src/mcp/session.rs:374",
   "src/mcp/session.rs:400"
  ],
  "prism_sites": [
   "src/api/nav.rs:26",
   "src/call_graph.rs:7063",
   "src/call_graph.rs:7076",
   "src/cpg/tests.rs:2542",
   "src/cpg_cache.rs:766",
   "src/executable_owner/acquisition.rs:127",
   "src/executable_owner/admission.rs:120",
   "src/executable_owner/audit_tests.rs:310",
   "src/executable_owner/audit_tests.rs:322",
   "src/executable_owner/audit_tests.rs:333",
   "src/executable_owner/integration.rs:156",
   "src/executable_owner/integration/tests.rs:22",
   "src/executable_owner/integration/tests.rs:79",
   "src/mcp/freshness.rs:401",
   "src/mcp/session.rs:359",
   "src/mcp/session.rs:374",
   "src/mcp/session.rs:400",
   "src/navigation/call_edge_cache.rs:736",
   "src/navigation/inventory.rs:16",
   "src/navigation/mod.rs:569",
   "src/navigation/mod.rs:584",
   "src/navigation/mod.rs:607",
   "src/navigation/mod.rs:651",
   "src/repo_loader.rs:993",
   "src/repo_loader.rs:1030",
   "src/repo_loader.rs:1049",
   "src/repo_loader.rs:1076",
   "src/repo_loader.rs:1114",
   "src/repo_loader.rs:1155",
   "src/repo_loader.rs:1193",
   "src/repo_loader.rs:1235",
   "src/repo_loader.rs:1293",
   "src/repo_loader.rs:1321",
   "src/repo_loader.rs:1380",
   "src/repo_loader.rs:1447",
   "src/repo_loader.rs:1483",
   "src/repo_loader.rs:1515",
   "src/repo_loader.rs:1540",
   "tests/ast/cpg_cache_test.rs:559",
   "tests/cli/nav_compat_test.rs:329",
   "tests/cli/nav_compat_test.rs:819",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/integration/dfg_label_parity_test.rs:30",
   "tests/integration/resolution_test.rs:5342",
   "tests/integration/resolution_test.rs:5428",
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
   "tests/reasoning/taint_reaches_test.rs:25"
  ],
  "raw": {
   "fn": 5,
   "fp": 9,
   "precision": [
    0.9274193548387096,
    0.8678041528026484,
    0.9613478369163021
   ],
   "recall": [
    0.9583333333333334,
    0.9061591471148217,
    0.9820732832333764
   ],
   "tp": 115
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
