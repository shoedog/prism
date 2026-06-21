# Tier-A run — packaging (2026-06-20)

- corpus: `packaging` @ `b61e85eafa19`
- prism: `20c8490591a3` · oracle: pyright-langserver · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.375 · sut_error_rate: 0.000 · baseline_invalid: True · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.164, 'm1_oracle_inventory': 0.022, 'm2': 9.923, 'm3': 0.013}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 0.08 [0.01–0.33] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.50 [0.09–0.91] | 1/0/0 | 12 | 0 |
| C-name | 0.33 [0.14–0.61] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.33 [0.06–0.79] | 0.25 [0.05–0.70] | 4/0/0 | 11 | 0 |
| Q-scoped | 1.00 [0.70–1.00] | 0.90 [0.60–0.98] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 1.00 [0.68–1.00] | 0.89 [0.57–0.98] | 9/0/0 | 1 | 0 |
| U-free | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 5/0/0 | 0 | 0 |
| U-method | 0.90 [0.70–0.97] | 0.67 [0.48–0.81] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 0.78 [0.45–0.94] | 0.64 [0.35–0.85] | 18/0/0 | 11 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.44–1.00] | 0.50 [0.19–0.81] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.67 [0.30–0.90] | 1.00 [0.51–1.00] | 3/0/0 | 3 | 0 |
| C-name | 0.93 [0.69–0.99] | 0.62 [0.41–0.79] | 1.00 [0.77–1.00] | 1.00 [0.77–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 13/0/0 | 9 | 0 |
| Q-scoped | 1.00 [0.81–1.00] | 0.80 [0.58–0.92] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 16/0/0 | 4 | 0 |
| U-free | 0.57 [0.25–0.84] | 0.80 [0.38–0.96] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.50 [0.19–0.81] | 1.00 [0.44–1.00] | 4/0/0 | 4 | 0 |
| U-method | 1.00 [0.70–1.00] | 0.69 [0.42–0.87] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 0.83 [0.44–0.97] | 0.83 [0.44–0.97] | 9/0/0 | 4 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 2,
 "matched": 447,
 "prism_extra": 15,
 "prism_missing": 0,
 "snapshot_prism_missing": 0
}
```

## M3 spot-check

```json
{
 "cap": 25,
 "checked": [
  {
   "probe": "callers:src/packaging/_manylinux.py:237",
   "site": "tests/test_tags.py:933",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_manylinux.py:237",
   "site": "tests/test_tags.py:940",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_manylinux.py:237",
   "site": "tests/test_tags.py:1873",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_manylinux.py:237",
   "site": "tests/test_tags.py:1898",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_musllinux.py:59",
   "site": "tests/test_tags.py:933",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_musllinux.py:59",
   "site": "tests/test_tags.py:940",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_musllinux.py:59",
   "site": "tests/test_tags.py:1873",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/_musllinux.py:59",
   "site": "tests/test_tags.py:1898",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/dependency_groups.py:143",
   "site": "tasks/licenses.py:51",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/packaging/dependency_groups.py:143",
   "site": "tests/test_tags.py:849",
   "verdict": "alias_site"
  }
 ],
 "counts": {
  "alias_site": 1,
  "ambiguous": 9,
  "confirmed_fp": 0,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "tests/test_tags.py:933",
  "site_fingerprint": "a46a75b9486c00cf"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "tests/test_tags.py:940",
  "site_fingerprint": "f91bc8286ba8a1f9"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "tests/test_tags.py:1873",
  "site_fingerprint": "adb708573b924c5f"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "tests/test_tags.py:1898",
  "site_fingerprint": "2fe58b25dfcbaae0"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "src/packaging/tags.py:800",
  "site_fingerprint": "4e2c2247f5307d75"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "tests/test_tags.py:933",
  "site_fingerprint": "a46a75b9486c00cf"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "tests/test_tags.py:940",
  "site_fingerprint": "f91bc8286ba8a1f9"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "tests/test_tags.py:1873",
  "site_fingerprint": "adb708573b924c5f"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "import_qualified",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "tests/test_tags.py:1898",
  "site_fingerprint": "2fe58b25dfcbaae0"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "src/packaging/_musllinux.py:87",
  "site_fingerprint": "7b161ab7b28ec9a4"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/_musllinux.py:59",
  "site": "src/packaging/tags.py:801",
  "site_fingerprint": "d0dae9e13861ebcf"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/packaging/dependency_groups.py:143",
  "site": "tasks/licenses.py:51",
  "site_fingerprint": "4df67d318e38516f"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/packaging/dependency_groups.py:143",
  "site": "tests/test_tags.py:849",
  "site_fingerprint": "5ed6721536dddf1f"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:47",
  "site_fingerprint": "c854d3d87917fed1"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:51",
  "site_fingerprint": "f1e5ff5ab48ded10"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:99",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:117",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:132",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:151",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:163",
  "site_fingerprint": "5c53715deaff7a40"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:200",
  "site_fingerprint": "fce440fd9ff12209"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:204",
  "site_fingerprint": "86d17266b37a9aa3"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:214",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:227",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/direct_url.py:301",
  "site": "tests/test_direct_url.py:239",
  "site_fingerprint": "fd315d946714fb86"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:113",
  "site_fingerprint": "8215391eaf216896"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:140",
  "site_fingerprint": "8435dc950a0bde1d"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:1254",
  "site_fingerprint": "8308d6ed13e31aab"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:1836",
  "site_fingerprint": "32a4d512126308cc"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:1848",
  "site_fingerprint": "32a4d512126308cc"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/tags.py:152",
  "site": "tests/test_tags.py:2022",
  "site_fingerprint": "25f401b1afbf02f5"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/version.py:1038",
  "site": "noxfile.py:412",
  "site_fingerprint": "4ffd842c6557c471"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/version.py:1038",
  "site": "tests/property/test_specifier_implied.py:180",
  "site_fingerprint": "73518a464225fc7b"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/packaging/version.py:1038",
  "site": "tests/test_version.py:986",
  "site_fingerprint": "f1cf3a4355e6162c"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "tasks/select_pypi_dist.py:15",
  "site": "tasks/select_pypi_dist.py:30",
  "site_fingerprint": "ca57d43fdbb2f1b1"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "benchmarks/markers.py:33",
  "site": "benchmarks/markers.py:32",
  "site_fingerprint": "c2c6e909c193308f"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "benchmarks/specifiers.py:75",
  "site": "benchmarks/specifiers.py:74",
  "site_fingerprint": "f87ccd5d39c2bcec"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "benchmarks/specifiers.py:75",
  "site": "benchmarks/specifiers.py:77",
  "site_fingerprint": "639e2a3ce6f5f8f0"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "noxfile.py:259",
  "site": "noxfile.py:272",
  "site_fingerprint": "6cb1a6f9cfc1c9d5"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "noxfile.py:371",
  "site": "noxfile.py:382",
  "site_fingerprint": "98c8023908e876dd"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "noxfile.py:371",
  "site": "noxfile.py:411",
  "site_fingerprint": "849d3a435c0f4774"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "noxfile.py:371",
  "site": "noxfile.py:412",
  "site_fingerprint": "4ffd842c6557c471"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "noxfile.py:426",
  "site": "noxfile.py:435",
  "site_fingerprint": "2b0e696eab881a72"
 },
 {
  "corpus": "packaging",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "noxfile.py:426",
  "site": "noxfile.py:458",
  "site_fingerprint": "6f86778cd9792332"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "noxfile.py:561",
  "site": "noxfile.py:577",
  "site_fingerprint": "2e07b94737ce007a"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "src/packaging/_manylinux.py:251",
  "site_fingerprint": "f31fe536785f93a1"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "src/packaging/_manylinux.py:254",
  "site_fingerprint": "0a56662d39993f64"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "src/packaging/_manylinux.py:265",
  "site_fingerprint": "c5f67018aeb9a4b5"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/_manylinux.py:237",
  "site": "src/packaging/_manylinux.py:274",
  "site_fingerprint": "2f2624e218a6e3ea"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/_ranges.py:114",
  "site": "src/packaging/_ranges.py:127",
  "site_fingerprint": "e07ae9fc64e7aa4d"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/markers.py:156",
  "site": "src/packaging/markers.py:167",
  "site_fingerprint": "081339530cfd0380"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/markers.py:156",
  "site": "src/packaging/markers.py:170",
  "site_fingerprint": "c465732a993d35d8"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/metadata.py:947",
  "site": "src/packaging/metadata.py:951",
  "site_fingerprint": "6b8544e859bbb070"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/pylock.py:197",
  "site": "src/packaging/pylock.py:204",
  "site_fingerprint": "e44430fcea6f5179"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/pylock.py:197",
  "site": "src/packaging/pylock.py:206",
  "site_fingerprint": "7d2b8d20cf62cabe"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/version.py:1038",
  "site": "src/packaging/version.py:1044",
  "site_fingerprint": "6fb0b929c497b711"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/version.py:1085",
  "site": "src/packaging/version.py:1095",
  "site_fingerprint": "9507ae321fd90c58"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/packaging/version.py:307",
  "site": "src/packaging/version.py:313",
  "site_fingerprint": "9538cd0ce824f869"
 },
 {
  "corpus": "packaging",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tasks/select_pypi_dist.py:15",
  "site": "tasks/select_pypi_dist.py:17",
  "site_fingerprint": "43728775b41bbdb2"
 }
]
```
