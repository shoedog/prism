# Tier-A base-vs-candidate comparison

**Verdict:** `NOT_COMPARABLE(corpus_sha_full; sha_aliases; closure_digest; policy_digest; population_digest; snapshot_digest; data; env_values; harness_sha; probe_set)`

## Runs

```json
{
  "base": {
    "digest": "sha256:0ff8a3c880e30766aa2623077f9f00d5a190c58617ee757d8b1bb799f944a5d4",
    "path": "docs/eval/tier-a/2026-10-30-prism-db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2-db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2.json",
    "path_base": "repo_root"
  },
  "candidate": {
    "digest": "sha256:524d4ec98251313a71fb904cad91a18e3b96952e75c28f8fe998a47246a54326",
    "path": "docs/eval/tier-a/2026-11-07-prism-2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1-2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1.json",
    "path_base": "repo_root"
  }
}
```

## Generation metadata

```json
{
  "generated_at": "2026-11-08T05:19:12.967621+00:00",
  "harness_sha": "2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1"
}
```

## Identities

```json
{
  "base_sha": "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2",
  "base_sut_digest": "sha256:089518a77144ad823a851a919cc393dd7e5d9c00bf326a9556ced47ac3130585",
  "candidate_sha": "2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1",
  "candidate_sut_digest": "sha256:40af684f1917a9079bfc67dd00e98cf40de1f81bceb246cacd5771c4670d1d24",
  "closure_digest": "sha256:bc43b0907c3b9a7f982fcf1449760ec35ff84382eaf1449ed306c0dc2860e2c3",
  "corpus_sha_full": "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2",
  "data": {
    "adjudications": {
      "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
      "path": "eval/adjudications.jsonl",
      "records": 2324
    },
    "fixtures": "sha256:f8bfdb4c93bd5a52986125edd15b1692586cd0e64ddac6eda6098be569597e8e",
    "pinned_probes": "sha256:7bf76644bd62a7e5dfdf92fee0a0696943bc6a790c91dba83150486d9ff24437"
  },
  "env_allow": [
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "TMPDIR",
    "LANG",
    "TIER_A_ROOT"
  ],
  "env_values": {
    "HOME": "/Users/wesleyjinks",
    "LANG": "C.UTF-8",
    "PATH": "/Users/wesleyjinks/code/slicing-tiera/eval/.venv/bin:/Users/wesleyjinks/.local/share/mise/installs/node/lts/bin:/Users/wesleyjinks/.local/share/mise/installs/go/1.26.2/bin:/Users/wesleyjinks/.bun/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.local/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/pkg/env/global/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/Users/wesleyjinks/.bun/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.local/bin:/Users/wesleyjinks/.cargo/bin:/Users/wesleyjinks/Library/Application Support/JetBrains/Toolbox/scripts:/Users/wesleyjinks/.orbstack/bin:/Users/wesleyjinks/Library/Application Support/JetBrains/Toolbox/scripts:/Users/wesleyjinks/.orbstack/bin",
    "TMPDIR": "/var/folders/mq/jvvlwk513zq0v6xsh8bjm7280000gn/T/"
  },
  "harness_sha": "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2",
  "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
  "oracle_init": {
    "cargo": {
      "features": "all"
    }
  },
  "policy_digest": "sha256:402d23439353842a958958953d3817612a70b52642c376cd6c261b3977a2a572",
  "population_digest": "sha256:8853ee89bc1148378750546b916b33f3499a0bfef33c1e8ec5e09fed2f6a64dd",
  "runtime": {
    "cargo": "cargo 1.94.0 (85eff7c80 2026-01-15)",
    "cargo_config": "absent",
    "python": "3.12.13",
    "python_version_file": "sha256:7b55f8e67b5623c4bef3fa691288da9437d79d3aba156de48d481db32ac7d16d",
    "rust_toolchain": "absent",
    "rustc": "rustc 1.94.0 (4a4ef493e 2026-03-02)",
    "uv_lock": "sha256:81a456c78e9970513959a341e711e989ab92b633af871a495a90fec1579ae085"
  },
  "sample_digest": {
    "full": "sha256:855512232fd6f3bf0b65dca9b6055d8f375742fb565e49ddb5e361b17459e36f",
    "quick": "sha256:07334dcd56d8cf203e868f5ee3ceafc545fe31c4a9fc35976d6ceabc3452b003"
  },
  "seed": 42,
  "sha_aliases": [
    "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2"
  ],
  "snapshot_digest": "sha256:4002324b536b6456d83c9eb0333e2ae36a50f6d7b3bc0405e04a94cbe6c86403"
}
```

## Identity legs

```json
{
  "base": {
    "closure_digest": "sha256:bc43b0907c3b9a7f982fcf1449760ec35ff84382eaf1449ed306c0dc2860e2c3",
    "corpus_sha_full": "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2",
    "data": {
      "adjudications": {
        "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
        "path": "eval/adjudications.jsonl",
        "records": 2324
      },
      "fixtures": "sha256:f8bfdb4c93bd5a52986125edd15b1692586cd0e64ddac6eda6098be569597e8e",
      "pinned_probes": "sha256:7bf76644bd62a7e5dfdf92fee0a0696943bc6a790c91dba83150486d9ff24437"
    },
    "env_allow": [
      "PATH",
      "HOME",
      "CARGO_HOME",
      "RUSTUP_HOME",
      "TMPDIR",
      "LANG",
      "TIER_A_ROOT"
    ],
    "env_values": {
      "HOME": "/Users/wesleyjinks",
      "LANG": "C.UTF-8",
      "PATH": "/Users/wesleyjinks/code/slicing-tiera/eval/.venv/bin:/Users/wesleyjinks/.local/share/mise/installs/node/lts/bin:/Users/wesleyjinks/.local/share/mise/installs/go/1.26.2/bin:/Users/wesleyjinks/.bun/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.local/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/pkg/env/global/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/Users/wesleyjinks/.bun/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.local/bin:/Users/wesleyjinks/.cargo/bin:/Users/wesleyjinks/Library/Application Support/JetBrains/Toolbox/scripts:/Users/wesleyjinks/.orbstack/bin:/Users/wesleyjinks/Library/Application Support/JetBrains/Toolbox/scripts:/Users/wesleyjinks/.orbstack/bin",
      "TMPDIR": "/var/folders/mq/jvvlwk513zq0v6xsh8bjm7280000gn/T/"
    },
    "harness_sha": "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2",
    "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
    "oracle_init": {
      "cargo": {
        "features": "all"
      }
    },
    "policy_digest": "sha256:402d23439353842a958958953d3817612a70b52642c376cd6c261b3977a2a572",
    "population_digest": "sha256:8853ee89bc1148378750546b916b33f3499a0bfef33c1e8ec5e09fed2f6a64dd",
    "runtime": {
      "cargo": "cargo 1.94.0 (85eff7c80 2026-01-15)",
      "cargo_config": "absent",
      "python": "3.12.13",
      "python_version_file": "sha256:7b55f8e67b5623c4bef3fa691288da9437d79d3aba156de48d481db32ac7d16d",
      "rust_toolchain": "absent",
      "rustc": "rustc 1.94.0 (4a4ef493e 2026-03-02)",
      "uv_lock": "sha256:81a456c78e9970513959a341e711e989ab92b633af871a495a90fec1579ae085"
    },
    "sample_digest": {
      "full": "sha256:855512232fd6f3bf0b65dca9b6055d8f375742fb565e49ddb5e361b17459e36f",
      "quick": "sha256:07334dcd56d8cf203e868f5ee3ceafc545fe31c4a9fc35976d6ceabc3452b003"
    },
    "seed": 42,
    "sha_aliases": [
      "db066ef3762eb3b94a8c6d7b84ebb83b4fe694b2"
    ],
    "snapshot_digest": "sha256:4002324b536b6456d83c9eb0333e2ae36a50f6d7b3bc0405e04a94cbe6c86403"
  },
  "candidate": {
    "closure_digest": "sha256:8c44cbe491295dd24e33ef70f89ae6b029fdfa4346ad3af0c1f7afb628f999bf",
    "corpus_sha_full": "2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1",
    "data": {
      "adjudications": {
        "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
        "path": "eval/adjudications.jsonl",
        "records": 2324
      },
      "fixtures": "sha256:6b54665e752c135af2639a8b564952a060f6ad3e09bcbec4b47e3f46695e0129",
      "pinned_probes": "sha256:05d762f00bc641dcceaf38233d9bddd79c5fd14afb933b318f9709c329c9838e"
    },
    "env_allow": [
      "PATH",
      "HOME",
      "CARGO_HOME",
      "RUSTUP_HOME",
      "TMPDIR",
      "LANG",
      "TIER_A_ROOT"
    ],
    "env_values": {
      "HOME": "/Users/wesleyjinks",
      "LANG": "C.UTF-8",
      "PATH": "/Users/wesleyjinks/code/prism-717/eval/.venv/bin:/Users/wesleyjinks/.local/share/mise/installs/node/lts/bin:/Users/wesleyjinks/.local/share/mise/installs/go/1.26.2/bin:/Users/wesleyjinks/.bun/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.local/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/pkg/env/global/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/Users/wesleyjinks/.local/bin:/Users/wesleyjinks/bin:/Users/wesleyjinks/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/bin:/Users/wesleyjinks/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin:/opt/local/bin:/Users/wesleyjinks/.cargo/bin:/Users/wesleyjinks/Library/Application Support/JetBrains/Toolbox/scripts:/Users/wesleyjinks/.orbstack/bin",
      "TMPDIR": "/var/folders/mq/jvvlwk513zq0v6xsh8bjm7280000gn/T/"
    },
    "harness_sha": "2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1",
    "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
    "oracle_init": {
      "cargo": {
        "features": "all"
      }
    },
    "policy_digest": "sha256:afda887f9d874f092814acfc937297cd7d9cc5ed693a868cfe758341ac547982",
    "population_digest": "sha256:b1cfdce6c7529c87a93bd89b7c351a902ab262ac36ce1c22b4aac6b2a30c9a0c",
    "runtime": {
      "cargo": "cargo 1.94.0 (85eff7c80 2026-01-15)",
      "cargo_config": "absent",
      "python": "3.12.13",
      "python_version_file": "sha256:7b55f8e67b5623c4bef3fa691288da9437d79d3aba156de48d481db32ac7d16d",
      "rust_toolchain": "absent",
      "rustc": "rustc 1.94.0 (4a4ef493e 2026-03-02)",
      "uv_lock": "sha256:81a456c78e9970513959a341e711e989ab92b633af871a495a90fec1579ae085"
    },
    "sample_digest": {
      "full": "sha256:855512232fd6f3bf0b65dca9b6055d8f375742fb565e49ddb5e361b17459e36f",
      "quick": "sha256:07334dcd56d8cf203e868f5ee3ceafc545fe31c4a9fc35976d6ceabc3452b003"
    },
    "seed": 42,
    "sha_aliases": [
      "2b0e2627057415dbb6a667e2e3a5f3f66a6acfe1"
    ],
    "snapshot_digest": "sha256:16f8d51dfceb7110fa975dbf5f60e6c61c47525faa33e57e2743b6fde242e267"
  }
}
```

## Comparison policy

```json
{
  "min_comparable_per_stratum": 4,
  "oracle_changed_max_fraction": 0.1
}
```

## Criteria

- [x] `both_admitted`
- [x] `both_baseline_valid`
- [ ] `identities_equal`
- [ ] `probe_sets_equal`
- [x] `adjudications_match`
- [x] `adjudications_digest_keys`
- [ ] `oracle_answers_joined`
- [x] `oracle_changed_within_threshold`
- [x] `no_new_sut_errors`
- [x] `unexplained_zero`
- [x] `no_recall_loss`
- [x] `no_lost_matched_oracle_sites`
- [x] `matrix_no_regression`
- [x] `pinned_no_regression`

## Probe classes

```json
{}
```

## Class counts

```json
{
  "carried_changed/content_change": 0,
  "carried_changed/sut_change": 0,
  "carried_same": 0,
  "oracle_changed": 0,
  "resampled": 0,
  "unexplained": 0
}
```

## Oracle changed summary (excluded)

```json
{
  "count": 0,
  "fraction": 0.0,
  "ids": []
}
```

## Oracle changed answers

```json
{}
```

## Pinned oracle exclusions

```json
[]
```

## Pinned outcome differences

```json
{}
```

## Matrix differences

```json
{}
```

## Status transitions

```json
{}
```

## Lost matched oracle sites

```json
{}
```

## Corrected measured metrics

```json
{
  "base": {},
  "candidate": {}
}
```

## Raw measured metrics

```json
{
  "base": {},
  "candidate": {}
}
```

## Measured metric rows

```json
{
  "base": {},
  "candidate": {}
}
```

## Full-sample metrics (appendix)

```json
{
  "base": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 7,
          "oracle_confirmed": 5,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.6755924316585924,
            1.0
          ],
          "recall": [
            1.0,
            0.6755924316585924,
            1.0
          ],
          "tp": 8
        },
        "exact_tier": {
          "raw": {
            "fn": 9,
            "fp": 0,
            "precision": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "recall": [
              0.25,
              0.08894166769488562,
              0.5323053370816633
            ],
            "tp": 3
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.8571428571428571,
              0.486872167860775,
              0.9743203759421759
            ],
            "recall": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "tp": 6
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.8571428571428571,
              0.486872167860775,
              0.9743203759421759
            ],
            "recall": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "tp": 6
          }
        },
        "pending": 6,
        "raw": {
          "fn": 4,
          "fp": 2,
          "precision": [
            0.8,
            0.4901624691221323,
            0.9433178490622331
          ],
          "recall": [
            0.6666666666666666,
            0.3906220869404939,
            0.86187990987514
          ],
          "tp": 8
        },
        "shortfall": 0
      },
      "C-name": {
        "candidate_tier": {
          "count": 9,
          "oracle_confirmed": 5,
          "oracle_unconfirmed": 4
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.862023793448672,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.862023793448672,
            0.9999999999999999
          ],
          "tp": 24
        },
        "exact_tier": {
          "raw": {
            "fn": 6,
            "fp": 19,
            "precision": [
              0.5128205128205128,
              0.3619936277532354,
              0.6613482477597057
            ],
            "recall": [
              0.7692307692307693,
              0.5794844576151241,
              0.889661506584867
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.5714285714285714,
              0.25045836275303557,
              0.8417801460075546
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          },
          "raw": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.5714285714285714,
              0.25045836275303557,
              0.8417801460075546
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 25,
        "raw": {
          "fn": 2,
          "fp": 23,
          "precision": [
            0.5106382978723404,
            0.37242453646853413,
            0.6472444506534676
          ],
          "recall": [
            0.9230769230769231,
            0.7585844605936842,
            0.9786449117205875
          ],
          "tp": 24
        },
        "shortfall": 0
      },
      "Q-scoped": {
        "candidate_tier": {
          "count": 5,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 5
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8063923170025951,
            1.0
          ],
          "recall": [
            1.0,
            0.8063923170025951,
            1.0
          ],
          "tp": 16
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 6,
            "precision": [
              0.7272727272727273,
              0.5184826582032107,
              0.8684923978853596
            ],
            "recall": [
              1.0,
              0.8063923170025951,
              1.0
            ],
            "tp": 16
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.875,
              0.5291118149439162,
              0.9775825088000281
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.875,
              0.5291118149439162,
              0.9775825088000281
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          }
        },
        "pending": 10,
        "raw": {
          "fn": 0,
          "fp": 10,
          "precision": [
            0.6153846153846154,
            0.42534849943440245,
            0.775714056651308
          ],
          "recall": [
            1.0,
            0.8063923170025951,
            1.0
          ],
          "tp": 16
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 9,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 5
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.92292643579224,
            1.0
          ],
          "recall": [
            1.0,
            0.92292643579224,
            1.0
          ],
          "tp": 46
        },
        "exact_tier": {
          "raw": {
            "fn": 3,
            "fp": 13,
            "precision": [
              0.7678571428571429,
              0.6423184667761581,
              0.8590062094739521
            ],
            "recall": [
              0.9347826086956522,
              0.824972588030455,
              0.9775721387454057
            ],
            "tp": 43
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 5,
            "precision": [
              0.375,
              0.13684428473912286,
              0.694257607346229
            ],
            "recall": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "tp": 3
          },
          "raw": {
            "fn": 0,
            "fp": 5,
            "precision": [
              0.375,
              0.13684428473912286,
              0.694257607346229
            ],
            "recall": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "tp": 3
          }
        },
        "pending": 18,
        "raw": {
          "fn": 0,
          "fp": 18,
          "precision": [
            0.71875,
            0.5986606961815428,
            0.8140662772904212
          ],
          "recall": [
            1.0,
            0.92292643579224,
            1.0
          ],
          "tp": 46
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 3,
          "oracle_confirmed": 2,
          "oracle_unconfirmed": 1
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.7575059904469021,
            1.0
          ],
          "recall": [
            1.0,
            0.7575059904469021,
            1.0
          ],
          "tp": 12
        },
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.7575059904469021,
              1.0
            ],
            "recall": [
              0.9230769230769231,
              0.666860488286395,
              0.986289578909774
            ],
            "tp": 12
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 2,
            "precision": [
              0.6666666666666666,
              0.29999331299474324,
              0.9032285897864654
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          },
          "raw": {
            "fn": 0,
            "fp": 2,
            "precision": [
              0.6666666666666666,
              0.29999331299474324,
              0.9032285897864654
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 4,
        "raw": {
          "fn": 1,
          "fp": 3,
          "precision": [
            0.8,
            0.5481455107594471,
            0.9295245070826608
          ],
          "recall": [
            0.9230769230769231,
            0.666860488286395,
            0.986289578909774
          ],
          "tp": 12
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
        "candidate_tier": {
          "count": 7,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 6
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.862023793448672,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.862023793448672,
            0.9999999999999999
          ],
          "tp": 24
        },
        "exact_tier": {
          "raw": {
            "fn": 44,
            "fp": 0,
            "precision": [
              1.0,
              0.8568833796149266,
              1.0
            ],
            "recall": [
              0.34328358208955223,
              0.24089006202229762,
              0.46267335505543067
            ],
            "tp": 23
          }
        },
        "function": {
          "corrected": {
            "fn": 27,
            "fp": 6,
            "precision": [
              0.75,
              0.5510055583116656,
              0.8800063384126703
            ],
            "recall": [
              0.4,
              0.27024886476102095,
              0.5454814552141962
            ],
            "tp": 18
          },
          "raw": {
            "fn": 27,
            "fp": 6,
            "precision": [
              0.75,
              0.5510055583116656,
              0.8800063384126703
            ],
            "recall": [
              0.4,
              0.27024886476102095,
              0.5454814552141962
            ],
            "tp": 18
          }
        },
        "pending": 49,
        "raw": {
          "fn": 43,
          "fp": 6,
          "precision": [
            0.8,
            0.6269430343619583,
            0.9049489287811706
          ],
          "recall": [
            0.3582089552238806,
            0.25398057003977614,
            0.4778149025543589
          ],
          "tp": 24
        },
        "shortfall": 0
      },
      "C-name": {
        "candidate_tier": {
          "count": 54,
          "oracle_confirmed": 5,
          "oracle_unconfirmed": 49
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8754449685379289,
            1.0
          ],
          "recall": [
            1.0,
            0.8754449685379289,
            1.0
          ],
          "tp": 27
        },
        "exact_tier": {
          "raw": {
            "fn": 6,
            "fp": 0,
            "precision": [
              1.0,
              0.8513451233948545,
              1.0
            ],
            "recall": [
              0.7857142857142857,
              0.6046141419113311,
              0.8978753972661142
            ],
            "tp": 22
          }
        },
        "function": {
          "corrected": {
            "fn": 1,
            "fp": 36,
            "precision": [
              0.4,
              0.28569490454383006,
              0.5263394660630154
            ],
            "recall": [
              0.96,
              0.8045593608838223,
              0.9929037665766179
            ],
            "tp": 24
          },
          "raw": {
            "fn": 1,
            "fp": 36,
            "precision": [
              0.4,
              0.28569490454383006,
              0.5263394660630154
            ],
            "recall": [
              0.96,
              0.8045593608838223,
              0.9929037665766179
            ],
            "tp": 24
          }
        },
        "pending": 50,
        "raw": {
          "fn": 1,
          "fp": 49,
          "precision": [
            0.35526315789473684,
            0.25701778697151323,
            0.4674361457564893
          ],
          "recall": [
            0.9642857142857143,
            0.8228780209422653,
            0.9936674802210836
          ],
          "tp": 27
        },
        "shortfall": 0
      },
      "Q-scoped": {
        "candidate_tier": {
          "count": 0,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "recall": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "tp": 7
        },
        "exact_tier": {
          "raw": {
            "fn": 16,
            "fp": 0,
            "precision": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "recall": [
              0.30434782608695654,
              0.1560402436889323,
              0.5086575642878356
            ],
            "tp": 7
          }
        },
        "function": {
          "corrected": {
            "fn": 12,
            "fp": 0,
            "precision": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "recall": [
              0.3333333333333333,
              0.16278774656229916,
              0.5625053287487956
            ],
            "tp": 6
          },
          "raw": {
            "fn": 12,
            "fp": 0,
            "precision": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "recall": [
              0.3333333333333333,
              0.16278774656229916,
              0.5625053287487956
            ],
            "tp": 6
          }
        },
        "pending": 16,
        "raw": {
          "fn": 16,
          "fp": 0,
          "precision": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "recall": [
            0.30434782608695654,
            0.1560402436889323,
            0.5086575642878356
          ],
          "tp": 7
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 0,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8984854444077807,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.8984854444077807,
            0.9999999999999999
          ],
          "tp": 34
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.8984854444077807,
              0.9999999999999999
            ],
            "recall": [
              1.0,
              0.8984854444077807,
              0.9999999999999999
            ],
            "tp": 34
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.7411670300055737,
              1.0
            ],
            "recall": [
              1.0,
              0.7411670300055737,
              1.0
            ],
            "tp": 11
          },
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.7411670300055737,
              1.0
            ],
            "recall": [
              1.0,
              0.7411670300055737,
              1.0
            ],
            "tp": 11
          }
        },
        "pending": 0,
        "raw": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8984854444077807,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.8984854444077807,
            0.9999999999999999
          ],
          "tp": 34
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 10,
          "oracle_confirmed": 5,
          "oracle_unconfirmed": 5
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8063923170025951,
            1.0
          ],
          "recall": [
            1.0,
            0.8063923170025951,
            1.0
          ],
          "tp": 16
        },
        "exact_tier": {
          "raw": {
            "fn": 21,
            "fp": 0,
            "precision": [
              1.0,
              0.7411670300055737,
              1.0
            ],
            "recall": [
              0.34375,
              0.20410437836619277,
              0.5168891215602698
            ],
            "tp": 11
          }
        },
        "function": {
          "corrected": {
            "fn": 12,
            "fp": 3,
            "precision": [
              0.85,
              0.6395811333571357,
              0.9476312545132735
            ],
            "recall": [
              0.5862068965517241,
              0.40737835855877247,
              0.7448682276258509
            ],
            "tp": 17
          },
          "raw": {
            "fn": 12,
            "fp": 3,
            "precision": [
              0.85,
              0.6395811333571357,
              0.9476312545132735
            ],
            "recall": [
              0.5862068965517241,
              0.40737835855877247,
              0.7448682276258509
            ],
            "tp": 17
          }
        },
        "pending": 21,
        "raw": {
          "fn": 16,
          "fp": 5,
          "precision": [
            0.7619047619047619,
            0.5490882614178733,
            0.8937198704460038
          ],
          "recall": [
            0.5,
            0.33630882754048685,
            0.6636911724595131
          ],
          "tp": 16
        },
        "shortfall": 0
      }
    }
  },
  "candidate": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 2,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "recall": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "tp": 7
        },
        "exact_tier": {
          "raw": {
            "fn": 2,
            "fp": 0,
            "precision": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "recall": [
              0.7142857142857143,
              0.3589344495310541,
              0.9177810767507166
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "recall": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "tp": 5
          },
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "recall": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "tp": 5
          }
        },
        "pending": 2,
        "raw": {
          "fn": 0,
          "fp": 2,
          "precision": [
            0.7777777777777778,
            0.4525889666922117,
            0.9367748934594303
          ],
          "recall": [
            1.0,
            0.6456695613241317,
            1.0
          ],
          "tp": 7
        },
        "shortfall": 0
      },
      "C-name": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 3,
          "oracle_unconfirmed": 1
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.871271076371412,
            1.0
          ],
          "recall": [
            1.0,
            0.871271076371412,
            1.0
          ],
          "tp": 26
        },
        "exact_tier": {
          "raw": {
            "fn": 4,
            "fp": 0,
            "precision": [
              1.0,
              0.871271076371412,
              1.0
            ],
            "recall": [
              0.8666666666666667,
              0.7031867316460598,
              0.9469034455288754
            ],
            "tp": 26
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.875,
              0.5291118149439162,
              0.9775825088000281
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.875,
              0.5291118149439162,
              0.9775825088000281
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          }
        },
        "pending": 6,
        "raw": {
          "fn": 4,
          "fp": 2,
          "precision": [
            0.9285714285714286,
            0.7735463727026133,
            0.9801879360635548
          ],
          "recall": [
            0.8666666666666667,
            0.7031867316460598,
            0.9469034455288754
          ],
          "tp": 26
        },
        "shortfall": 0
      },
      "Q-scoped": {
        "candidate_tier": {
          "count": 2,
          "oracle_confirmed": 2,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "recall": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "tp": 22
        },
        "exact_tier": {
          "raw": {
            "fn": 2,
            "fp": 1,
            "precision": [
              0.9523809523809523,
              0.7733063972991618,
              0.9915440122838988
            ],
            "recall": [
              0.9090909090909091,
              0.721850529721749,
              0.9747045712376774
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          },
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "recall": [
              1.0,
              0.6456695613241317,
              1.0
            ],
            "tp": 7
          }
        },
        "pending": 1,
        "raw": {
          "fn": 0,
          "fp": 1,
          "precision": [
            0.9565217391304348,
            0.7900884474454931,
            0.9922833339420488
          ],
          "recall": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "tp": 22
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 6,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.9059421728185313,
            1.0
          ],
          "recall": [
            1.0,
            0.9059421728185313,
            1.0
          ],
          "tp": 37
        },
        "exact_tier": {
          "raw": {
            "fn": 3,
            "fp": 0,
            "precision": [
              1.0,
              0.901099006269668,
              1.0
            ],
            "recall": [
              0.9210526315789473,
              0.7920062841024803,
              0.9727854047051169
            ],
            "tp": 35
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 2,
            "precision": [
              0.75,
              0.40927542792725174,
              0.9285207879020445
            ],
            "recall": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "tp": 6
          },
          "raw": {
            "fn": 0,
            "fp": 2,
            "precision": [
              0.75,
              0.40927542792725174,
              0.9285207879020445
            ],
            "recall": [
              1.0,
              0.6096657083436265,
              1.0
            ],
            "tp": 6
          }
        },
        "pending": 3,
        "raw": {
          "fn": 1,
          "fp": 2,
          "precision": [
            0.9487179487179487,
            0.8311435982403941,
            0.9858220447125474
          ],
          "recall": [
            0.9736842105263158,
            0.8650512375427468,
            0.9953394123657999
          ],
          "tp": 37
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 3
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.6096657083436265,
            1.0
          ],
          "recall": [
            1.0,
            0.6096657083436265,
            1.0
          ],
          "tp": 6
        },
        "exact_tier": {
          "raw": {
            "fn": 56,
            "fp": 0,
            "precision": [
              1.0,
              0.5655175313406071,
              1.0
            ],
            "recall": [
              0.08196721311475409,
              0.0355195663820707,
              0.17794661898916364
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 41,
            "fp": 3,
            "precision": [
              0.625,
              0.30574239265377096,
              0.8631557152608771
            ],
            "recall": [
              0.10869565217391304,
              0.04733145828147013,
              0.23037828762025514
            ],
            "tp": 5
          },
          "raw": {
            "fn": 41,
            "fp": 3,
            "precision": [
              0.625,
              0.30574239265377096,
              0.8631557152608771
            ],
            "recall": [
              0.10869565217391304,
              0.04733145828147013,
              0.23037828762025514
            ],
            "tp": 5
          }
        },
        "pending": 58,
        "raw": {
          "fn": 55,
          "fp": 3,
          "precision": [
            0.6666666666666666,
            0.35420213352505864,
            0.8794161825659264
          ],
          "recall": [
            0.09836065573770492,
            0.04586457554741866,
            0.19844607314259083
          ],
          "tp": 6
        },
        "shortfall": 0
      },
      "C-name": {
        "candidate_tier": {
          "count": 34,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 34
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "tp": 45
        },
        "exact_tier": {
          "raw": {
            "fn": 2,
            "fp": 0,
            "precision": [
              1.0,
              0.921348400123914,
              0.9999999999999999
            ],
            "recall": [
              0.9574468085106383,
              0.8575147221070161,
              0.9882517241390656
            ],
            "tp": 45
          }
        },
        "function": {
          "corrected": {
            "fn": 2,
            "fp": 6,
            "precision": [
              0.875,
              0.7529954159822886,
              0.9414294859563513
            ],
            "recall": [
              0.9545454545454546,
              0.8486500144514563,
              0.9874448901635516
            ],
            "tp": 42
          },
          "raw": {
            "fn": 2,
            "fp": 6,
            "precision": [
              0.875,
              0.7529954159822886,
              0.9414294859563513
            ],
            "recall": [
              0.9545454545454546,
              0.8486500144514563,
              0.9874448901635516
            ],
            "tp": 42
          }
        },
        "pending": 36,
        "raw": {
          "fn": 2,
          "fp": 34,
          "precision": [
            0.569620253164557,
            0.45972198883382454,
            0.6730617664083661
          ],
          "recall": [
            0.9574468085106383,
            0.8575147221070161,
            0.9882517241390656
          ],
          "tp": 45
        },
        "shortfall": 0
      },
      "Q-scoped": {
        "candidate_tier": {
          "count": 0,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.43850296436068037,
            1.0
          ],
          "recall": [
            1.0,
            0.43850296436068037,
            1.0
          ],
          "tp": 3
        },
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "recall": [
              0.75,
              0.3006418400969081,
              0.954412739704631
            ],
            "tp": 3
          }
        },
        "function": {
          "corrected": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "recall": [
              0.75,
              0.3006418400969081,
              0.954412739704631
            ],
            "tp": 3
          },
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.43850296436068037,
              1.0
            ],
            "recall": [
              0.75,
              0.3006418400969081,
              0.954412739704631
            ],
            "tp": 3
          }
        },
        "pending": 1,
        "raw": {
          "fn": 1,
          "fp": 0,
          "precision": [
            1.0,
            0.43850296436068037,
            1.0
          ],
          "recall": [
            0.75,
            0.3006418400969081,
            0.954412739704631
          ],
          "tp": 3
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 76,
          "oracle_confirmed": 36,
          "oracle_unconfirmed": 40
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.9388706285678009,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.9388706285678009,
            0.9999999999999999
          ],
          "tp": 59
        },
        "exact_tier": {
          "raw": {
            "fn": 36,
            "fp": 0,
            "precision": [
              1.0,
              0.8568833796149266,
              1.0
            ],
            "recall": [
              0.3898305084745763,
              0.27579361754225085,
              0.5173365829428099
            ],
            "tp": 23
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 38,
            "precision": [
              0.5913978494623656,
              0.48979959872098766,
              0.6857450512293798
            ],
            "recall": [
              1.0,
              0.9347150979202339,
              1.0
            ],
            "tp": 55
          },
          "raw": {
            "fn": 0,
            "fp": 38,
            "precision": [
              0.5913978494623656,
              0.48979959872098766,
              0.6857450512293798
            ],
            "recall": [
              1.0,
              0.9347150979202339,
              1.0
            ],
            "tp": 55
          }
        },
        "pending": 40,
        "raw": {
          "fn": 0,
          "fp": 40,
          "precision": [
            0.5959595959595959,
            0.4974689610496835,
            0.6872814324849962
          ],
          "recall": [
            1.0,
            0.9388706285678009,
            0.9999999999999999
          ],
          "tp": 59
        },
        "shortfall": 0
      }
    }
  }
}
```

## Wilson flags (descriptive)

```json
[]
```

## Recall regressions

```json
[]
```
