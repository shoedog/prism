# Tier-A run — excalidraw-ts (2026-10-14)

- corpus: `excalidraw-ts` @ `0642e72cfa2d`
- prism: `d650e01eb9fa` · oracle: tsserver 6.0.3 · seed: 42 · harness: `d650e01eb9fa`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 0.183, 'm1_oracle_inventory': 5.129, 'm2': 107.78, 'm3': 0.007, 'total': 122.008}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}
- oracle retries: 0 · restarts: 0

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–0.66] | 0.00 [0.00–0.56] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0.00 [0.00–0.56] | 0/0/0 | 5 | 0 |
| C-name | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0/0/0 | 1 | 0 |
| Q-scoped | 1.00 [0.34–1.00] | 0.09 [0.03–0.28] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 2/0/0 | 20 | 0 |
| U-free | 1.00 [0.65–1.00] | 0.70 [0.40–0.89] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 0.78 [0.45–0.94] | 7/0/0 | 3 | 0 |
| U-method | 0.67 [0.35–0.88] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 6/0/0 | 3 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.56] | 0/0/3 | 2 | 0 | 2 |
| C-name | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0/1/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.34–1.00] | 0.09 [0.03–0.28] | 2/0/20 | 0 | 0 | 0 |
| U-free | 1.00 [0.65–1.00] | 0.70 [0.40–0.89] | 7/0/3 | 0 | 0 | 0 |
| U-method | 1.00 [0.44–1.00] | 0.50 [0.19–0.81] | 3/0/3 | 6 | 3 | 3 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.43 [0.16–0.75] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 3/0/0 | 4 | 0 |
| C-name | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 |
| Q-scoped | 0.56 [0.27–0.81] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 5/0/0 | 4 | 0 |
| U-free | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| U-method | 0.50 [0.24–0.76] | 0.83 [0.44–0.97] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 5/0/0 | 6 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 2/0/1 | 5 | 1 | 4 |
| C-name | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 | 0 |
| Q-scoped | 0.56 [0.27–0.81] | 1.00 [0.57–1.00] | 5/4/0 | 0 | 0 | 0 |
| U-free | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.57–1.00] | 0.83 [0.44–0.97] | 5/0/1 | 7 | 2 | 5 |

## Sample-pooled M2 summary (95% Wilson intervals)

```json
{
 "callees": {
  "exact_tier": {
   "fn": 2,
   "fp": 4,
   "precision": [
    0.7777777777777778,
    0.5478541549399641,
    0.9099907195415445
   ],
   "recall": [
    0.875,
    0.6397717251782153,
    0.9650225125737311
   ],
   "tp": 14
  },
  "raw": {
   "fn": 1,
   "fp": 13,
   "precision": [
    0.5357142857142857,
    0.35812689108225854,
    0.704684301314922
   ],
   "recall": [
    0.9375,
    0.716712622018518,
    0.9888806553587528
   ],
   "tp": 15
  }
 },
 "callers": {
  "exact_tier": {
   "fn": 29,
   "fp": 1,
   "precision": [
    0.9230769230769231,
    0.666860488286395,
    0.986289578909774
   ],
   "recall": [
    0.2926829268292683,
    0.176091875531421,
    0.44479467847204524
   ],
   "tp": 12
  },
  "raw": {
   "fn": 26,
   "fp": 6,
   "precision": [
    0.7142857142857143,
    0.5004361911516968,
    0.8618613712823846
   ],
   "recall": [
    0.36585365853658536,
    0.23589830933578665,
    0.5187929903135151
   ],
   "tp": 15
  }
 }
}
```

## Oracle and SUT failures (excluded from accuracy, retained for validity)

```json
{}
```

## Incoming sites excluded by static definition binding

```json
[
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 120,
     "offset": 4
    },
    "contextStart": {
     "line": 114,
     "offset": 3
    },
    "end": {
     "line": 114,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/animatedTrail.ts",
    "start": {
     "line": 114,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/laserTrails.ts",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 46,
    "offset": 30
   },
   "start": {
    "line": 46,
    "offset": 21
   }
  }
 },
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 120,
     "offset": 4
    },
    "contextStart": {
     "line": 114,
     "offset": 3
    },
    "end": {
     "line": 114,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/animatedTrail.ts",
    "start": {
     "line": 114,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/laserTrails.ts",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 115,
    "offset": 26
   },
   "start": {
    "line": 115,
    "offset": 17
   }
  }
 },
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 120,
     "offset": 4
    },
    "contextStart": {
     "line": 114,
     "offset": 3
    },
    "end": {
     "line": 114,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/animatedTrail.ts",
    "start": {
     "line": 114,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/lasso/index.ts",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 75,
    "offset": 20
   },
   "start": {
    "line": 75,
    "offset": 11
   }
  }
 },
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 120,
     "offset": 4
    },
    "contextStart": {
     "line": 114,
     "offset": 3
    },
    "end": {
     "line": 114,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/animatedTrail.ts",
    "start": {
     "line": 114,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/eraser/index.ts",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 72,
    "offset": 20
   },
   "start": {
    "line": 72,
    "offset": 11
   }
  }
 },
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 47,
     "offset": 4
    },
    "contextStart": {
     "line": 45,
     "offset": 3
    },
    "end": {
     "line": 45,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/laserTrails.ts",
    "start": {
     "line": 45,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/components/App.tsx",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 8059,
    "offset": 33
   },
   "start": {
    "line": 8059,
    "offset": 24
   }
  }
 },
 {
  "definitions": [
   {
    "contextEnd": {
     "line": 74,
     "offset": 4
    },
    "contextStart": {
     "line": 70,
     "offset": 3
    },
    "end": {
     "line": 70,
     "offset": 12
    },
    "file": "/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source/packages/excalidraw/eraser/index.ts",
    "start": {
     "line": 70,
     "offset": 3
    }
   }
  ],
  "file": "packages/excalidraw/components/App.tsx",
  "seed": "packages/excalidraw/lasso/index.ts:71",
  "span": {
   "end": {
    "line": 8082,
    "offset": 33
   },
   "start": {
    "line": 8082,
    "offset": 24
   }
  }
 }
]
```

## Callable population and hierarchy coverage

```json
{
 "hierarchy_named": 2908,
 "outside_by_kind": {
  "arrow_function": 1011,
  "function_declaration": 8,
  "function_expression": 124,
  "method_definition": 64
 },
 "policy": "native hierarchy inventory plus independent property/getter site sampling",
 "prism_named": 4062,
 "prism_outside_hierarchy": 1207,
 "syntax_callable_by_shape": {
  "function": 295,
  "getter": 10,
  "method": 470,
  "property_callable": 661,
  "variable_callable": 1843
 }
}
```

## Member calls and getter accesses: definition-at-site sample

```json
{
 "policy": "static definition binding; getter accesses separate from calls",
 "shapes": {
  "getter": {
   "candidate_sites": 2282,
   "declarations": 10,
   "detection_recall": null,
   "exclusions": {
    "nonconcrete_or_unsupported_definition": 3
   },
   "found": 0,
   "missing": 0,
   "rows": [
    {
     "definitions": [
      {
       "end": {
        "line": 1193,
        "offset": 20
       },
       "file": null,
       "start": {
        "line": 1193,
        "offset": 14
       }
      }
     ],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 23,
      "file": "packages/element/src/transform.ts",
      "kind": "access",
      "line": 799,
      "name": "length"
     }
    },
    {
     "definitions": [
      {
       "end": {
        "line": 1193,
        "offset": 20
       },
       "file": null,
       "start": {
        "line": 1193,
        "offset": 14
       }
      }
     ],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 66,
      "file": "packages/element/src/binding.ts",
      "kind": "access",
      "line": 1185,
      "name": "length"
     }
    },
    {
     "definitions": [],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 28,
      "file": "packages/excalidraw/components/SearchMenu.tsx",
      "kind": "access",
      "line": 142,
      "name": "length"
     }
    }
   ],
   "sampled_sites": 3
  },
  "property_callable": {
   "candidate_sites": 6118,
   "declarations": 661,
   "detection_recall": [
    0.0,
    0.0,
    0.6576197760453506
   ],
   "exclusions": {
    "nonconcrete_or_unsupported_definition": 1
   },
   "found": 0,
   "missing": 2,
   "rows": [
    {
     "definitions": [
      {
       "end": {
        "line": 62,
        "offset": 21
       },
       "file": "packages/excalidraw/tests/helpers/api.ts",
       "start": {
        "line": 62,
        "offset": 10
       }
      },
      {
       "end": {
        "line": 4620,
        "offset": 6
       },
       "file": "packages/excalidraw/components/App.tsx",
       "start": {
        "line": 4573,
        "offset": 5
       }
      }
     ],
     "outcome": "missing",
     "site": {
      "character": 12,
      "file": "packages/excalidraw/tests/history.test.tsx",
      "kind": "call",
      "line": 4457,
      "name": "updateScene"
     }
    },
    {
     "definitions": [
      {
       "end": {
        "line": 42,
        "offset": 8
       },
       "file": null,
       "start": {
        "line": 42,
        "offset": 5
       }
      }
     ],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 28,
      "file": "packages/element/src/transform.ts",
      "kind": "call",
      "line": 491,
      "name": "set"
     }
    },
    {
     "definitions": [
      {
       "end": {
        "line": 355,
        "offset": 4
       },
       "file": "excalidraw-app/collab/Collab.tsx",
       "start": {
        "line": 315,
        "offset": 30
       }
      }
     ],
     "outcome": "missing",
     "site": {
      "character": 13,
      "file": "excalidraw-app/collab/Collab.tsx",
      "kind": "call",
      "line": 977,
      "name": "saveCollabRoomToFirebase"
     }
    }
   ],
   "sampled_sites": 3
  }
 }
}
```

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 4459,
 "matched": 2855,
 "prism_extra": 1207,
 "prism_missing": 53,
 "shared_oracle_selection_lines": 0,
 "shared_sut_start_lines": 10,
 "snapshot_prism_missing": 53
}
```

## M3 spot-check

```json
{
 "cap": 10,
 "checked": [
  {
   "probe": "callers:excalidraw-app/data/LocalData.ts:171",
   "site": "excalidraw-app/data/FileManager.ts:158",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:packages/common/src/appEventBus.ts:101",
   "site": "excalidraw-app/collab/Portal.tsx:45",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:packages/common/src/appEventBus.ts:101",
   "site": "excalidraw-app/collab/Portal.tsx:95",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:packages/common/src/appEventBus.ts:101",
   "site": "excalidraw-app/collab/Portal.tsx:252",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:packages/excalidraw/lasso/index.ts:71",
   "site": "packages/excalidraw/lasso/index.ts:75",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:packages/excalidraw/subset/woff2/woff2-bindings.ts:103",
   "site": "packages/excalidraw/subset/woff2/woff2-bindings.ts:917",
   "verdict": "confirmed_fp"
  }
 ],
 "counts": {
  "alias_site": 1,
  "ambiguous": 4,
  "confirmed_fp": 1,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "excalidraw-app/data/LocalData.ts:171",
  "site": "excalidraw-app/data/FileManager.ts:158",
  "site_fingerprint": "f9d3bc37b093d296",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "excalidraw-app/collab/Portal.tsx:45",
  "site_fingerprint": "a8a623d8b576cda0",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "excalidraw-app/collab/Portal.tsx:95",
  "site_fingerprint": "d98e4f7a0301eef7",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "excalidraw-app/collab/Portal.tsx:252",
  "site_fingerprint": "513203e511267bb3",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/common/src/utils.ts:1208",
  "site": "packages/excalidraw/data/library.ts:851",
  "site_fingerprint": "daa40ca4e7059164"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/common/src/utils.ts:1208",
  "site": "packages/excalidraw/data/library.ts:899",
  "site_fingerprint": "760a169307ab5ec5"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/src/groups.ts:303",
  "site": "packages/excalidraw/actions/actionGroup.tsx:159",
  "site_fingerprint": "88d442da3f26c2b7"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:196",
  "site_fingerprint": "9832f308675a142e"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:204",
  "site_fingerprint": "9976871c19bf9778"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:219",
  "site_fingerprint": "fc97e8f13a317a3f"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:366",
  "site_fingerprint": "1138883827167874"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:379",
  "site_fingerprint": "9ee93d824c0e24f6"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:480",
  "site_fingerprint": "25cebde8c34f750d"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:532",
  "site_fingerprint": "def3d64f3073c7c2"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:542",
  "site_fingerprint": "f5922834e8209264"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:599",
  "site_fingerprint": "60375a2493502605"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:640",
  "site_fingerprint": "7c36865eea6c8702"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:675",
  "site_fingerprint": "cd17b3f19b0ea917"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:731",
  "site_fingerprint": "edbc810d3247c850"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:741",
  "site_fingerprint": "f5922834e8209264"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:796",
  "site_fingerprint": "60375a2493502605"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:844",
  "site_fingerprint": "7c36865eea6c8702"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:873",
  "site_fingerprint": "606ed3ce1648f91a"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:937",
  "site_fingerprint": "7f1d6c791d6edc15"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:1059",
  "site_fingerprint": "edbc810d3247c850"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/element/tests/linearElementEditor.test.tsx:143",
  "site": "packages/element/tests/linearElementEditor.test.tsx:1266",
  "site_fingerprint": "4856d5ee63f11508"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/components/App.tsx:8113",
  "site": "packages/excalidraw/components/App.tsx:2406",
  "site_fingerprint": "284094ff6babe3f9"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/lasso/index.ts:75",
  "site_fingerprint": "462f4a68d52c73a3",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:7927",
  "site_fingerprint": "f27182cdb00530f7"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:8740",
  "site_fingerprint": "0de6ac628e38603a"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:10325",
  "site_fingerprint": "7072a75c1f5666e3"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/subset/woff2/woff2-bindings.ts:103",
  "site": "packages/excalidraw/subset/woff2/woff2-bindings.ts:917",
  "site_fingerprint": "ccff2d972f981cba"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "excalidraw-app/data/LocalData.ts:171",
  "site": "excalidraw-app/data/LocalData.ts:186",
  "site_fingerprint": "c5ece834276e4787",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "excalidraw-app/data/LocalData.ts:171",
  "site": "excalidraw-app/data/LocalData.ts:187",
  "site_fingerprint": "b33895579cdaf60c",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "excalidraw-app/data/LocalData.ts:171",
  "site": "excalidraw-app/data/LocalData.ts:189",
  "site_fingerprint": "94a0c95a11a925d1",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "packages/common/src/appEventBus.ts:106",
  "site_fingerprint": "542a1bca17335a5a",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "packages/common/src/appEventBus.ts:109",
  "site_fingerprint": "156964a7c2ea328a",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "packages/common/src/appEventBus.ts:101",
  "site": "packages/common/src/appEventBus.ts:114",
  "site_fingerprint": "120bf58d79f2f99a",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "packages/common/src/versionedSnapshotStore.ts:61",
  "site": "packages/common/src/versionedSnapshotStore.ts:67",
  "site_fingerprint": "095c3132a5570c70",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "packages/element/src/delta.ts:205",
  "site": "packages/element/src/delta.ts:246",
  "site_fingerprint": "48f410b0a343f446",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "r6_multi_owner_candidate",
  "measurement": "callees",
  "seed_def": "packages/element/src/delta.ts:205",
  "site": "packages/element/src/delta.ts:247",
  "site_fingerprint": "8e0af54353803eac",
  "tier": "candidate"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/components/App.tsx:8113",
  "site": "packages/excalidraw/components/App.tsx:8120",
  "site_fingerprint": "f92cc4ca60c080c8"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/components/App.tsx:8113",
  "site": "packages/excalidraw/components/App.tsx:8121",
  "site_fingerprint": "3d79da811e462166"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/components/App.tsx:8113",
  "site": "packages/excalidraw/components/App.tsx:8156",
  "site_fingerprint": "81a8af03f47bf65c"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/components/App.tsx:8113",
  "site": "packages/excalidraw/components/App.tsx:8166",
  "site_fingerprint": "aa9483f5cf27addf"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/subset/woff2/woff2-bindings.ts:883",
  "site": "packages/excalidraw/subset/woff2/woff2-bindings.ts:884",
  "site_fingerprint": "2d2680da71db2335"
 }
]
```
