# Tier-A run — flask-basedpyright (2026-06-20)

- corpus: `flask-basedpyright` @ `36e4a824f340`
- prism: `20c8490591a3` · oracle: basedpyright 1.39.8 · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.362 · sut_error_rate: 0.000 · baseline_invalid: True · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.155, 'm1_oracle_inventory': 0.071, 'm2': 6.056, 'm3': 0.002}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.81–1.00] | 1.00 [0.80–1.00] | 1.00 [0.80–1.00] | 16/0/0 | 0 | 0 |
| U-method | 0.57 [0.25–0.84] | 0.50 [0.22–0.78] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.57 [0.25–0.84] | 0.57 [0.25–0.84] | 4/0/0 | 7 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.44–1.00] | 0.38 [0.14–0.69] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.25 [0.07–0.59] | 1.00 [0.34–1.00] | 3/0/0 | 5 | 0 |
| C-name | 1.00 [0.44–1.00] | 0.43 [0.16–0.75] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.57 [0.25–0.84] | 0.57 [0.25–0.84] | 3/0/0 | 4 | 0 |
| Q-scoped | 0.80 [0.38–0.96] | 0.27 [0.11–0.52] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.55 [0.28–0.79] | 1.00 [0.61–1.00] | 4/0/0 | 12 | 0 |
| U-free | 0.75 [0.47–0.91] | 0.69 [0.42–0.87] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 0.70 [0.40–0.89] | 1.00 [0.65–1.00] | 9/0/0 | 7 | 0 |
| U-method | 1.00 [0.72–1.00] | 0.53 [0.32–0.73] | 1.00 [0.72–1.00] | 1.00 [0.72–1.00] | 0.71 [0.36–0.92] | 1.00 [0.57–1.00] | 10/0/0 | 9 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 0,
 "matched": 1367,
 "prism_extra": 32,
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
   "probe": "callers:src/flask/ctx.py:416",
   "site": "tests/test_reqctx.py:23",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/flask/ctx.py:416",
   "site": "tests/test_reqctx.py:126",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/flask/ctx.py:416",
   "site": "tests/test_reqctx.py:140",
   "verdict": "ambiguous"
  }
 ],
 "counts": {
  "alias_site": 0,
  "ambiguous": 3,
  "confirmed_fp": 0,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/flask/ctx.py:416",
  "site": "tests/test_reqctx.py:23",
  "site_fingerprint": "d8394c9646b72866"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/flask/ctx.py:416",
  "site": "tests/test_reqctx.py:126",
  "site_fingerprint": "11aa7aad5427cebd"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/flask/ctx.py:416",
  "site": "tests/test_reqctx.py:140",
  "site_fingerprint": "60a67cf1451cce1b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/flask/sansio/scaffold.py:657",
  "site": "src/flask/sansio/app.py:876",
  "site_fingerprint": "445373be3774754e"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/flask/sessions.py:195",
  "site": "src/flask/sessions.py:346",
  "site_fingerprint": "edef4f4f49e2f032"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/flask/wrappers.py:247",
  "site": "tests/test_basic.py:1934",
  "site_fingerprint": "291e5f9f5296659b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/flask/wrappers.py:247",
  "site": "tests/test_basic.py:1938",
  "site_fingerprint": "95ebf3eb7b70843c"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/app.py:897",
  "site": "src/flask/app.py:926",
  "site_fingerprint": "74515540419db004"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/app.py:897",
  "site": "src/flask/app.py:942",
  "site_fingerprint": "5e3128fa69dcad9b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/app.py:897",
  "site": "src/flask/app.py:943",
  "site_fingerprint": "f1584f077b160d6b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/app.py:97",
  "site": "src/flask/app.py:100",
  "site_fingerprint": "a94921e2ef5e4cac"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/app.py:97",
  "site": "src/flask/app.py:102",
  "site_fingerprint": "0ed2b4495171b343"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:131",
  "site_fingerprint": "db07179c266a03f2"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:142",
  "site_fingerprint": "098d3d605fa352bb"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:159",
  "site_fingerprint": "ac254dc3bf9ffff8"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:163",
  "site_fingerprint": "773c114c5b093105"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:170",
  "site_fingerprint": "ed00fc6670374fb1"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:183",
  "site_fingerprint": "1a89202a706c1929"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:120",
  "site": "src/flask/cli.py:194",
  "site_fingerprint": "32f670b52e7cff78"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/cli.py:493",
  "site": "src/flask/cli.py:502",
  "site_fingerprint": "3087241e73108b54"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/ctx.py:416",
  "site": "src/flask/ctx.py:434",
  "site_fingerprint": "c0edb4446b4d1883"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/debughelpers.py:28",
  "site": "src/flask/debughelpers.py:29",
  "site_fingerprint": "38ee8725f7aa7548"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/debughelpers.py:28",
  "site": "src/flask/debughelpers.py:33",
  "site_fingerprint": "631eec0652a5f55f"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/helpers.py:304",
  "site": "src/flask/helpers.py:323",
  "site_fingerprint": "26ea7137312f4bee"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/flask/helpers.py:63",
  "site": "src/flask/helpers.py:141",
  "site_fingerprint": "539efa9bcf2f1f4b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/flask/json/tag.py:297",
  "site": "src/flask/json/tag.py:307",
  "site_fingerprint": "e378d70d33d4cd0b"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "tests/conftest.py:72",
  "site": "tests/conftest.py:74",
  "site_fingerprint": "832ec710c66ebf29"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_apps/helloworld/hello.py:7",
  "site": "tests/test_apps/helloworld/hello.py:6",
  "site_fingerprint": "26d812e4c853ce50"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_basic.py:1432",
  "site": "tests/test_basic.py:1433",
  "site_fingerprint": "f6acd7a604bf4b9e"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_basic.py:1468",
  "site": "tests/test_basic.py:1471",
  "site_fingerprint": "14aebcd77d6ea113"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_blueprints.py:255",
  "site": "tests/test_blueprints.py:256",
  "site_fingerprint": "d76fd9496749e5f7"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_blueprints.py:730",
  "site": "tests/test_blueprints.py:729",
  "site_fingerprint": "db8b0af6f315ece0"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_cli.py:231",
  "site": "tests/test_cli.py:239",
  "site_fingerprint": "cd68d897617fb7a5"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:53",
  "site_fingerprint": "b5987ae903680955"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:63",
  "site_fingerprint": "8830ab9b775be011"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:66",
  "site_fingerprint": "c5a5bdcca3d68607"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:73",
  "site_fingerprint": "46a7278d987badd2"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:77",
  "site_fingerprint": "4396d65d065464d1"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:78",
  "site_fingerprint": "f9cd7d0de97b23c8"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_helpers.py:44",
  "site": "tests/test_helpers.py:82",
  "site_fingerprint": "0e9941977eb023df"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "tests/test_instance_config.py:16",
  "site": "tests/test_instance_config.py:23",
  "site_fingerprint": "f8c770f8430ebd8f"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "tests/test_instance_config.py:65",
  "site": "tests/test_instance_config.py:71",
  "site_fingerprint": "9923eeb2ad1ee8e7"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_reqctx.py:155",
  "site": "tests/test_reqctx.py:154",
  "site_fingerprint": "1669d8761968f64d"
 },
 {
  "corpus": "flask-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_reqctx.py:155",
  "site": "tests/test_reqctx.py:162",
  "site_fingerprint": "c3adb71f8b1d49de"
 }
]
```
