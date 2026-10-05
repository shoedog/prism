# Tier-A run — prism-quick (meas-b-final)

- corpus: `prism-quick` @ `20c8490591a3`
- prism: `4e592daa7858` · oracle: rust-analyzer 1.94.0 (4a4ef493 2026-03-02) · seed: 42 · harness: `4e592daa7858`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'m1_oracle_inventory': 5.596, 'm2': 20.343, 'm3': 0.001, 'matrix': 7.998, 'oracle_start': 7.631, 'pinned': 2.515, 'total': 45.152}
- invalid reasons: []
- oracle configuration: {'cargo_features': ['mcp']}

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.75 [0.30–0.95] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 3/0/0 | 1 | 0 |
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.61–1.00] | 0.67 [0.35–0.88] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 6/0/0 | 3 | 0 |
| U-free | 0.71 [0.53–0.85] | 1.00 [0.84–1.00] | 1.00 [0.84–1.00] | 1.00 [0.84–1.00] | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 20/0/0 | 8 | 0 |
| U-method | 0.83 [0.44–0.97] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 5/0/0 | 1 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 3/0/0 | 1 | 0 | 1 |
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.61–1.00] | 0.67 [0.35–0.88] | 6/0/3 | 1 | 1 | 0 |
| U-free | 0.71 [0.53–0.85] | 1.00 [0.84–1.00] | 20/8/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 5/0/0 | 1 | 0 | 1 |

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 0.09 [0.02–0.38] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.09 [0.02–0.38] | 1/0/0 | 10 | 0 |
| C-name | 1.00 [0.92–1.00] | 0.98 [0.88–1.00] | 1.00 [0.92–1.00] | 1.00 [0.92–1.00] | 1.00 [0.90–1.00] | 0.97 [0.85–0.99] | 42/0/0 | 1 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1/0/0 | 0 | 0 |
| U-free | 1.00 [0.72–1.00] | 0.91 [0.62–0.98] | 1.00 [0.72–1.00] | 1.00 [0.72–1.00] | 1.00 [0.44–1.00] | 0.75 [0.30–0.95] | 10/0/0 | 1 | 0 |
| U-method | 1.00 [0.81–1.00] | 0.89 [0.67–0.97] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.78–1.00] | 0.93 [0.70–0.99] | 16/0/0 | 2 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 0.09 [0.02–0.38] | 1/0/10 | 0 | 0 | 0 |
| C-name | 1.00 [0.92–1.00] | 0.98 [0.88–1.00] | 42/0/1 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1/0/0 | 0 | 0 | 0 |
| U-free | 1.00 [0.72–1.00] | 0.91 [0.62–0.98] | 10/0/1 | 0 | 0 | 0 |
| U-method | 1.00 [0.34–1.00] | 0.11 [0.03–0.33] | 2/0/16 | 14 | 14 | 0 |

## Edge-weighted M2 summary

```json
{
 "callees": {
  "exact_tier": {
   "fn": 3,
   "fp": 8,
   "precision": [
    0.8095238095238095,
    0.6669922586404059,
    0.9001799176929347
   ],
   "recall": [
    0.918918918918919,
    0.7869932051629003,
    0.972039426117491
   ],
   "tp": 34
  },
  "raw": {
   "fn": 3,
   "fp": 10,
   "precision": [
    0.7727272727272727,
    0.6300764181310986,
    0.8715805246379061
   ],
   "recall": [
    0.918918918918919,
    0.7869932051629003,
    0.972039426117491
   ],
   "tp": 34
  }
 },
 "callers": {
  "exact_tier": {
   "fn": 28,
   "fp": 0,
   "precision": [
    1.0,
    0.9358060623335392,
    1.0
   ],
   "recall": [
    0.6666666666666666,
    0.5605282296760283,
    0.7582278606325084
   ],
   "tp": 56
  },
  "raw": {
   "fn": 14,
   "fp": 0,
   "precision": [
    1.0,
    0.9479769368117258,
    0.9999999999999999
   ],
   "recall": [
    0.8333333333333334,
    0.7394696017084232,
    0.8980425789086501
   ],
   "tp": 70
  }
 }
}
```

## Oracle failures (excluded from accuracy, retained for validity)

```json
{}
```

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 0,
 "matched": 4530,
 "prism_extra": 0,
 "prism_missing": 17,
 "snapshot_prism_missing": 17
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
  "capability": "s1b_jsx_intrinsic_tag_refused",
  "expected": [],
  "expected_resolution_kind": null,
  "forbid_resolution_kind": "free_single",
  "got": [],
  "got_kinds": {},
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
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/ast/cpg_cache_test.rs:604",
  "site_fingerprint": "7ee819c81a50f34b"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/ast/cpg_cache_test.rs:650",
  "site_fingerprint": "fa63983c663a829c"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/ast/cpg_cache_test.rs:701",
  "site_fingerprint": "a9228fdfd9257760"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/integration/call_graph_test.rs:1149",
  "site_fingerprint": "702a5b100351cb26"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/integration/resolution_test.rs:2267",
  "site_fingerprint": "06668ffbc9efdbe5"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/call_graph.rs:1009",
  "site": "tests/name_resolution/build_wiring_test.rs:171",
  "site_fingerprint": "2db8c76430fb6ac0"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/main.rs:24",
  "site": "src/main.rs:72",
  "site_fingerprint": "ffb7857ebf314230"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/name_resolution/binding_lookup.rs:86",
  "site": "src/name_resolution/binding_lookup.rs:59",
  "site_fingerprint": "0921887b31d9b62d"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/navigation/mod.rs:82",
  "site": "src/navigation/queries.rs:335",
  "site_fingerprint": "e6c3103e94ecde7d"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/navigation/mod.rs:82",
  "site": "src/navigation/queries.rs:821",
  "site_fingerprint": "d31fc8775f5dbc97"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/navigation/mod.rs:82",
  "site": "src/navigation/seed.rs:50",
  "site_fingerprint": "d31fc8775f5dbc97"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/navigation/mod.rs:82",
  "site": "tests/navigation/index_test.rs:33",
  "site_fingerprint": "4bf3376e118fb12c"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/type_provider.rs:259",
  "site": "tests/ast/cpp_type_provider_test.rs:525",
  "site_fingerprint": "123483e2ef0706df"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/type_provider.rs:259",
  "site": "tests/ast/cpp_type_provider_test.rs:544",
  "site_fingerprint": "02f0bba3c353c5e3"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/main.rs:1298",
  "site": "src/main.rs:1299",
  "site_fingerprint": "a6c08076b5984625"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/main.rs:1298",
  "site": "src/main.rs:1300",
  "site_fingerprint": "53c29f5ee666721e"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:345",
  "site_fingerprint": "58d40ae92a8a6578"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:368",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:398",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:428",
  "site_fingerprint": "dd833960289f5602"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:441",
  "site_fingerprint": "d2e134d7522d07d0"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "qualified_owner",
  "measurement": "callees",
  "seed_def": "src/main.rs:332",
  "site": "src/main.rs:447",
  "site_fingerprint": "56de19243e1be085"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/navigation/mod.rs:82",
  "site": "src/navigation/mod.rs:83",
  "site_fingerprint": "c14011a6e225d1fc",
  "tier": "candidate"
 },
 {
  "corpus": "prism-quick",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/type_provider.rs:259",
  "site": "src/type_provider.rs:260",
  "site_fingerprint": "e884fef269f9ad6b",
  "tier": "candidate"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/integration/call_graph_test.rs:101",
  "site": "tests/integration/call_graph_test.rs:117",
  "site_fingerprint": "d1f627a43e88d3f6"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/integration/call_graph_test.rs:101",
  "site": "tests/integration/call_graph_test.rs:131",
  "site_fingerprint": "efbfd21aa4bbdca6"
 },
 {
  "corpus": "prism-quick",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/lang/bash/bash_test.rs:526",
  "site": "tests/lang/bash/bash_test.rs:543",
  "site_fingerprint": "c35c54daae30a929"
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
   "src/algorithms/taint.rs:1763",
   "src/algorithms/taint.rs:4420",
   "src/algorithms/taint.rs:4430",
   "src/algorithms/taint.rs:4438",
   "src/algorithms/taint.rs:4450"
  ],
  "outcome": "flip_candidate",
  "prism_only": [
   "src/algorithms/circular_slice.rs:119",
   "src/algorithms/circular_slice.rs:162",
   "src/algorithms/circular_slice.rs:164",
   "src/algorithms/circular_slice.rs:240",
   "src/algorithms/circular_slice.rs:242",
   "src/algorithms/gradient_slice.rs:116",
   "src/cpg/build.rs:522",
   "src/cpg/build.rs:544",
   "src/cpg/cfg_queries.rs:51",
   "src/cpg/query.rs:75",
   "src/cpg/query.rs:156",
   "src/cpg/query.rs:157",
   "src/cpg/query.rs:270",
   "src/cpg/query.rs:480",
   "src/cpg/query.rs:520",
   "src/cpg/query.rs:565",
   "src/cpg/query.rs:615",
   "src/cpg/query.rs:679",
   "src/cpg/query.rs:680",
   "src/cpg/trace.rs:469",
   "src/navigation/queries.rs:889",
   "tests/ast/cpg_test.rs:26",
   "tests/ast/cpg_test.rs:84"
  ],
  "prism_sites": [
   "src/algorithms/taint.rs:1763",
   "src/algorithms/taint.rs:4420",
   "src/algorithms/taint.rs:4430",
   "src/algorithms/taint.rs:4438",
   "src/algorithms/taint.rs:4450",
   "src/algorithms/circular_slice.rs:119",
   "src/algorithms/circular_slice.rs:162",
   "src/algorithms/circular_slice.rs:164",
   "src/algorithms/circular_slice.rs:240",
   "src/algorithms/circular_slice.rs:242",
   "src/algorithms/gradient_slice.rs:116",
   "src/cpg/build.rs:522",
   "src/cpg/build.rs:544",
   "src/cpg/cfg_queries.rs:51",
   "src/cpg/query.rs:75",
   "src/cpg/query.rs:156",
   "src/cpg/query.rs:157",
   "src/cpg/query.rs:270",
   "src/cpg/query.rs:480",
   "src/cpg/query.rs:520",
   "src/cpg/query.rs:565",
   "src/cpg/query.rs:615",
   "src/cpg/query.rs:679",
   "src/cpg/query.rs:680",
   "src/cpg/trace.rs:469",
   "src/navigation/queries.rs:889",
   "tests/ast/cpg_test.rs:26",
   "tests/ast/cpg_test.rs:84"
  ],
  "raw": {
   "fn": 0,
   "fp": 23,
   "precision": [
    0.17857142857142858,
    0.07878501895759982,
    0.3559142494677742
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
   "src/main.rs:440",
   "tests/name_resolution/build_wiring_test.rs:417",
   "tests/name_resolution/build_wiring_test.rs:418",
   "tests/navigation/scoped_calls_test.rs:214",
   "tests/navigation/module_graph_test.rs:152",
   "tests/navigation/module_graph_test.rs:143",
   "src/mcp/tools.rs:162",
   "tests/navigation/module_graph_test.rs:93",
   "tests/navigation/module_graph_test.rs:169",
   "tests/navigation/module_graph_test.rs:213",
   "tests/navigation/module_graph_test.rs:240",
   "tests/navigation/module_graph_test.rs:266",
   "tests/navigation/module_graph_test.rs:291",
   "tests/navigation/module_graph_test.rs:311"
  ],
  "outcome": "missing",
  "prism_only": [],
  "prism_sites": [
   "src/main.rs:440",
   "src/mcp/tools.rs:162",
   "tests/name_resolution/build_wiring_test.rs:417",
   "tests/name_resolution/build_wiring_test.rs:418",
   "tests/navigation/module_graph_test.rs:93",
   "tests/navigation/module_graph_test.rs:143",
   "tests/navigation/module_graph_test.rs:152",
   "tests/navigation/module_graph_test.rs:169",
   "tests/navigation/module_graph_test.rs:213",
   "tests/navigation/module_graph_test.rs:240",
   "tests/navigation/module_graph_test.rs:266",
   "tests/navigation/module_graph_test.rs:291",
   "tests/navigation/module_graph_test.rs:311",
   "tests/navigation/scoped_calls_test.rs:214"
  ],
  "raw": {
   "fn": 0,
   "fp": 0,
   "precision": [
    1.0,
    0.7846891945970195,
    0.9999999999999999
   ],
   "recall": [
    1.0,
    0.7846891945970195,
    0.9999999999999999
   ],
   "tp": 14
  }
 },
 {
  "expected": "oracle_miss_site",
  "id": "load-repo-feature-gated",
  "oracle_miss_site_found": false,
  "oracle_only": [],
  "oracle_sites": [
   "src/navigation/inventory.rs:16",
   "tests/navigation/ego_test.rs:9",
   "tests/navigation/callees_test.rs:11",
   "src/call_graph.rs:2166",
   "src/call_graph.rs:2179",
   "src/repo_loader.rs:553",
   "tests/name_resolution/build_wiring_test.rs:72",
   "tests/name_resolution/build_wiring_test.rs:162",
   "tests/name_resolution/build_wiring_test.rs:308",
   "tests/name_resolution/build_wiring_test.rs:367",
   "tests/name_resolution/build_wiring_test.rs:404",
   "src/main.rs:490",
   "tests/navigation/seed_test.rs:11",
   "tests/navigation/loader_test.rs:31",
   "tests/navigation/nodes_at_test.rs:17",
   "src/mcp/session.rs:28",
   "tests/navigation/loader_test.rs:12",
   "tests/navigation/loader_test.rs:40",
   "tests/navigation/loader_test.rs:58",
   "tests/navigation/index_test.rs:13",
   "tests/navigation/index_test.rs:31",
   "tests/navigation/callers_test.rs:11",
   "tests/navigation/cache_test.rs:34",
   "tests/navigation/cache_test.rs:52",
   "tests/navigation/cache_test.rs:90",
   "tests/navigation/cache_test.rs:92",
   "tests/navigation/cache_test.rs:101",
   "tests/navigation/cache_test.rs:132",
   "tests/navigation/cache_test.rs:133",
   "tests/cli/nav_compat_test.rs:252",
   "tests/reasoning/taint_reaches_test.rs:25",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/ast/cpg_cache_test.rs:121",
   "tests/navigation/scoped_calls_test.rs:19",
   "tests/navigation/module_graph_test.rs:17"
  ],
  "outcome": "missing",
  "prism_only": [],
  "prism_sites": [
   "src/call_graph.rs:2166",
   "src/call_graph.rs:2179",
   "src/main.rs:490",
   "src/mcp/session.rs:28",
   "src/navigation/inventory.rs:16",
   "src/repo_loader.rs:553",
   "tests/ast/cpg_cache_test.rs:121",
   "tests/cli/nav_compat_test.rs:252",
   "tests/infra/parallel_equality_test.rs:18",
   "tests/infra/parallel_equality_test.rs:22",
   "tests/name_resolution/build_wiring_test.rs:72",
   "tests/name_resolution/build_wiring_test.rs:162",
   "tests/name_resolution/build_wiring_test.rs:308",
   "tests/name_resolution/build_wiring_test.rs:367",
   "tests/name_resolution/build_wiring_test.rs:404",
   "tests/navigation/cache_test.rs:34",
   "tests/navigation/cache_test.rs:52",
   "tests/navigation/cache_test.rs:90",
   "tests/navigation/cache_test.rs:92",
   "tests/navigation/cache_test.rs:101",
   "tests/navigation/cache_test.rs:132",
   "tests/navigation/cache_test.rs:133",
   "tests/navigation/callees_test.rs:11",
   "tests/navigation/callers_test.rs:11",
   "tests/navigation/ego_test.rs:9",
   "tests/navigation/index_test.rs:13",
   "tests/navigation/index_test.rs:31",
   "tests/navigation/loader_test.rs:12",
   "tests/navigation/loader_test.rs:31",
   "tests/navigation/loader_test.rs:40",
   "tests/navigation/loader_test.rs:58",
   "tests/navigation/module_graph_test.rs:17",
   "tests/navigation/nodes_at_test.rs:17",
   "tests/navigation/scoped_calls_test.rs:19",
   "tests/navigation/seed_test.rs:11",
   "tests/reasoning/taint_reaches_test.rs:25"
  ],
  "raw": {
   "fn": 0,
   "fp": 0,
   "precision": [
    1.0,
    0.9035813700311206,
    1.0
   ],
   "recall": [
    1.0,
    0.9035813700311206,
    1.0
   ],
   "tp": 36
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
