# Tier-A run — secbench-node (2026-10-14)

- corpus: `secbench-node` @ `secbench-node-r1`
- prism: `d650e01eb9fa` · oracle: tsserver 6.0.3 · seed: 42 · harness: `d650e01eb9fa`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 0.048, 'm1_oracle_inventory': 0.667, 'm2': 3.202, 'm3': 0.0, 'total': 4.532}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}
- oracle retries: 0 · restarts: 0

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| U-free | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 1/0/0 | 2 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.35] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.35] | 0/0/0 | 7 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| U-free | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 1/0/2 | 0 | 0 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.35] | 0/0/7 | 0 | 0 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | caller context P | caller context R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.30] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0/0/0 | 9 | 0 |
| C-name | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 6/0/0 | 0 | 0 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–0.56] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 0/0/0 | 3 | 0 |
| U-free | 0.00 [0.00–1.00] | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.50 [0.09–0.91] | 1.00 [0.21–1.00] | 0/0/0 | 1 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 0/0/0 | 2 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.30] | 0/0/9 | 0 | 0 | 0 |
| C-name | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 6/0/0 | 0 | 0 | 0 |
| Q-scoped | 0.00 [0.00–1.00] | 0.00 [0.00–0.56] | 0/0/3 | 0 | 0 | 0 |
| U-free | 0.00 [0.00–1.00] | 0.00 [0.00–0.79] | 0/0/1 | 0 | 0 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0/0/2 | 0 | 0 | 0 |

## Sample-pooled M2 summary (95% Wilson intervals)

```json
{
 "callees": {
  "exact_tier": {
   "fn": 15,
   "fp": 0,
   "precision": [
    1.0,
    0.6096657083436265,
    1.0
   ],
   "recall": [
    0.2857142857142857,
    0.1381386287176154,
    0.4995638088483033
   ],
   "tp": 6
  },
  "raw": {
   "fn": 15,
   "fp": 0,
   "precision": [
    1.0,
    0.6096657083436265,
    1.0
   ],
   "recall": [
    0.2857142857142857,
    0.1381386287176154,
    0.4995638088483033
   ],
   "tp": 6
  }
 },
 "callers": {
  "exact_tier": {
   "fn": 9,
   "fp": 0,
   "precision": [
    1.0,
    0.2065493117918027,
    1.0
   ],
   "recall": [
    0.1,
    0.017876212896073995,
    0.4041500295247722
   ],
   "tp": 1
  },
  "raw": {
   "fn": 9,
   "fp": 0,
   "precision": [
    1.0,
    0.2065493117918027,
    1.0
   ],
   "recall": [
    0.1,
    0.017876212896073995,
    0.4041500295247722
   ],
   "tp": 1
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
[]
```

## Callable population and hierarchy coverage

```json
{
 "hierarchy_named": 121,
 "outside_by_kind": {
  "arrow_function": 12,
  "function_declaration": 5,
  "function_expression": 208,
  "method_definition": 10
 },
 "policy": "native hierarchy inventory plus independent property/getter site sampling",
 "prism_named": 356,
 "prism_outside_hierarchy": 235,
 "syntax_callable_by_shape": {
  "function": 84,
  "getter": 6,
  "method": 20,
  "property_callable": 98,
  "variable_callable": 11
 }
}
```

## Member calls and getter accesses: definition-at-site sample

```json
{
 "policy": "static definition binding; getter accesses separate from calls",
 "shapes": {
  "getter": {
   "candidate_sites": 37,
   "declarations": 6,
   "detection_recall": null,
   "exclusions": {
    "nonconcrete_or_unsupported_definition": 3
   },
   "found": 0,
   "missing": 0,
   "rows": [
    {
     "definitions": [],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 9,
      "file": "node-http-server_8.1.2/package/example/advanced/basicTemplate.js",
      "kind": "access",
      "line": 44,
      "name": "value"
     }
    },
    {
     "definitions": [],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 40,
      "file": "algoliasearch-helper_3.6.0/package/src/algoliasearch.helper.js",
      "kind": "access",
      "line": 366,
      "name": "value"
     }
    },
    {
     "definitions": [],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 24,
      "file": "node-http-server_8.1.2/package/example/advanced/https-randomCSS.js",
      "kind": "access",
      "line": 41,
      "name": "value"
     }
    }
   ],
   "sampled_sites": 3
  },
  "property_callable": {
   "candidate_sites": 277,
   "declarations": 98,
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
        "line": 1367,
        "offset": 50
       },
       "file": "algoliasearch-helper_3.6.0/package/src/SearchParameters/index.js",
       "start": {
        "line": 1367,
        "offset": 32
       }
      }
     ],
     "outcome": "missing",
     "site": {
      "character": 16,
      "file": "algoliasearch-helper_3.6.0/package/src/SearchParameters/index.js",
      "kind": "call",
      "line": 819,
      "name": "setQueryParameters"
     }
    },
    {
     "definitions": [
      {
       "end": {
        "line": 95,
        "offset": 44
       },
       "file": "algoliasearch-helper_3.6.0/package/src/SearchParameters/RefinementList.js",
       "start": {
        "line": 95,
        "offset": 29
       }
      }
     ],
     "outcome": "missing",
     "site": {
      "character": 37,
      "file": "algoliasearch-helper_3.6.0/package/src/SearchParameters/index.js",
      "kind": "call",
      "line": 386,
      "name": "clearRefinement"
     }
    },
    {
     "definitions": [],
     "outcome": "nonconcrete_or_unsupported_definition",
     "site": {
      "character": 28,
      "file": "algoliasearch-helper_3.6.0/package/src/SearchResults/index.js",
      "kind": "call",
      "line": 645,
      "name": "isDisjunctiveFacet"
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
 "anon_prism": 159,
 "matched": 121,
 "prism_extra": 235,
 "prism_missing": 0,
 "shared_oracle_selection_lines": 0,
 "shared_sut_start_lines": 2,
 "snapshot_prism_missing": 0
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

## Pending triage

```json
[
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:181",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:125",
  "site_fingerprint": "399bd1e28d13bd3c"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchResults/index.js:751",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchResults/index.js:832",
  "site_fingerprint": "e0cdbe8f32a2de53"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:132",
  "site": "find-process_1.4.4/package/lib/find_process.js:134",
  "site_fingerprint": "dc1737590d68d12c"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_pid.js:40",
  "site_fingerprint": "43a3377a46cd3ae7"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_pid.js:78",
  "site_fingerprint": "f227d5dbb0961263"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_pid.js:114",
  "site_fingerprint": "9e573abffef3a118"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_pid.js:150",
  "site_fingerprint": "96daccad563a932c"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_process.js:83",
  "site_fingerprint": "e2f8fd4821c2c801"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "find-process_1.4.4/package/lib/utils.js:37",
  "site": "find-process_1.4.4/package/lib/find_process.js:182",
  "site_fingerprint": "2637bae46210e9f2"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchParameters/index.js:41",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchParameters/index.js:43",
  "site_fingerprint": "d4ee83cbe14d440c"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:67",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:110",
  "site_fingerprint": "a1a67db01e17d8d9"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:67",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:120",
  "site_fingerprint": "871ffaeb29b7776a"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:67",
  "site": "algoliasearch-helper_3.6.0/package/src/SearchResults/generate-hierarchical-tree.js:125",
  "site_fingerprint": "399bd1e28d13bd3c"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find.js:14",
  "site": "find-process_1.4.4/package/lib/find.js:15",
  "site_fingerprint": "0b28d71900646a4f"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find.js:14",
  "site": "find-process_1.4.4/package/lib/find.js:17",
  "site_fingerprint": "8a3e37692ca1517f"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_pid.js:27",
  "site": "find-process_1.4.4/package/lib/find_pid.js:29",
  "site_fingerprint": "e2442d99fafe9f73"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_pid.js:27",
  "site": "find-process_1.4.4/package/lib/find_pid.js:40",
  "site_fingerprint": "43a3377a46cd3ae7"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_pid.js:27",
  "site": "find-process_1.4.4/package/lib/find_pid.js:41",
  "site_fingerprint": "2e6319b2c24ac7a7"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:166",
  "site_fingerprint": "4d160991bbaf2e7d"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:182",
  "site_fingerprint": "2637bae46210e9f2"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:183",
  "site_fingerprint": "594ab8a72fcdbfff"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:187",
  "site_fingerprint": "44b0d23ca6f34a4f"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:195",
  "site_fingerprint": "c3317b33713afc63"
 },
 {
  "corpus": "secbench-node",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "find-process_1.4.4/package/lib/find_process.js:162",
  "site": "find-process_1.4.4/package/lib/find_process.js:202",
  "site_fingerprint": "a16969b20dd29fdf"
 }
]
```
