# Tier-A run — prometheus (2026-07-03)

- corpus: `prometheus` @ `505095b64b43`
- prism: `555714da1991` · oracle: gopls {"GoVersion":"go1.26.2","Path":"golang.org/x/tools/gopls","Main":{"Path":"golang.org/x/tools/gopls","Version":"v0.22.0","Sum":"h1:9aON2mTxKrZ1y71RbQVnlPTTNfKZ9y5cv3x/f6XpsZw="},"Deps":[{"Path":"github.com/BurntSushi/toml","Version":"v1.6.0","Sum":"h1:dRaEfpa2VI55EwlIW72hMRHdWouJeRF7TPYhI+AUQjk="},{"Path":"github.com/fatih/camelcase","Version":"v1.0.0","Sum":"h1:hxNvNX/xYBp0ovncs8WyWZrOrpBNub/JfaMvbURyft8="},{"Path":"github.com/fatih/gomodifytags","Version":"v1.17.1-0.20250423142747-f3939df9aa3c","Sum":"h1:dDSgAjoOMp8da3egfz0t2S+t8RGOpEmEXZubcGuc0Bg="},{"Path":"github.com/fatih/structtag","Version":"v1.2.0","Sum":"h1:/OdNE99OxoI/PqaW/SuSK9uxxT3f/tcSZgon/ssNSx4="},{"Path":"github.com/fsnotify/fsnotify","Version":"v1.9.0","Sum":"h1:2Ml+OJNzbYCTzsxtv8vKSFD9PbJjmhYF14k/jKC7S9k="},{"Path":"github.com/google/go-cmp","Version":"v0.7.0","Sum":"h1:wk8382ETsv4JYUZwIsn6YpYiWiBsYLSJiTsyBybVuN8="},{"Path":"github.com/google/jsonschema-go","Version":"v0.4.2","Sum":"h1:tmrUohrwoLZZS/P3x7ex0WAVknEkBZM46iALbcqoRA8="},{"Path":"github.com/modelcontextprotocol/go-sdk","Version":"v1.4.0","Sum":"h1:u0kr8lbJc1oBcawK7Df+/ajNMpIDFE41OEPxdeTLOn8="},{"Path":"github.com/segmentio/asm","Version":"v1.2.1","Sum":"h1:DTNbBqs57ioxAD4PrArqftgypG4/qNpXoJx8TVXxPR0="},{"Path":"github.com/segmentio/encoding","Version":"v0.5.3","Sum":"h1:OjMgICtcSFuNvQCdwqMCv9Tg7lEOXGwm1J5RPQccx6w="},{"Path":"github.com/yosida95/uritemplate/v3","Version":"v3.0.2","Sum":"h1:Ed3Oyj9yrmi9087+NczuL5BwkIc4wvTb5zIM+UJPGz4="},{"Path":"golang.org/x/exp/typeparams","Version":"v0.0.0-20260312153236-7ab1446f8b90","Sum":"h1:cfW8UCYSVdPblxA7qQe3o5Iad55Vsx4BFmuGS9RNOmc="},{"Path":"golang.org/x/mod","Version":"v0.35.0","Sum":"h1:Ww1D637e6Pg+Zb2KrWfHQUnH2dQRLBQyAtpr/haaJeM="},{"Path":"golang.org/x/oauth2","Version":"v0.36.0","Sum":"h1:peZ/1z27fi9hUOFCAZaHyrpWG5lwe0RJEEEeH0ThlIs="},{"Path":"golang.org/x/sync","Version":"v0.20.0","Sum":"h1:e0PTpb7pjO8GAtTs2dQ6jYa5BWYlMuX047Dco/pItO4="},{"Path":"golang.org/x/sys","Version":"v0.43.0","Sum":"h1:Rlag2XtaFTxp19wS8MXlJwTvoh8ArU6ezoyFsMyCTNI="},{"Path":"golang.org/x/telemetry","Version":"v0.0.0-20260409153401-be6f6cb8b1fa","Sum":"h1:efT73AJZfAAUV7SOip6pWGkwJDzIGiKBZGVzHYa+ve4="},{"Path":"golang.org/x/text","Version":"v0.36.0","Sum":"h1:JfKh3XmcRPqZPKevfXVpI1wXPTqbkE5f7JA92a55Yxg="},{"Path":"golang.org/x/tools","Version":"v0.44.1-0.20260513175300-635ae9663724","Sum":"h1:rz7KAMVmPA+7ecCT/BHKLE353MUOoCAwMqlUveE8508="},{"Path":"golang.org/x/vuln","Version":"v1.1.4","Sum":"h1:Ju8QsuyhX3Hk8ma3CesTbO8vfJD9EvUBgHvkxHBzj0I="},{"Path":"honnef.co/go/tools","Version":"v0.7.0","Sum":"h1:w6WUp1VbkqPEgLz4rkBzH/CSU6HkoqNLp6GstyTx3lU="},{"Path":"mvdan.cc/gofumpt","Version":"v0.9.2","Sum":"h1:zsEMWL8SVKGHNztrx6uZrXdp7AX8r421Vvp23sz7ik4="},{"Path":"mvdan.cc/xurls/v2","Version":"v2.6.0","Sum":"h1:3NTZpeTxYVWNSokW3MKeyVkz/j7uYXYiMtXRUfmjbgI="}],"Settings":[{"Key":"-buildmode","Value":"exe"},{"Key":"-compiler","Value":"gc"},{"Key":"CGO_ENABLED","Value":"1"},{"Key":"CGO_CFLAGS"},{"Key":"CGO_CPPFLAGS"},{"Key":"CGO_CXXFLAGS"},{"Key":"CGO_LDFLAGS"},{"Key":"GOARCH","Value":"arm64"},{"Key":"GOOS","Value":"darwin"},{"Key":"GOARM64","Value":"v8.0"}],"Version":"v0.22.0"} · seed: 42 · harness: `555714da1991`
- oracle_error_rate: 0.000 · sut_error_rate: 0.000 · baseline_invalid: False · oracle_not_quiescent: False
- wall (s): {'oracle_start': 6.615, 'm1_oracle_inventory': 87.29, 'm2': 127.11, 'm3': 0.096}

## M2 callers

| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |
|---|---|---|---|---|---|---|---|---|---|
| C-method | 0.26 [0.13–0.45] | 0.21 [0.10–0.37] | 0.88 [0.53–0.98] | 0.78 [0.45–0.94] | 0.26 [0.13–0.46] | 0.21 [0.10–0.38] | 7/1/2 | 44 | 0 |
| C-name | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 1.00 [0.77–1.00] | 1.00 [0.77–1.00] | 17/0/0 | 0 | 0 |
| Q-scoped | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 1.00 [0.68–1.00] | 8/0/0 | 0 | 0 |
| U-method | 0.85 [0.74–0.92] | 0.84 [0.72–0.91] | 1.00 [0.92–1.00] | 0.98 [0.89–1.00] | 1.00 [0.85–1.00] | 0.95 [0.78–0.99] | 47/0/1 | 16 | 0 |

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
| C-method | 0.83 [0.61–0.94] | 0.88 [0.66–0.97] | 1.00 [0.80–1.00] | 1.00 [0.80–1.00] | 0.86 [0.49–0.97] | 1.00 [0.61–1.00] | 15/0/0 | 5 | 0 |
| C-name | 0.83 [0.55–0.95] | 0.59 [0.36–0.78] | 1.00 [0.72–1.00] | 1.00 [0.72–1.00] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 10/0/0 | 9 | 0 |
| Q-scoped | 1.00 [0.82–1.00] | 0.50 [0.34–0.66] | 1.00 [0.82–1.00] | 0.95 [0.75–0.99] | 0.57 [0.25–0.84] | 1.00 [0.51–1.00] | 18/0/1 | 17 | 0 |
| U-method | 0.95 [0.75–0.99] | 0.78 [0.58–0.90] | 1.00 [0.82–1.00] | 1.00 [0.82–1.00] | 0.71 [0.36–0.92] | 1.00 [0.57–1.00] | 18/0/0 | 6 | 0 |

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
[
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/dns/dns_test.go:270",
  "site_fingerprint": "c193e17636bc445e",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/eureka/eureka_test.go:59",
  "site_fingerprint": "1ad7801cb754d916",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/file/file.go:249",
  "site_fingerprint": "1def8e52da607a92",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/file/file.go:278",
  "site_fingerprint": "2d6301a702cf8a50",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/linode/linode_test.go:251",
  "site_fingerprint": "8dbae7c7fd50bee4",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/moby/nodes_test.go:59",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/moby/services_test.go:59",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/moby/services_test.go:364",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/moby/tasks_test.go:59",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/nomad/nomad_test.go:195",
  "site_fingerprint": "8dbae7c7fd50bee4",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/puppetdb/puppetdb_test.go:109",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/puppetdb/puppetdb_test.go:161",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/puppetdb/puppetdb_test.go:224",
  "site_fingerprint": "558b2568870fa3b7",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/puppetdb/puppetdb_test.go:256",
  "site_fingerprint": "6651807c423b47ab",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/refresh/refresh.go:69",
  "site_fingerprint": "1594af472460bb9e",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "self_receiver",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/refresh/refresh.go:88",
  "site_fingerprint": "7936ad624cde8556",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/uyuni/uyuni_test.go:59",
  "site_fingerprint": "1ad7801cb754d916",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/uyuni/uyuni_test.go:149",
  "site_fingerprint": "3dbe8f313b2dd727",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "return_typed",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/vultr/vultr_test.go:71",
  "site_fingerprint": "8d8ef591395b4050",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "discovery/marathon/marathon.go:229",
  "site": "discovery/marathon/marathon.go:174",
  "site_fingerprint": "15fce3bf8e2eb869"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "discovery/stackit/server.go:114",
  "site": "discovery/stackit/stackit.go:146",
  "site_fingerprint": "2de152f685eec3ff"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/benchmark_test.go:209",
  "site_fingerprint": "74487fbf58fb3c9f"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/benchmark_test.go:304",
  "site_fingerprint": "728b829ce82dce99"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/benchmark_test.go:334",
  "site_fingerprint": "bd120576f48a5375"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/interface_test.go:259",
  "site_fingerprint": "40aa3b33c7572fa8"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/nhcbparse.go:166",
  "site_fingerprint": "9fc33e0e546437c0"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "model/textparse/nhcbparse.go:321",
  "site_fingerprint": "a3c1343065d1ee48"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "scrape/scrape_append_v2.go:261",
  "site_fingerprint": "3e61a7704f9b0979"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "util/fuzzing/fuzz_test.go:327",
  "site_fingerprint": "ebd4eb914c9112b5"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "model/textparse/openmetricsparse.go:290",
  "site": "util/fuzzing/fuzz_test.go:335",
  "site_fingerprint": "95797218a2efd67a"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "prompb/io/prometheus/client/metrics.pb.go:1139",
  "site": "prompb/io/prometheus/client/metrics.pb.go:1730",
  "site_fingerprint": "3e9d13165a99e3d0"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "storage/merge.go:262",
  "site_fingerprint": "e89d0b3a87e15586"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "storage/merge_test.go:1636",
  "site_fingerprint": "8f5c33edaac183af"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "storage/secondary.go:64",
  "site_fingerprint": "07d97db1ad965a6c"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/db_append_v2_test.go:1779",
  "site_fingerprint": "c72b3ec7598eaf47"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/db_test.go:2499",
  "site_fingerprint": "c72b3ec7598eaf47"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/head_append_v2_test.go:1618",
  "site_fingerprint": "c3870789bfb75cbf"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/head_test.go:4309",
  "site_fingerprint": "c3870789bfb75cbf"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/ooo_head_read.go:595",
  "site_fingerprint": "b0ca2f094e6158e7"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "tsdb/ooo_head_read.go:669",
  "site_fingerprint": "b0ca2f094e6158e7"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "web/api/v1/api.go:813",
  "site_fingerprint": "e422ef62d6ade252"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "storage/remote/read.go:219",
  "site": "web/api/v1/api.go:835",
  "site_fingerprint": "320b6e425d6468b5"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:47",
  "site_fingerprint": "d40340de0d79267e",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:58",
  "site_fingerprint": "3c477a80a6319add",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:164",
  "site_fingerprint": "f2fb829e33822a1b",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:202",
  "site_fingerprint": "51c64c422c352114",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:310",
  "site_fingerprint": "5c4b2fcfb7aa960d",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:321",
  "site_fingerprint": "6e445b7226a22d29",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:391",
  "site_fingerprint": "68c9a9ec21bad887",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:410",
  "site_fingerprint": "a2b8058d314a0034",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:53",
  "site_fingerprint": "8b833d563937ae54"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:63",
  "site_fingerprint": "d81029253d8e1732"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:170",
  "site_fingerprint": "8b833d563937ae54"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:207",
  "site_fingerprint": "e6030ee7bdd9a8e9"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:326",
  "site_fingerprint": "26c41e3e081b83ec"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:397",
  "site_fingerprint": "8b833d563937ae54"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/testhelpers/openapi.go:98",
  "site": "web/api/v1/api_scenarios_test.go:415",
  "site_fingerprint": "e6030ee7bdd9a8e9"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callers",
  "seed_def": "web/api/v1/api.go:2064",
  "site": "web/api/v1/api.go:477",
  "site_fingerprint": "15ed316862c2067c"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "cmd/promtool/main.go:1369",
  "site": "cmd/promtool/main.go:1377",
  "site_fingerprint": "b9b39f692d0f4e70"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "cmd/promtool/main.go:1369",
  "site": "cmd/promtool/main.go:1379",
  "site_fingerprint": "94b83b8c689ad367"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "cmd/promtool/main.go:1369",
  "site": "cmd/promtool/main.go:1381",
  "site_fingerprint": "386648c76e7adeb1"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "cmd/promtool/main.go:1369",
  "site": "cmd/promtool/main.go:1383",
  "site_fingerprint": "5cc98dcd8cf609e6"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "cmd/promtool/main.go:1369",
  "site": "cmd/promtool/main.go:1410",
  "site_fingerprint": "d6769598618a6dc0"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "config/config.go:1547",
  "site": "config/config.go:1549",
  "site_fingerprint": "f58e0cd7a16d55e5"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "discovery/stackit/server.go:114",
  "site": "discovery/stackit/server.go:136",
  "site_fingerprint": "22312ef1d99faf29"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "r6_single_owner",
  "measurement": "callees",
  "seed_def": "discovery/stackit/server.go:114",
  "site": "discovery/stackit/server.go:151",
  "site_fingerprint": "3f181aad8830c8ed",
  "tier": "candidate"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "model/labels/labels_test.go:967",
  "site": "model/labels/labels_test.go:977",
  "site_fingerprint": "e4c816a2dcf6c5eb"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "prompb/io/prometheus/client/metrics.pb.go:1139",
  "site": "prompb/io/prometheus/client/metrics.pb.go:1146",
  "site_fingerprint": "1361f27583afe3dc"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "prompb/io/prometheus/client/metrics.pb.go:1139",
  "site": "prompb/io/prometheus/client/metrics.pb.go:1162",
  "site_fingerprint": "940bad2dc080122d"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "storage/buffer.go:620",
  "site": "storage/buffer.go:629",
  "site_fingerprint": "2b9659a4149a8964"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "free_single",
  "measurement": "callees",
  "seed_def": "storage/buffer.go:620",
  "site": "storage/buffer.go:630",
  "site_fingerprint": "e52a0e4bdaec0180"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "storage/buffer.go:620",
  "site": "storage/buffer.go:648",
  "site_fingerprint": "ecf07f212d2cf482"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "storage/remote/codec.go:491",
  "site": "storage/remote/codec.go:508",
  "site_fingerprint": "433b9ab2deb68952"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "storage/remote/codec.go:491",
  "site": "storage/remote/codec.go:513",
  "site_fingerprint": "d5749de58dbcbd3b"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "storage/remote/codec.go:491",
  "site": "storage/remote/codec.go:526",
  "site_fingerprint": "6d1b7dff19a56921"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "storage/remote/codec.go:491",
  "site": "storage/remote/codec.go:531",
  "site_fingerprint": "b3c446696e9d1550"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1257",
  "site_fingerprint": "e96f12d36c120973"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1262",
  "site_fingerprint": "c62c34700b8de6ee"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1268",
  "site_fingerprint": "78bbf1ca93b88c4c"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1270",
  "site_fingerprint": "aa4bcc8949b52d2e"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1284",
  "site_fingerprint": "c62c34700b8de6ee"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1290",
  "site_fingerprint": "746e3a138c1e72c0"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1292",
  "site_fingerprint": "317bb9a93656adfa"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1301",
  "site_fingerprint": "e96f12d36c120973"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1306",
  "site_fingerprint": "c62c34700b8de6ee"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1312",
  "site_fingerprint": "53bb7a9cedacc966"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/chunkenc/float_histogram_test.go:1252",
  "site": "tsdb/chunkenc/float_histogram_test.go:1314",
  "site_fingerprint": "347cba9b891eb6cd"
 },
 {
  "corpus": "prometheus",
  "direction": "prism_only",
  "dispatch_kind": "interface_dispatch",
  "measurement": "callees",
  "seed_def": "tsdb/head_read.go:559",
  "site": "tsdb/head_read.go:590",
  "site_fingerprint": "b41c3cc937a42fda"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/head_read.go:559",
  "site": "tsdb/head_read.go:578",
  "site_fingerprint": "4af5f1465bb9bb3a"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:54",
  "site_fingerprint": "1f72607669ea104d"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:68",
  "site_fingerprint": "f6f191e822457d00"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:81",
  "site_fingerprint": "a32c46b3c7d6ced7"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:92",
  "site_fingerprint": "c763f7f7b004d4b5"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:96",
  "site_fingerprint": "969fe83cbdb68fca"
 },
 {
  "corpus": "prometheus",
  "direction": "oracle_only",
  "measurement": "callees",
  "seed_def": "tsdb/record/bench_test.go:36",
  "site": "tsdb/record/bench_test.go:102",
  "site_fingerprint": "35d781224257cfe6"
 }
]
```
