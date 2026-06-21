# Tier-A run — black-basedpyright (2026-06-20)

- corpus: `black-basedpyright` @ `b74b23013fe6`
- prism: `20c8490591a3` · oracle: basedpyright 1.39.8 · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.171 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.188, 'm1_oracle_inventory': 0.045, 'm2': 77.289, 'm3': 0.0}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-name | 1.00 [0.87–1.00] | 0.89 [0.73–0.96] | 1.00 [0.87–1.00] | 1.00 [0.87–1.00] | 1.00 [0.82–1.00] | 0.85 [0.64–0.95] | 25/0/0 | 3 | 0 |
| Q-scoped | 1.00 [0.91–1.00] | 1.00 [0.91–1.00] | 1.00 [0.91–1.00] | 1.00 [0.91–1.00] | 1.00 [0.78–1.00] | 1.00 [0.78–1.00] | 40/0/0 | 0 | 0 |
| U-free | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 2/0/0 | 1 | 5 |
| U-method | 1.00 [0.44–1.00] | 0.07 [0.02–0.18] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.14 [0.05–0.33] | 3/0/0 | 43 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.78 [0.62–0.88] | 0.67 [0.52–0.79] | 1.00 [0.88–1.00] | 1.00 [0.88–1.00] | 1.00 [0.65–1.00] | 1.00 [0.65–1.00] | 28/0/0 | 22 | 0 |
| C-name | 0.97 [0.86–1.00] | 0.36 [0.27–0.46] | 1.00 [0.90–1.00] | 1.00 [0.90–1.00] | 0.88 [0.53–0.98] | 1.00 [0.65–1.00] | 36/0/0 | 65 | 0 |
| Q-scoped | 0.88 [0.73–0.95] | 0.68 [0.53–0.80] | 1.00 [0.89–1.00] | 1.00 [0.89–1.00] | 1.00 [0.72–1.00] | 1.00 [0.72–1.00] | 30/0/0 | 18 | 0 |
| U-free | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 2/0/0 | 0 | 5 |
| U-method | 1.00 [0.72–1.00] | 0.91 [0.62–0.98] | 1.00 [0.72–1.00] | 1.00 [0.72–1.00] | 0.50 [0.22–0.78] | 1.00 [0.51–1.00] | 10/0/0 | 1 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 6,
 "matched": 604,
 "prism_extra": 2,
 "prism_missing": 0,
 "snapshot_prism_missing": 0
}
```

## M3 spot-check

```json
{
 "cap": 25,
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
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "action/main.py:29",
  "site": "action/main.py:128",
  "site_fingerprint": "babdd433343891ec"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/black/__init__.py:1700",
  "site": "src/black/__init__.py:1712",
  "site_fingerprint": "fe8c65f3b0bb147f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/black/__init__.py:1700",
  "site": "src/black/__main__.py:3",
  "site_fingerprint": "6d3e0fa23d4f54db"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/black/trans.py:376",
  "site": "src/black/trans.py:1598",
  "site_fingerprint": "c5105bb58d58f019"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pgen2/driver.py:299",
  "site": "src/blib2to3/pgen2/driver.py:313",
  "site_fingerprint": "b706be6f5ed842cb"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/brackets.py:292",
  "site_fingerprint": "a1a39a3727abec88"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/brackets.py:293",
  "site_fingerprint": "73ed84cd8f93ef57"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:673",
  "site_fingerprint": "3d09bb2d64119750"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:684",
  "site_fingerprint": "b14b0dabd5674c6c"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:694",
  "site_fingerprint": "c07e0240751fc1cf"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:705",
  "site_fingerprint": "4ae01fdc3d14f53c"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:708",
  "site_fingerprint": "d8b7b76184477e0b"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:714",
  "site_fingerprint": "f63257d465765064"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:715",
  "site_fingerprint": "d098c6a999cd7504"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:717",
  "site_fingerprint": "e2520fa197be3bc7"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:749",
  "site_fingerprint": "862f012e1e87055d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:755",
  "site_fingerprint": "e2b784a0f42e3ba7"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:756",
  "site_fingerprint": "40c285901f21b9a1"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/comments.py:805",
  "site_fingerprint": "6c5d4e78d8711f52"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/linegen.py:177",
  "site_fingerprint": "a123fcaa85b4f936"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/linegen.py:1638",
  "site_fingerprint": "3ae2ca3514214bcf"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/linegen.py:1639",
  "site_fingerprint": "1fc47d90cd11e837"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/linegen.py:1640",
  "site_fingerprint": "2fdf8c72bc397324"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/lines.py:364",
  "site_fingerprint": "3528e54a16a7346f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/lines.py:365",
  "site_fingerprint": "eb596333912baaae"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/lines.py:598",
  "site_fingerprint": "a58753549a8919ac"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/lines.py:604",
  "site_fingerprint": "b1b0dfad643b8fb6"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/lines.py:1348",
  "site_fingerprint": "8e3605a77471be39"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:212",
  "site_fingerprint": "a943e6891e91546a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:441",
  "site_fingerprint": "4e374b8f2c96847f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:469",
  "site_fingerprint": "ee6d6596cb6d5bdd"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:528",
  "site_fingerprint": "f546874ab3ccc4d3"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:586",
  "site_fingerprint": "88f9cb6bc6d42f60"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:1067",
  "site_fingerprint": "89fb44cc1580cb0a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:1111",
  "site_fingerprint": "7292d1729c6b0278"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/nodes.py:1115",
  "site_fingerprint": "10120781a633f3a5"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:252",
  "site_fingerprint": "a5f7ef937ec93e51"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:253",
  "site_fingerprint": "12f4fd41cd5ad7e0"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:275",
  "site_fingerprint": "16fcfc41b98179c8"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:278",
  "site_fingerprint": "3285c0480c8db81a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:285",
  "site_fingerprint": "75e0df284d06b036"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:290",
  "site_fingerprint": "047dd55f5b7178bb"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:296",
  "site_fingerprint": "5ff1ff8a4a248d4d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:297",
  "site_fingerprint": "6df98133d122e068"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/ranges.py:299",
  "site_fingerprint": "e5b1ecd1b563cb25"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/trans.py:1315",
  "site_fingerprint": "fd70b48b65004485"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/blib2to3/pytree.py:197",
  "site": "src/black/trans.py:1325",
  "site_fingerprint": "08d53e3929dfc2a2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:1081",
  "site": "src/black/__init__.py:1100",
  "site_fingerprint": "8f9a6fbdaff7b3c0"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:736",
  "site_fingerprint": "11f2ec506cf9f2c0"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:262",
  "site_fingerprint": "e0877bdcca7a51f9"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:268",
  "site_fingerprint": "45d4805824f7ebb3"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:269",
  "site_fingerprint": "229a1a0e6f182452"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:277",
  "site_fingerprint": "166b1c659764277f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:280",
  "site_fingerprint": "d2db6711847c0350"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:290",
  "site_fingerprint": "a1cb7ac69bffb1ef"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:298",
  "site_fingerprint": "394dd95c9f0c5509"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:306",
  "site_fingerprint": "daf49f26853c7755"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:316",
  "site_fingerprint": "1341a03af5405bea"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:322",
  "site_fingerprint": "acd89879c93ba8cb"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:328",
  "site_fingerprint": "37a2ae73da84b7ec"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:334",
  "site_fingerprint": "63bfa5d411178c55"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:342",
  "site_fingerprint": "f468e98d72847813"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:351",
  "site_fingerprint": "38ce68cede0ba1fb"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:353",
  "site_fingerprint": "46717e7d7dc70cd7"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:362",
  "site_fingerprint": "5090a1844969b6ef"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:371",
  "site_fingerprint": "ccf2e89e047218a6"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:379",
  "site_fingerprint": "82debc193a0c653e"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:384",
  "site_fingerprint": "e2baf4923b0c335f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:397",
  "site_fingerprint": "75331b7515159159"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:406",
  "site_fingerprint": "f421403161cd4730"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:417",
  "site_fingerprint": "8a8db26c2c33d882"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:430",
  "site_fingerprint": "66c56b633fbec459"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:439",
  "site_fingerprint": "34e0812c047604f7"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:450",
  "site_fingerprint": "4f35135fd44d4dbd"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:460",
  "site_fingerprint": "8bb75c229ff55ddd"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:474",
  "site_fingerprint": "8067fcb2159e6e51"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:477",
  "site_fingerprint": "eaed028c4c71528b"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:486",
  "site_fingerprint": "4947283b6c2b848a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:495",
  "site_fingerprint": "7930fe609c76a58e"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:505",
  "site_fingerprint": "03efdb07ccf8236c"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:512",
  "site_fingerprint": "bf0af0049247f333"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:515",
  "site_fingerprint": "a80d9190bb182518"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:521",
  "site_fingerprint": "3ec3f48c0a8c0f2f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:523",
  "site_fingerprint": "0a622dba6d4229fa"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:535",
  "site_fingerprint": "cd7b60b599c8d6fc"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:577",
  "site_fingerprint": "4b46b491d0b6e702"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:586",
  "site_fingerprint": "cdfce5a30b85f165"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:590",
  "site_fingerprint": "b5cb110b15213b78"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:593",
  "site_fingerprint": "5fadc6328a9e4ed2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:596",
  "site_fingerprint": "b735bf7e6b2bfea3"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:601",
  "site_fingerprint": "285365b5de8293b9"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:604",
  "site_fingerprint": "cdfce5a30b85f165"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:619",
  "site_fingerprint": "6123b30bf156457e"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:648",
  "site_fingerprint": "774c3facea77e42d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:651",
  "site_fingerprint": "68ba80f1ea6fcfe9"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:659",
  "site_fingerprint": "9728d4bcbc271933"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:685",
  "site_fingerprint": "a8d350f6735d7b41"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:691",
  "site_fingerprint": "ad9b124202ad89b6"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:698",
  "site_fingerprint": "fa3a8862a73e3f7f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:725",
  "site_fingerprint": "dbb4693411d1eaeb"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:749",
  "site_fingerprint": "2a16b75d79fd1179"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:765",
  "site_fingerprint": "0e3f1bbd071b2cbd"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/__init__.py:544",
  "site": "src/black/__init__.py:766",
  "site_fingerprint": "f4c797cf8c6fc844"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:431",
  "site_fingerprint": "4435fd9336572934"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:434",
  "site_fingerprint": "45ca0919507909e3"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:436",
  "site_fingerprint": "297c9c352a78bb1d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:455",
  "site_fingerprint": "bdf369a73693fb09"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:462",
  "site_fingerprint": "55dffe66bc12216e"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:483",
  "site_fingerprint": "4ff1794b69b78320"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/comments.py:420",
  "site": "src/black/comments.py:519",
  "site_fingerprint": "18cb1f832a383dbe"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/handle_ipynb_magics.py:401",
  "site": "src/black/handle_ipynb_magics.py:409",
  "site_fingerprint": "a1115baaf908f5b2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:1369",
  "site": "src/black/linegen.py:1376",
  "site_fingerprint": "6eb1781a049941c0"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:1369",
  "site": "src/black/linegen.py:1377",
  "site_fingerprint": "d21ba4e4c2a4c421"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1029",
  "site_fingerprint": "88ddda633774f6bf"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1031",
  "site_fingerprint": "37bf1bb608219970"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1032",
  "site_fingerprint": "0b2a3cb5aa2c6f8a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1052",
  "site_fingerprint": "65cece78b23e870c"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1005",
  "site_fingerprint": "dcf4cef80b1ec113"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/linegen.py:975",
  "site": "src/black/linegen.py:1075",
  "site_fingerprint": "c4af80d13b546f96"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/lines.py:478",
  "site": "src/black/lines.py:486",
  "site_fingerprint": "3b8fcb58090b68a2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/nodes.py:945",
  "site": "src/black/nodes.py:954",
  "site_fingerprint": "1b8146d27e32c441"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/nodes.py:945",
  "site": "src/black/nodes.py:955",
  "site_fingerprint": "c3f22a37bf09f7bc"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:1137",
  "site": "src/black/trans.py:1178",
  "site_fingerprint": "52c0e50db51d056f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2300",
  "site_fingerprint": "7a2ecb929e281d4e"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2303",
  "site_fingerprint": "9e7283b13fe086b3"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2310",
  "site_fingerprint": "a65f9d085aa1ed77"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2254",
  "site_fingerprint": "8a8f1b59a1a500e4"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2280",
  "site_fingerprint": "10f80491b2efb000"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2282",
  "site_fingerprint": "1bcc4641c03a6d13"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2288",
  "site_fingerprint": "bf17a16e74c0dc30"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2295",
  "site_fingerprint": "188445cd90385d57"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2335",
  "site_fingerprint": "81ed2c090270c097"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2338",
  "site_fingerprint": "b5dc21ea55122250"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2348",
  "site_fingerprint": "afa77979f0af8d1f"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2350",
  "site_fingerprint": "3a97fabbbee5591d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:2231",
  "site": "src/black/trans.py:2352",
  "site_fingerprint": "e00dce1a2c556672"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:933",
  "site_fingerprint": "daebff0916946851"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:941",
  "site_fingerprint": "d5ccc0d12c44eb49"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:957",
  "site_fingerprint": "e89e07024f11c2a6"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:988",
  "site_fingerprint": "4a71fa07084ef519"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:992",
  "site_fingerprint": "d04f1022ef092aaf"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:951",
  "site_fingerprint": "cae9f6205d5cddc9"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/black/trans.py:903",
  "site": "src/black/trans.py:1006",
  "site_fingerprint": "3055e4d4a9532216"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:71",
  "site_fingerprint": "f0949162c5e6358a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:72",
  "site_fingerprint": "989da93898afafa2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:79",
  "site_fingerprint": "ad6d0e2133228ad2"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:82",
  "site_fingerprint": "d6efb2f0ecc6f07a"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:88",
  "site_fingerprint": "e9fdbcb473f6c97d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:90",
  "site_fingerprint": "60200fce10d31d34"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blackd/__init__.py:96",
  "site": "src/blackd/__init__.py:95",
  "site_fingerprint": "1f551c4a2232c070"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blib2to3/pytree.py:525",
  "site": "src/blib2to3/pytree.py:540",
  "site_fingerprint": "380134b08d6fecc7"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blib2to3/pytree.py:525",
  "site": "src/blib2to3/pytree.py:542",
  "site_fingerprint": "4305bb92a523943d"
 },
 {
  "corpus": "black-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/blib2to3/pytree.py:998",
  "site": "src/blib2to3/pytree.py:1017",
  "site_fingerprint": "ff1da4cd40b76c85"
 }
]
```
