# Tier-A run — starlette (2026-06-20)

- corpus: `starlette` @ `de970d7b3fac`
- prism: `20c8490591a3` · oracle: pyright-langserver · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.344 · sut_error_rate: 0.000 · baseline_invalid: True · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.112, 'm1_oracle_inventory': 0.019, 'm2': 6.338, 'm3': 0.003}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.51–1.00] | 0.22 [0.09–0.45] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 1.00 [0.51–1.00] | 0.50 [0.22–0.78] | 4/0/0 | 14 | 0 |
| C-name | 0.23 [0.08–0.50] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 1.00 [0.44–1.00] | 0.22 [0.06–0.55] | 1.00 [0.34–1.00] | 3/0/0 | 10 | 0 |
| Q-scoped | 1.00 [0.85–1.00] | 1.00 [0.85–1.00] | 1.00 [0.85–1.00] | 1.00 [0.85–1.00] | 1.00 [0.78–1.00] | 1.00 [0.78–1.00] | 21/0/0 | 0 | 0 |
| U-method | 1.00 [0.87–1.00] | 0.83 [0.66–0.93] | 1.00 [0.87–1.00] | 1.00 [0.87–1.00] | 1.00 [0.61–1.00] | 0.55 [0.28–0.79] | 25/0/0 | 5 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.50 [0.15–0.85] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.50 [0.19–0.81] | 1.00 [0.44–1.00] | 2/0/0 | 3 | 0 |
| C-name | 0.22 [0.06–0.55] | 0.67 [0.21–0.94] | 1.00 [0.34–1.00] | 1.00 [0.34–1.00] | 0.40 [0.12–0.77] | 1.00 [0.34–1.00] | 2/0/0 | 8 | 0 |
| Q-scoped | 0.50 [0.25–0.75] | 0.50 [0.25–0.75] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 0.55 [0.28–0.79] | 1.00 [0.61–1.00] | 6/0/0 | 12 | 0 |
| U-method | 0.83 [0.44–0.97] | 0.38 [0.18–0.64] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.67 [0.30–0.90] | 1.00 [0.51–1.00] | 5/0/0 | 9 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 1,
 "matched": 489,
 "prism_extra": 9,
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
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/middleware/wsgi.py:104",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/requests.py:242",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/requests.py:335",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/responses.py:244",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/routing.py:636",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/routing.py:645",
   "verdict": "confirmed_fp"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/testclient.py:726",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/testclient.py:732",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/websockets.py:40",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:starlette/testclient.py:735",
   "site": "starlette/websockets.py:47",
   "verdict": "alias_site"
  }
 ],
 "counts": {
  "alias_site": 4,
  "ambiguous": 2,
  "confirmed_fp": 4,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/formparsers.py:101",
  "site": "starlette/formparsers.py:112",
  "site_fingerprint": "b8bf7b7472aafde8"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/formparsers.py:211",
  "site": "starlette/formparsers.py:266",
  "site_fingerprint": "8edd6ead667374d3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/middleware/gzip.py:48",
  "site": "starlette/middleware/gzip.py:46",
  "site_fingerprint": "e395978743bc44ae"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/requests.py:234",
  "site": "starlette/middleware/base.py:32",
  "site_fingerprint": "67305dc9144ece69"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/requests.py:234",
  "site": "starlette/middleware/base.py:83",
  "site_fingerprint": "8b151ed21876f04f"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/routing.py:606",
  "site": "starlette/routing.py:580",
  "site_fingerprint": "82dc7ce6dc2866a8"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/middleware/wsgi.py:104",
  "site_fingerprint": "f118e226ed9de209"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/requests.py:242",
  "site_fingerprint": "2d931a58c85c1631"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/requests.py:335",
  "site_fingerprint": "3f97db0c10b00a3e"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/responses.py:244",
  "site_fingerprint": "918451fba629118c"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/routing.py:636",
  "site_fingerprint": "cd4aad2ff0bfe632"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/routing.py:645",
  "site_fingerprint": "291d598da357fa9c"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/testclient.py:726",
  "site_fingerprint": "56fcaa333af27a34"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/testclient.py:732",
  "site_fingerprint": "cdb66c9339ee8e6f"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/websockets.py:40",
  "site_fingerprint": "b8a51ca886012b7d"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callers",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/websockets.py:47",
  "site_fingerprint": "9025425fd017f806"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "starlette/endpoints.py:78",
  "site_fingerprint": "7ad51a267ec64fe8"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "starlette/websockets.py:185",
  "site_fingerprint": "3e8f2eae90a45f39"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:299",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:313",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:329",
  "site_fingerprint": "d4efc65ffe3333ff"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:352",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:367",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:392",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:412",
  "site_fingerprint": "0f30e31309184cf3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:500",
  "site_fingerprint": "3c645328233bceb5"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:502",
  "site_fingerprint": "2dbd711a6e4ebd8a"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:636",
  "site_fingerprint": "65825ad26f634a3c"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "starlette/websockets.py:35",
  "site": "tests/test_websockets.py:648",
  "site_fingerprint": "bb325d96e845b046"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/_exception_handler.py:23",
  "site": "starlette/_exception_handler.py:39",
  "site_fingerprint": "429e394e4cb21ecb"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/_exception_handler.py:23",
  "site": "starlette/_exception_handler.py:42",
  "site_fingerprint": "a8eb2b929d1ead1e"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/applications.py:98",
  "site": "starlette/applications.py:101",
  "site_fingerprint": "96d288d1f14735f4"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/concurrency.py:16",
  "site": "starlette/concurrency.py:25",
  "site_fingerprint": "724a9950fda6ddd8"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/base.py:205",
  "site": "starlette/middleware/base.py:217",
  "site_fingerprint": "29b4b46551813ad3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/gzip.py:48",
  "site": "starlette/middleware/gzip.py:54",
  "site_fingerprint": "50d9ec0f53d0dbb7"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/gzip.py:48",
  "site": "starlette/middleware/gzip.py:74",
  "site_fingerprint": "d032c39d55e2a466"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/gzip.py:48",
  "site": "starlette/middleware/gzip.py:87",
  "site_fingerprint": "d032c39d55e2a466"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/middleware/sessions.py:58",
  "site": "starlette/middleware/sessions.py:70",
  "site_fingerprint": "8e328a17f5e24b9a"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/middleware/sessions.py:58",
  "site": "starlette/middleware/sessions.py:86",
  "site_fingerprint": "d238d9133f87f617"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/sessions.py:58",
  "site": "starlette/middleware/sessions.py:61",
  "site_fingerprint": "b1844d6a8a65f5db"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/sessions.py:58",
  "site": "starlette/middleware/sessions.py:75",
  "site_fingerprint": "42678667feba831e"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/middleware/sessions.py:58",
  "site": "starlette/middleware/sessions.py:85",
  "site_fingerprint": "77faa80d31b030a4"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/requests.py:234",
  "site": "starlette/requests.py:242",
  "site_fingerprint": "2d931a58c85c1631"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/requests.py:234",
  "site": "starlette/requests.py:251",
  "site_fingerprint": "032bfe174efbee7c"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:46",
  "site": "starlette/routing.py:58",
  "site_fingerprint": "06a3aa3926ec2783"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:546",
  "site": "starlette/routing.py:547",
  "site_fingerprint": "945af6dc1e9ea3b3"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:57",
  "site": "starlette/routing.py:58",
  "site_fingerprint": "06a3aa3926ec2783"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:606",
  "site": "starlette/routing.py:608",
  "site_fingerprint": "89bfc75dc9da55a5"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:606",
  "site": "starlette/routing.py:616",
  "site_fingerprint": "228f33253db57c4a"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:606",
  "site": "starlette/routing.py:618",
  "site_fingerprint": "60f6cab200a12d86"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:92",
  "site": "starlette/routing.py:101",
  "site_fingerprint": "1d183ff36f384635"
 },
 {
  "corpus": "starlette",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "starlette/routing.py:92",
  "site": "starlette/routing.py:100",
  "site_fingerprint": "5e914718fb439179"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:300",
  "site": "starlette/testclient.py:308",
  "site_fingerprint": "773f43d6e14c9fc6"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:300",
  "site": "starlette/testclient.py:315",
  "site_fingerprint": "aef1035b44128417"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:328",
  "site": "starlette/testclient.py:334",
  "site_fingerprint": "6d13d888429905eb"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:328",
  "site": "starlette/testclient.py:339",
  "site_fingerprint": "1e3c6117b1b5deb0"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:328",
  "site": "starlette/testclient.py:340",
  "site_fingerprint": "e49b45f6099b06ce"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:328",
  "site": "starlette/testclient.py:344",
  "site_fingerprint": "38f87de1583470aa"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "starlette/testclient.py:735",
  "site": "starlette/testclient.py:736",
  "site_fingerprint": "d2ac4c964d223cce"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/websockets.py:35",
  "site": "starlette/websockets.py:40",
  "site_fingerprint": "b8a51ca886012b7d"
 },
 {
  "corpus": "starlette",
  "direction": "prism_only",
  "dispatch_kind": "free_multi",
  "measurement": "callees",
  "seed_def": "starlette/websockets.py:35",
  "site": "starlette/websockets.py:47",
  "site_fingerprint": "9025425fd017f806"
 }
]
```
