# Tier-A run — prometheus (2026-07-04)

- corpus: `prometheus` @ `505095b64b43`
- prism: `fb81481dafa7` · oracle: gopls {"GoVersion":"go1.26.2","Path":"golang.org/x/tools/gopls","Main":{"Path":"golang.org/x/tools/gopls","Version":"v0.22.0","Sum":"h1:9aON2mTxKrZ1y71RbQVnlPTTNfKZ9y5cv3x/f6XpsZw="},"Deps":[{"Path":"github.com/BurntSushi/toml","Version":"v1.6.0","Sum":"h1:dRaEfpa2VI55EwlIW72hMRHdWouJeRF7TPYhI+AUQjk="},{"Path":"github.com/fatih/camelcase","Version":"v1.0.0","Sum":"h1:hxNvNX/xYBp0ovncs8WyWZrOrpBNub/JfaMvbURyft8="},{"Path":"github.com/fatih/gomodifytags","Version":"v1.17.1-0.20250423142747-f3939df9aa3c","Sum":"h1:dDSgAjoOMp8da3egfz0t2S+t8RGOpEmEXZubcGuc0Bg="},{"Path":"github.com/fatih/structtag","Version":"v1.2.0","Sum":"h1:/OdNE99OxoI/PqaW/SuSK9uxxT3f/tcSZgon/ssNSx4="},{"Path":"github.com/fsnotify/fsnotify","Version":"v1.9.0","Sum":"h1:2Ml+OJNzbYCTzsxtv8vKSFD9PbJjmhYF14k/jKC7S9k="},{"Path":"github.com/google/go-cmp","Version":"v0.7.0","Sum":"h1:wk8382ETsv4JYUZwIsn6YpYiWiBsYLSJiTsyBybVuN8="},{"Path":"github.com/google/jsonschema-go","Version":"v0.4.2","Sum":"h1:tmrUohrwoLZZS/P3x7ex0WAVknEkBZM46iALbcqoRA8="},{"Path":"github.com/modelcontextprotocol/go-sdk","Version":"v1.4.0","Sum":"h1:u0kr8lbJc1oBcawK7Df+/ajNMpIDFE41OEPxdeTLOn8="},{"Path":"github.com/segmentio/asm","Version":"v1.2.1","Sum":"h1:DTNbBqs57ioxAD4PrArqftgypG4/qNpXoJx8TVXxPR0="},{"Path":"github.com/segmentio/encoding","Version":"v0.5.3","Sum":"h1:OjMgICtcSFuNvQCdwqMCv9Tg7lEOXGwm1J5RPQccx6w="},{"Path":"github.com/yosida95/uritemplate/v3","Version":"v3.0.2","Sum":"h1:Ed3Oyj9yrmi9087+NczuL5BwkIc4wvTb5zIM+UJPGz4="},{"Path":"golang.org/x/exp/typeparams","Version":"v0.0.0-20260312153236-7ab1446f8b90","Sum":"h1:cfW8UCYSVdPblxA7qQe3o5Iad55Vsx4BFmuGS9RNOmc="},{"Path":"golang.org/x/mod","Version":"v0.35.0","Sum":"h1:Ww1D637e6Pg+Zb2KrWfHQUnH2dQRLBQyAtpr/haaJeM="},{"Path":"golang.org/x/oauth2","Version":"v0.36.0","Sum":"h1:peZ/1z27fi9hUOFCAZaHyrpWG5lwe0RJEEEeH0ThlIs="},{"Path":"golang.org/x/sync","Version":"v0.20.0","Sum":"h1:e0PTpb7pjO8GAtTs2dQ6jYa5BWYlMuX047Dco/pItO4="},{"Path":"golang.org/x/sys","Version":"v0.43.0","Sum":"h1:Rlag2XtaFTxp19wS8MXlJwTvoh8ArU6ezoyFsMyCTNI="},{"Path":"golang.org/x/telemetry","Version":"v0.0.0-20260409153401-be6f6cb8b1fa","Sum":"h1:efT73AJZfAAUV7SOip6pWGkwJDzIGiKBZGVzHYa+ve4="},{"Path":"golang.org/x/text","Version":"v0.36.0","Sum":"h1:JfKh3XmcRPqZPKevfXVpI1wXPTqbkE5f7JA92a55Yxg="},{"Path":"golang.org/x/tools","Version":"v0.44.1-0.20260513175300-635ae9663724","Sum":"h1:rz7KAMVmPA+7ecCT/BHKLE353MUOoCAwMqlUveE8508="},{"Path":"golang.org/x/vuln","Version":"v1.1.4","Sum":"h1:Ju8QsuyhX3Hk8ma3CesTbO8vfJD9EvUBgHvkxHBzj0I="},{"Path":"honnef.co/go/tools","Version":"v0.7.0","Sum":"h1:w6WUp1VbkqPEgLz4rkBzH/CSU6HkoqNLp6GstyTx3lU="},{"Path":"mvdan.cc/gofumpt","Version":"v0.9.2","Sum":"h1:zsEMWL8SVKGHNztrx6uZrXdp7AX8r421Vvp23sz7ik4="},{"Path":"mvdan.cc/xurls/v2","Version":"v2.6.0","Sum":"h1:3NTZpeTxYVWNSokW3MKeyVkz/j7uYXYiMtXRUfmjbgI="}],"Settings":[{"Key":"-buildmode","Value":"exe"},{"Key":"-compiler","Value":"gc"},{"Key":"CGO_ENABLED","Value":"1"},{"Key":"CGO_CFLAGS"},{"Key":"CGO_CPPFLAGS"},{"Key":"CGO_CXXFLAGS"},{"Key":"CGO_LDFLAGS"},{"Key":"GOARCH","Value":"arm64"},{"Key":"GOOS","Value":"darwin"},{"Key":"GOARM64","Value":"v8.0"}],"Version":"v0.22.0"} · seed: 42 · harness: `fb81481dafa7`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 6.179, 'm1_oracle_inventory': 77.044, 'm2': 129.071, 'm3': 0.151}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.26 [0.13–0.45] | 0.21 [0.10–0.37] | 0.26 [0.13–0.45] | 0.58 [0.32–0.81] | 0.26 [0.13–0.46] | 0.21 [0.10–0.38] | 7/20/5 | 2 | 0 |
| C-name | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.77–1.00] | 1.00 [0.77–1.00] | 17/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 8/0/0 | 0 | 0 |
| U-method | 0.85 [0.74–0.92] | 0.84 [0.72–0.91] | 1.00 [0.92–1.00] | 0.84 [0.72–0.91] | 1.00 [0.85–1.00] | 0.95 [0.78–0.99] | 47/0/9 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 1.00 [0.51–1.00] | 0.12 [0.05–0.27] | 4/0/30 | 23 | 3 | 20 |
| C-name | 1.00 [0.81–1.00] | 0.94 [0.73–0.99] | 16/0/1 | 1 | 1 | 0 |
| Q-scoped | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 8/0/0 | 0 | 0 | 0 |
| U-method | 1.00 [0.72–1.00] | 0.18 [0.10–0.30] | 10/0/46 | 45 | 37 | 8 |

## M2 callees

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.83 [0.61–0.94] | 0.88 [0.66–0.97] | 1.00 [0.82–1.00] | 0.90 [0.70–0.97] | 0.86 [0.49–0.97] | 1.00 [0.61–1.00] | 18/0/2 | 0 | 0 |
| C-name | 0.83 [0.55–0.95] | 0.59 [0.36–0.78] | 1.00 [0.76–1.00] | 0.63 [0.41–0.81] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 12/0/7 | 0 | 0 |
| Q-scoped | 1.00 [0.82–1.00] | 0.50 [0.34–0.66] | 1.00 [0.82–1.00] | 0.50 [0.34–0.66] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 18/0/18 | 0 | 0 |
| U-method | 0.95 [0.75–0.99] | 0.78 [0.58–0.90] | 1.00 [0.82–1.00] | 0.78 [0.58–0.90] | 0.71 [0.36–0.92] | 1.00 [0.57–1.00] | 18/0/5 | 0 | 0 |

_exact/candidate tier (P3 gate reads exact_tier only; candidate_tier is informational)_

| stratum | exact P | exact R | exact tp/fp/fn | candidate count | oracle-confirmed | oracle-unconfirmed |
|---|---|---|---|---|---|---|
| C-method | 0.88 [0.66–0.97] | 0.88 [0.66–0.97] | 15/2/2 | 1 | 0 | 1 |
| C-name | 0.83 [0.55–0.95] | 0.59 [0.36–0.78] | 10/2/7 | 1 | 1 | 0 |
| Q-scoped | 1.00 [0.81–1.00] | 0.44 [0.30–0.60] | 16/0/20 | 3 | 3 | 0 |
| U-method | 0.94 [0.72–0.99] | 0.65 [0.45–0.81] | 15/1/8 | 3 | 3 | 0 |

## M1 inventory diff

```json
{
 "anon_oracle": 0,
 "anon_prism": 0,
 "matched": 9289,
 "prism_extra": 0,
 "prism_missing": 442,
 "snapshot_prism_missing": 442
}
```

## M3 spot-check

```json
{
 "cap": 25,
 "checked": [
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/dns/dns_test.go:270",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/eureka/eureka_test.go:59",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/file/file.go:249",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/file/file.go:273",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/file/file.go:278",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/linode/linode_test.go:251",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/moby/nodes_test.go:59",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/moby/services_test.go:59",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/moby/services_test.go:364",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/moby/tasks_test.go:59",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/nomad/nomad_test.go:195",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/puppetdb/puppetdb_test.go:109",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/puppetdb/puppetdb_test.go:161",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/puppetdb/puppetdb_test.go:224",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/puppetdb/puppetdb_test.go:256",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/refresh/refresh.go:69",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/refresh/refresh.go:88",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/uyuni/uyuni_test.go:59",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/uyuni/uyuni_test.go:149",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:discovery/marathon/marathon.go:229",
   "site": "discovery/vultr/vultr_test.go:71",
   "verdict": "ambiguous"
  },
  {
   "probe": "callers:web/api/testhelpers/openapi.go:98",
   "site": "web/api/v1/api_scenarios_test.go:47",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:web/api/testhelpers/openapi.go:98",
   "site": "web/api/v1/api_scenarios_test.go:58",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:web/api/testhelpers/openapi.go:98",
   "site": "web/api/v1/api_scenarios_test.go:164",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:web/api/testhelpers/openapi.go:98",
   "site": "web/api/v1/api_scenarios_test.go:202",
   "verdict": "alias_site"
  },
  {
   "probe": "callers:web/api/testhelpers/openapi.go:98",
   "site": "web/api/v1/api_scenarios_test.go:310",
   "verdict": "alias_site"
  }
 ],
 "counts": {
  "alias_site": 5,
  "ambiguous": 20,
  "confirmed_fp": 0,
  "confirmed_tp": 0
 }
}
```

## Pending triage

```json
[]
```
