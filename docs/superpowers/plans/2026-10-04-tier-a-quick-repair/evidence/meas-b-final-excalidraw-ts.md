# Tier-A run — excalidraw-ts (meas-b-final)

- corpus: `excalidraw-ts` @ `0642e72cfa2d`
- prism: `4e592daa7858` · oracle: tsserver 6.0.3 · seed: 42 · harness: `4e592daa7858`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'m1_oracle_inventory': 4.452, 'm2': 60.445, 'm3': 0.023, 'oracle_start': 0.152, 'total': 67.867}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.43 [0.16–0.75] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 3/0/0 | 4 | 0 |
| C-name | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 1.00 [0.21–1.00] | 1/0/0 | 0 | 0 |
| U-method | 0.50 [0.24–0.76] | 0.83 [0.44–0.97] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 5/0/0 | 6 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 2/0/1 | 5 | 1 | 4 |
| C-name | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.57–1.00] | 0.83 [0.44–0.97] | 5/0/1 | 7 | 2 | 5 |

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.50 [0.09–0.91] | 0.11 [0.02–0.43] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.50 [0.09–0.91] | 0.14 [0.03–0.51] | 1/0/0 | 9 | 0 |
| C-name | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0/0/0 | 1 | 0 |
| Q-scoped | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 |
| U-method | 0.67 [0.35–0.88] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 6/0/0 | 3 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.30] | 0/0/9 | 2 | 1 | 1 |
| C-name | 0.00 [0.00–0.79] | 0.00 [0.00–1.00] | 0/1/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.44–1.00] | 0.50 [0.19–0.81] | 3/0/3 | 6 | 3 | 3 |

## Edge-weighted M2 summary

```json
{
 "callees": {
  "exact_tier": {
   "fn": 2,
   "fp": 0,
   "precision": [
    1.0,
    0.7224671969739421,
    0.9999999999999999
   ],
   "recall": [
    0.8333333333333334,
    0.5519691353881888,
    0.9530348582430792
   ],
   "tp": 10
  },
  "raw": {
   "fn": 1,
   "fp": 9,
   "precision": [
    0.55,
    0.3420853410191274,
    0.7418021429623598
   ],
   "recall": [
    0.9166666666666666,
    0.6461200862787544,
    0.9851349057603305
   ],
   "tp": 11
  }
 },
 "callers": {
  "exact_tier": {
   "fn": 12,
   "fp": 1,
   "precision": [
    0.8333333333333334,
    0.4364971749742326,
    0.969946630588185
   ],
   "recall": [
    0.29411764705882354,
    0.1327998952626943,
    0.5313311024822815
   ],
   "tp": 5
  },
  "raw": {
   "fn": 8,
   "fp": 5,
   "precision": [
    0.6428571428571429,
    0.38764422924295516,
    0.8365526834990504
   ],
   "recall": [
    0.5294117647058824,
    0.3096323491502231,
    0.7383489368862088
   ],
   "tp": 9
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
 "anon_prism": 4459,
 "matched": 2855,
 "prism_extra": 1207,
 "prism_missing": 54,
 "snapshot_prism_missing": 54
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
   "probe": "callers:packages/excalidraw/subset/woff2/woff2-bindings.ts:103",
   "site": "packages/excalidraw/subset/woff2/woff2-bindings.ts:917",
   "verdict": "confirmed_fp"
  }
 ],
 "counts": {
  "alias_site": 1,
  "ambiguous": 3,
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
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:7927",
  "site_fingerprint": "f27182cdb00530f7"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:8059",
  "site_fingerprint": "526f540a33fc10ae"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/components/App.tsx:8082",
  "site_fingerprint": "dac8824f2d8571aa"
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
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/eraser/index.ts:72",
  "site_fingerprint": "0dba49264a334ea0"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/laserTrails.ts:46",
  "site_fingerprint": "5f3afae386bb61f2"
 },
 {
  "corpus": "excalidraw-ts",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "packages/excalidraw/lasso/index.ts:71",
  "site": "packages/excalidraw/laserTrails.ts:115",
  "site_fingerprint": "0246fd378ffb7f7d"
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
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "packages/excalidraw/subset/woff2/woff2-bindings.ts:886",
  "site": "packages/excalidraw/subset/woff2/woff2-bindings.ts:887",
  "site_fingerprint": "1a5510209f4fde84"
 }
]
```
