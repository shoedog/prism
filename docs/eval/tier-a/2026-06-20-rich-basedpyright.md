# Tier-A run — rich-basedpyright (2026-06-20)

- corpus: `rich-basedpyright` @ `46cebbb032f9`
- prism: `20c8490591a3` · oracle: basedpyright 1.39.8 · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.250 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.203, 'm1_oracle_inventory': 0.046, 'm2': 11.454, 'm3': 0.0}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 0.01 [0.00–0.07] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.04 [0.01–0.20] | 1/0/0 | 82 | 0 |
| C-name | 1.00 [0.57–1.00] | 0.71 [0.36–0.92] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.44–1.00] | 0.60 [0.23–0.88] | 5/0/0 | 2 | 0 |
| Q-scoped | 1.00 [0.78–1.00] | 0.93 [0.70–0.99] | 1.00 [0.78–1.00] | 1.00 [0.78–1.00] | 1.00 [0.68–1.00] | 0.89 [0.57–0.98] | 14/0/0 | 1 | 0 |
| U-method | 1.00 [0.68–1.00] | 0.19 [0.10–0.33] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.57–1.00] | 0.19 [0.09–0.38] | 8/0/0 | 35 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.51–1.00] | 0.29 [0.12–0.55] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.80 [0.38–0.96] | 1.00 [0.51–1.00] | 4/0/0 | 10 | 0 |
| C-name | 0.71 [0.36–0.92] | 0.45 [0.21–0.72] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.50 [0.24–0.76] | 1.00 [0.57–1.00] | 5/0/0 | 8 | 0 |
| Q-scoped | 0.71 [0.36–0.92] | 0.45 [0.21–0.72] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.50 [0.22–0.78] | 1.00 [0.51–1.00] | 5/0/0 | 8 | 0 |
| U-method | 0.67 [0.21–0.94] | 0.29 [0.08–0.64] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.43 [0.16–0.75] | 1.00 [0.44–1.00] | 2/0/0 | 6 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 9,
 "matched": 931,
 "prism_extra": 24,
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
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/_win32_console.py:78",
  "site": "rich/_win32_console.py:576",
  "site_fingerprint": "6070a479db18122e"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:260",
  "site": "rich/console.py:1319",
  "site_fingerprint": "86eed9239324bbf2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:464",
  "site": "rich/console.py:473",
  "site_fingerprint": "9f204329ae6b8fca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:464",
  "site": "rich/console.py:480",
  "site_fingerprint": "3300b9f7ebc359e2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:391",
  "site_fingerprint": "7e8036fa1e53f692"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:392",
  "site_fingerprint": "e6308f4ab668bcbd"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:876",
  "site_fingerprint": "6e9e9e38013267e7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:877",
  "site_fingerprint": "dde83f861e4505ac"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:1618",
  "site_fingerprint": "efd4f429963dcdcc"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:1750",
  "site_fingerprint": "ad4db6ff5386832f"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:1756",
  "site_fingerprint": "c881d8734b83d584"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:1870",
  "site_fingerprint": "33b9986f254e376e"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2024",
  "site_fingerprint": "8f8f55681fae9a45"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2051",
  "site_fingerprint": "2b3145bacc051ce9"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2065",
  "site_fingerprint": "1ce9db31e304beee"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2071",
  "site_fingerprint": "499783c23ce93017"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2072",
  "site_fingerprint": "33b50e76596330ff"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2087",
  "site_fingerprint": "abef5389bcbc71e8"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2094",
  "site_fingerprint": "d7f0d38c7e89cb74"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2122",
  "site_fingerprint": "0bc201665e39cf39"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/console.py:771",
  "site": "rich/console.py:2130",
  "site_fingerprint": "0fd3524037959ad4"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/control.py:69",
  "site": "rich/console.py:1094",
  "site_fingerprint": "453cad0275b3a785"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/logging.py:291",
  "site": "rich/logging.py:301",
  "site_fingerprint": "e34e0f0bc379156d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/columns.py:72",
  "site_fingerprint": "e72ef4703a280d60"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/panel.py:144",
  "site_fingerprint": "509284d49d39feaa"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/panel.py:281",
  "site_fingerprint": "d7f8b6266900f946"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/syntax.py:240",
  "site_fingerprint": "42a72031040937ac"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/syntax.py:312",
  "site_fingerprint": "8ced4d695cba8420"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/table.py:225",
  "site_fingerprint": "8bcc44f51b0f0de4"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "rich/table.py:361",
  "site_fingerprint": "7d1da89b2414c35a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "tests/test_padding.py:23",
  "site_fingerprint": "44c89c3e2d456a96"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "tests/test_padding.py:24",
  "site_fingerprint": "5b0f46aed0a4126c"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "tests/test_padding.py:25",
  "site_fingerprint": "443a6abc4426e71d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "tests/test_padding.py:26",
  "site_fingerprint": "94ec6790022194f8"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/padding.py:61",
  "site": "tests/test_padding.py:28",
  "site_fingerprint": "0f0ce654a2b520c4"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/scope.py:81",
  "site": "rich/scope.py:91",
  "site_fingerprint": "28664f5b7864ac01"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/segment.py:426",
  "site": "rich/align.py:156",
  "site_fingerprint": "59c91defde8d6bfb"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/segment.py:426",
  "site": "rich/screen.py:49",
  "site_fingerprint": "1f2a7a2c12d2387a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/segment.py:426",
  "site": "tests/test_segment.py:86",
  "site_fingerprint": "736458a927e850cd"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/segment.py:426",
  "site": "tests/test_segment.py:89",
  "site_fingerprint": "21cccc1b3c77cbb7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "benchmarks/benchmarks.py:153",
  "site_fingerprint": "2850bbd9fa554c53"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "benchmarks/benchmarks.py:154",
  "site_fingerprint": "9d3a1e7cebe4f5c4"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "benchmarks/benchmarks.py:157",
  "site_fingerprint": "f54c0fbd76dc64b0"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "benchmarks/benchmarks.py:160",
  "site_fingerprint": "d7eb79506732887f"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "benchmarks/benchmarks.py:163",
  "site_fingerprint": "efc1284392760a05"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:22",
  "site_fingerprint": "56491b0479eb39c6"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:23",
  "site_fingerprint": "f9d1cf80f1d37a40"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:24",
  "site_fingerprint": "97d1724d7a8ada92"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:25",
  "site_fingerprint": "6322c9a3cb0e9948"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:26",
  "site_fingerprint": "0966002aadf02b91"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "examples/log.py:27",
  "site_fingerprint": "ffa9ea293d569370"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/_win32_console.py:587",
  "site_fingerprint": "39a5adb5937e4a43"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/_win32_console.py:639",
  "site_fingerprint": "3baf4d4dcf9d5d65"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/_win32_console.py:650",
  "site_fingerprint": "3ed0d6db43326a20"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/_win32_console.py:656",
  "site_fingerprint": "005b6286987db648"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/ansi.py:176",
  "site_fingerprint": "21602afa3202e690"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/console.py:1491",
  "site_fingerprint": "5c48400815762c66"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/theme.py:24",
  "site_fingerprint": "5cdb3a23e56eb54c"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "rich/theme.py:55",
  "site_fingerprint": "4852cc0aa7c4c48a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_ansi.py:26",
  "site_fingerprint": "9d7fc0cdd9ea64e5"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_ansi.py:27",
  "site_fingerprint": "a50039627207d5a3"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_ansi.py:28",
  "site_fingerprint": "693aa359346feb4a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_ansi.py:29",
  "site_fingerprint": "2fd854881dbb9ba2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:67",
  "site_fingerprint": "cb150d1a78dda88e"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:71",
  "site_fingerprint": "d19d43f9687d7e38"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:72",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:73",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:74",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:75",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:76",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:77",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:78",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:79",
  "site_fingerprint": "b94c8759c37d9bca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:80",
  "site_fingerprint": "ef7aa177a93d22ae"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:81",
  "site_fingerprint": "259f4d3aa6a4bc4d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:82",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:83",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:84",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:85",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:86",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:87",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:88",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:89",
  "site_fingerprint": "3bdea6a5852ef26d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_bar.py:90",
  "site_fingerprint": "e49d28a95690bd92"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_containers.py:54",
  "site_fingerprint": "7e56749a869dc206"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_live_render.py:45",
  "site_fingerprint": "549261333e5788dc"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:85",
  "site_fingerprint": "f1d1c7391820d42c"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:97",
  "site_fingerprint": "8e709e864149e5c8"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:98",
  "site_fingerprint": "3fb49d8a5e0c2c67"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:99",
  "site_fingerprint": "0e79fc3b02a788d7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:100",
  "site_fingerprint": "d12735458b4f9f19"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:103",
  "site_fingerprint": "e572f4850fb40661"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:107",
  "site_fingerprint": "a6c57b5e5f5c8849"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:109",
  "site_fingerprint": "ec34aebe0c3ad24d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:111",
  "site_fingerprint": "5510544668ea76f7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:113",
  "site_fingerprint": "2399294bc697e96e"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:115",
  "site_fingerprint": "46e6b08b4376b6a9"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:120",
  "site_fingerprint": "15d574e752656998"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:121",
  "site_fingerprint": "b78e9dd1277b89cb"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:122",
  "site_fingerprint": "c69ab04a1ea8cdb6"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:235",
  "site_fingerprint": "d9764c3d25ae9261"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_style.py:259",
  "site_fingerprint": "cc7b499c55483eee"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_syntax.py:169",
  "site_fingerprint": "1a5ebcea687291e7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_syntax.py:199",
  "site_fingerprint": "8063ee3f1d6d3747"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_syntax.py:206",
  "site_fingerprint": "d812bc6f46d6c0a6"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_syntax.py:207",
  "site_fingerprint": "deee78ea5395be54"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_syntax.py:208",
  "site_fingerprint": "375907af28d9604a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_table.py:157",
  "site_fingerprint": "5d1d527012294fb6"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_table.py:158",
  "site_fingerprint": "04bdeeedd4044eec"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_text.py:236",
  "site_fingerprint": "59343a8edda46cc5"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_theme.py:46",
  "site_fingerprint": "7348a1c850f9d82a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_theme.py:49",
  "site_fingerprint": "d922ed709e1325cd"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_theme.py:51",
  "site_fingerprint": "52efe711a286e13a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:85",
  "site_fingerprint": "5dc436feb7221034"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:109",
  "site_fingerprint": "ffad39f59d15ac18"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:126",
  "site_fingerprint": "82e729f6bdd5da1f"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:143",
  "site_fingerprint": "ca495dc6c7855843"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:160",
  "site_fingerprint": "7ac09f90c65e69d0"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_win32_console.py:177",
  "site_fingerprint": "8b2e5fff22592e45"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "rich/style.py:498",
  "site": "tests/test_windows_renderer.py:43",
  "site_fingerprint": "5cf265cbf7d8f64f"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "benchmarks/benchmarks.py:31",
  "site": "benchmarks/benchmarks.py:32",
  "site_fingerprint": "ace3c40ac2415448"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/_extension.py:4",
  "site": "rich/_extension.py:10",
  "site_fingerprint": "82c0c5f12c4fc0ce"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "rich/console.py:1550",
  "site": "rich/console.py:1554",
  "site_fingerprint": "eea17e1ad444c351"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/console.py:1550",
  "site": "rich/console.py:1552",
  "site_fingerprint": "74a5d164ebe5088c"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "rich/console.py:2386",
  "site": "rich/console.py:2415",
  "site_fingerprint": "5d9e68307c368675"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/console.py:2386",
  "site": "rich/console.py:2393",
  "site_fingerprint": "7330dd63d7d1c79e"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/console.py:2386",
  "site": "rich/console.py:2398",
  "site_fingerprint": "24c1aae0ba439d45"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/console.py:2386",
  "site": "rich/console.py:2405",
  "site_fingerprint": "845f664b87a4e540"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/console.py:490",
  "site": "rich/console.py:498",
  "site_fingerprint": "6fecdfbe4de6ce25"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/highlighter.py:20",
  "site": "rich/highlighter.py:33",
  "site_fingerprint": "c8c65071d78dfec5"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/highlighter.py:20",
  "site": "rich/highlighter.py:35",
  "site_fingerprint": "1b5b1ae61c397257"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/progress.py:104",
  "site": "rich/progress.py:148",
  "site_fingerprint": "c853c8cef580fa57"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/progress.py:104",
  "site": "rich/progress.py:152",
  "site_fingerprint": "8a7d73a78d919a9a"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/progress.py:104",
  "site": "rich/progress.py:158",
  "site_fingerprint": "a7e3812795996ce7"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/progress.py:104",
  "site": "rich/progress.py:159",
  "site_fingerprint": "72703bd9add9c4db"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/progress.py:104",
  "site": "rich/progress.py:162",
  "site_fingerprint": "98a8a3f4c77e8ee9"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "rich/repr.py:41",
  "site": "rich/repr.py:63",
  "site_fingerprint": "d0a6403ba53e06cd"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "rich/repr.py:41",
  "site": "rich/repr.py:65",
  "site_fingerprint": "0c78a2d65bad4eb3"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/repr.py:41",
  "site": "rich/repr.py:85",
  "site_fingerprint": "e5d85ffa1d8793ab"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/rule.py:105",
  "site": "rich/rule.py:106",
  "site_fingerprint": "ee679e51c5f104ff"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/rule.py:105",
  "site": "rich/rule.py:107",
  "site_fingerprint": "095e99cecdd62edd"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:525",
  "site_fingerprint": "b2b61eb19c8d7fe2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:527",
  "site_fingerprint": "35a1f9632f82b8ca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:529",
  "site_fingerprint": "44804f76b710473d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:538",
  "site_fingerprint": "148dc048fc917c4c"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:546",
  "site_fingerprint": "d7330e7e186dceb2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:554",
  "site_fingerprint": "35a1f9632f82b8ca"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:556",
  "site_fingerprint": "982a473c0a1d3243"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/style.py:498",
  "site": "rich/style.py:560",
  "site_fingerprint": "ca58cedf4c07d358"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "rich/text.py:572",
  "site": "rich/text.py:588",
  "site_fingerprint": "a743916feb250ce2"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/text.py:572",
  "site": "rich/text.py:580",
  "site_fingerprint": "194edcafea093f0d"
 },
 {
  "corpus": "rich-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "rich/text.py:572",
  "site": "rich/text.py:591",
  "site_fingerprint": "e71d718f16d49077"
 }
]
```
