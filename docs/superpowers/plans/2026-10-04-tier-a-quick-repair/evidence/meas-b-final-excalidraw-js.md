# Tier-A run — excalidraw-js (meas-b-final)

- corpus: `excalidraw-js` @ `0642e72cfa2d`
- prism: `4e592daa7858` · oracle: tsserver 6.0.3 · seed: 42 · harness: `4e592daa7858`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'m1_oracle_inventory': 1.295, 'm2': 45.141, 'm3': 0.0, 'oracle_start': 0.084, 'total': 49.052}
- invalid reasons: []
- oracle configuration: {'cargo_features': None}

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–0.56] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 3/0/0 | 0 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0/0/0 | 2 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-name | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 3/0/0 | 0 | 0 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–0.66] | 0/0/2 | 0 | 0 | 0 |

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-name | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 4/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 0.25 [0.05–0.70] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.33 [0.06–0.79] | 1/0/0 | 3 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-name | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 4/0/0 | 0 | 0 | 0 |
| Q-scoped | 1.00 [0.21–1.00] | 0.25 [0.05–0.70] | 1/0/3 | 0 | 0 | 0 |
| U-method | 0.00 [0.00–1.00] | 0.00 [0.00–1.00] | 0/0/0 | 0 | 0 | 0 |

## Edge-weighted M2 summary

```json
{
 "callees": {
  "exact_tier": {
   "fn": 2,
   "fp": 0,
   "precision": [
    1.0,
    0.43850296436068037,
    1.0
   ],
   "recall": [
    0.6,
    0.23072427940911855,
    0.8823792268590028
   ],
   "tp": 3
  },
  "raw": {
   "fn": 2,
   "fp": 0,
   "precision": [
    1.0,
    0.43850296436068037,
    1.0
   ],
   "recall": [
    0.6,
    0.23072427940911855,
    0.8823792268590028
   ],
   "tp": 3
  }
 },
 "callers": {
  "exact_tier": {
   "fn": 3,
   "fp": 0,
   "precision": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "recall": [
    0.625,
    0.30574239265377096,
    0.8631557152608771
   ],
   "tp": 5
  },
  "raw": {
   "fn": 3,
   "fp": 0,
   "precision": [
    1.0,
    0.5655175313406071,
    1.0
   ],
   "recall": [
    0.625,
    0.30574239265377096,
    0.8631557152608771
   ],
   "tp": 5
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
 "anon_prism": 29,
 "matched": 42,
 "prism_extra": 11,
 "prism_missing": 0,
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
  "corpus": "excalidraw-js",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "scripts/build-version.js:34",
  "site": "scripts/build-version.js:41",
  "site_fingerprint": "4213e1c70ad7727d"
 },
 {
  "corpus": "excalidraw-js",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "scripts/build-version.js:34",
  "site": "scripts/build-version.js:54",
  "site_fingerprint": "a8383c28ed1ba524"
 },
 {
  "corpus": "excalidraw-js",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "scripts/buildPackage.js:22",
  "site": "scripts/buildPackage.js:49",
  "site_fingerprint": "b0d7f9451ace5610"
 },
 {
  "corpus": "excalidraw-js",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "scripts/woff2/woff2-esbuild-plugins.js:23",
  "site": "scripts/woff2/woff2-esbuild-plugins.js:229",
  "site_fingerprint": "ca6196ebbed25521"
 },
 {
  "corpus": "excalidraw-js",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "scripts/woff2/woff2-esbuild-plugins.js:23",
  "site": "scripts/woff2/woff2-esbuild-plugins.js:230",
  "site_fingerprint": "b936fdaca9364e2c"
 }
]
```
