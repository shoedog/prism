# Tier-A run — click-basedpyright (2026-06-20)

- corpus: `click-basedpyright` @ `8a1b1a33d739`
- prism: `20c8490591a3` · oracle: basedpyright 1.39.8 · seed: 42 · harness: `06e6ac4ee96e`
- oracle_error_rate: 0.312 · sut_error_rate: 0.000 · baseline_invalid: True · oracle_not_quiescent: False
- wall (s): {'oracle_start': 5.203, 'm1_oracle_inventory': 0.085, 'm2': 9.481, 'm3': 0.003}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 1.00 [0.21–1.00] | 0.25 [0.05–0.70] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 1.00 [0.21–1.00] | 0.25 [0.05–0.70] | 1/0/0 | 3 | 0 |
| Q-scoped | 0.95 [0.75–0.99] | 0.86 [0.65–0.95] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 0.93 [0.69–0.99] | 0.93 [0.69–0.99] | 18/0/0 | 4 | 0 |
| U-method | 0.84 [0.67–0.93] | 0.96 [0.82–0.99] | 1.00 [0.87–1.00] | 1.00 [0.87–1.00] | 0.86 [0.67–0.95] | 0.95 [0.76–0.99] | 26/0/0 | 6 | 0 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.92 [0.65–0.99] | 0.61 [0.39–0.80] | 1.00 [0.74–1.00] | 1.00 [0.74–1.00] | 1.00 [0.61–1.00] | 1.00 [0.61–1.00] | 11/0/0 | 8 | 0 |
| C-name | 1.00 [0.74–1.00] | 0.50 [0.31–0.69] | 1.00 [0.74–1.00] | 1.00 [0.74–1.00] | 0.86 [0.49–0.97] | 0.75 [0.41–0.93] | 11/0/0 | 11 | 0 |
| Q-scoped | 0.62 [0.31–0.86] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 1.00 [0.57–1.00] | 0.27 [0.10–0.57] | 1.00 [0.44–1.00] | 5/0/0 | 3 | 0 |
| U-free | 1.00 [0.84–1.00] | 0.43 [0.30–0.58] | 1.00 [0.84–1.00] | 1.00 [0.84–1.00] | 0.89 [0.57–0.98] | 1.00 [0.68–1.00] | 20/0/0 | 26 | 0 |
| U-method | 0.90 [0.60–0.98] | 0.90 [0.60–0.98] | 1.00 [0.70–1.00] | 1.00 [0.70–1.00] | 0.86 [0.49–0.97] | 1.00 [0.61–1.00] | 9/0/0 | 2 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 1,
 "matched": 1541,
 "prism_extra": 42,
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
   "probe": "callers:src/click/_compat.py:150",
   "site": "src/click/_compat.py:246",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:src/click/core.py:748",
   "site": "examples/aliases/aliases.py:50",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/click/core.py:748",
   "site": "examples/aliases/aliases.py:82",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/click/formatting.py:213",
   "site": "tests/test_types.py:147",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/click/parser.py:265",
   "site": "tests/test_commands.py:175",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:src/click/parser.py:265",
   "site": "tests/test_commands.py:178",
   "verdict": "ambiguous"
  }
 ],
 "counts": {
  "alias_site": 1,
  "ambiguous": 5,
  "confirmed_fp": 0,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "local_def",
  "measurement": "callers",
  "seed_def": "src/click/_compat.py:150",
  "site": "src/click/_compat.py:246",
  "site_fingerprint": "4224a3be2bda6f34"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/_compat.py:543",
  "site": "src/click/_compat.py:571",
  "site_fingerprint": "3841b59d7650a633"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/_compat.py:543",
  "site": "src/click/_compat.py:572",
  "site_fingerprint": "d1a6fe436648a1b3"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/_compat.py:543",
  "site": "src/click/_compat.py:573",
  "site_fingerprint": "b8e1bf6974ec4683"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/core.py:1317",
  "site": "src/click/core.py:1940",
  "site_fingerprint": "7f006de1cc1748a4"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/core.py:1317",
  "site": "tests/test_commands.py:558",
  "site_fingerprint": "99a0c20b0043b21a"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/core.py:1660",
  "site": "src/click/core.py:2095",
  "site_fingerprint": "740e2da8295af9c7"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/click/core.py:748",
  "site": "examples/aliases/aliases.py:50",
  "site_fingerprint": "b34d005a13a2561f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/click/core.py:748",
  "site": "examples/aliases/aliases.py:82",
  "site_fingerprint": "16d97be6830cd797"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/click/formatting.py:213",
  "site": "tests/test_types.py:147",
  "site_fingerprint": "c8aa89747c2aa58a"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/click/parser.py:265",
  "site": "tests/test_commands.py:175",
  "site_fingerprint": "6cacb91f1745d9db"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "src/click/parser.py:265",
  "site": "tests/test_commands.py:178",
  "site_fingerprint": "c875bfa3554e6ee5"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "src/click/utils.py:183",
  "site": "src/click/types.py:944",
  "site_fingerprint": "2c1bc147ffa853d2"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/click/_compat.py:150",
  "site": "src/click/_compat.py:152",
  "site_fingerprint": "0dabe77469daa4a7"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/click/core.py:1317",
  "site": "src/click/core.py:1350",
  "site_fingerprint": "fe5e049a2825826f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/core.py:1317",
  "site": "src/click/core.py:1319",
  "site_fingerprint": "8e20a4c03a325bfd"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/core.py:1317",
  "site": "src/click/core.py:1322",
  "site_fingerprint": "dfaa6397edde7fc8"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/core.py:1660",
  "site": "src/click/core.py:1673",
  "site_fingerprint": "bf62c030ff9d487a"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/core.py:2120",
  "site": "src/click/core.py:2121",
  "site_fingerprint": "81c80e7646371f4e"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/decorators.py:76",
  "site": "src/click/decorators.py:93",
  "site_fingerprint": "9f79d31a960727ce"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/decorators.py:77",
  "site": "src/click/decorators.py:93",
  "site_fingerprint": "9f79d31a960727ce"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/exceptions.py:335",
  "site": "src/click/exceptions.py:336",
  "site_fingerprint": "7dd15f080b6ec5a8"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/exceptions.py:87",
  "site": "src/click/exceptions.py:100",
  "site_fingerprint": "d9bf1ed295ec7e09"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/exceptions.py:87",
  "site": "src/click/exceptions.py:108",
  "site_fingerprint": "16eee64e36f2f60f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "src/click/parser.py:265",
  "site": "src/click/parser.py:284",
  "site_fingerprint": "8fd999c1d71f2aef"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "src/click/parser.py:265",
  "site": "src/click/parser.py:283",
  "site_fingerprint": "4e2ec9b31ecc8bf2"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/click/termui.py:83",
  "site": "src/click/termui.py:89",
  "site_fingerprint": "4555459541e224f0"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "src/click/termui.py:83",
  "site": "src/click/termui.py:90",
  "site_fingerprint": "840fc126d4f30b43"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_arguments.py:192",
  "site": "tests/test_arguments.py:190",
  "site_fingerprint": "134d05a507bc161c"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_arguments.py:229",
  "site": "tests/test_arguments.py:227",
  "site_fingerprint": "42231de450c6181c"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_arguments.py:229",
  "site": "tests/test_arguments.py:228",
  "site_fingerprint": "1fce0e12ce4a92a6"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_basic.py:691",
  "site": "tests/test_basic.py:690",
  "site_fingerprint": "85ecf5de765e5e7f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_chain.py:54",
  "site": "tests/test_chain.py:53",
  "site_fingerprint": "e16b31fab399f646"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:622",
  "site_fingerprint": "9c6b12920711b203"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:638",
  "site_fingerprint": "4a82e721f09436fd"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:648",
  "site_fingerprint": "b4e21b0710a9fac0"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:650",
  "site_fingerprint": "8042b09f18af2460"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:658",
  "site_fingerprint": "b4e21b0710a9fac0"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:592",
  "site": "tests/test_context.py:660",
  "site_fingerprint": "8042b09f18af2460"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:700",
  "site": "tests/test_context.py:704",
  "site_fingerprint": "74e169e86a212f96"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_context.py:700",
  "site": "tests/test_context.py:706",
  "site_fingerprint": "d4a768f0813a4b69"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_defaults.py:332",
  "site": "tests/test_defaults.py:324",
  "site_fingerprint": "743dee480246452f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_defaults.py:332",
  "site": "tests/test_defaults.py:325",
  "site_fingerprint": "b53ce84d227cc3dc"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:99",
  "site_fingerprint": "896300fe9bbc6f29"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:110",
  "site_fingerprint": "7612c040461e0b20"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:120",
  "site_fingerprint": "be6956b73cdaff7d"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:126",
  "site_fingerprint": "13ce7082e26ff772"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:142",
  "site_fingerprint": "5729346b88127c25"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:159",
  "site_fingerprint": "af7e150b42cc966f"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:164",
  "site_fingerprint": "612ba159a6d2ebdf"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:178",
  "site_fingerprint": "66d0bd7f5943c70c"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_info_dict.py:212",
  "site": "tests/test_info_dict.py:188",
  "site_fingerprint": "be7ad556d02bf742"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_options.py:1250",
  "site": "tests/test_options.py:1254",
  "site_fingerprint": "c1368959d8df7912"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_options.py:1250",
  "site": "tests/test_options.py:1261",
  "site_fingerprint": "f18ac8cd5a256e80"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_options.py:1554",
  "site": "tests/test_options.py:1552",
  "site_fingerprint": "bebe16063489c787"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_options.py:1554",
  "site": "tests/test_options.py:1553",
  "site_fingerprint": "848265c8cd2fdddd"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:177",
  "site": "tests/test_shell_completion.py:178",
  "site_fingerprint": "46a4cc75ce2d524d"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:177",
  "site": "tests/test_shell_completion.py:182",
  "site_fingerprint": "94982be3c48603a8"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:177",
  "site": "tests/test_shell_completion.py:183",
  "site_fingerprint": "5e9dcea68c44080c"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:226",
  "site": "tests/test_shell_completion.py:227",
  "site_fingerprint": "05c4c9652cc0887c"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:226",
  "site": "tests/test_shell_completion.py:231",
  "site_fingerprint": "0574d4071e6c5c1e"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:226",
  "site": "tests/test_shell_completion.py:232",
  "site_fingerprint": "366d19979c5f485e"
 },
 {
  "corpus": "click-basedpyright",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tests/test_shell_completion.py:226",
  "site": "tests/test_shell_completion.py:233",
  "site_fingerprint": "154ea24dc8d4b4bb"
 }
]
```
