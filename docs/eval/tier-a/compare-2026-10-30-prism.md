# Tier-A base-vs-candidate comparison

**Verdict:** `NO_REGRESSION`

## Runs

```json
{
  "base": {
    "digest": "sha256:38361d2f7304ffaf35776829ef694eb5a4220409ed35961b62d9b82ce4b737ab",
    "path": "eval/runs/2026-10-30-prism-mbase.json",
    "path_base": "repo_root"
  },
  "candidate": {
    "digest": "sha256:1aeaac8cd7832f0398440bbd213ab7b71506aaeaf52579d5a6daf7102c51f641",
    "path": "eval/runs/2026-10-30-prism-mcand.json",
    "path_base": "repo_root"
  }
}
```

## Generation metadata

```json
{
  "generated_at": "2026-10-30T08:07:01.620132+00:00",
  "harness_sha": "84383d0ff077ee58b63e16401c3be2fe0805b003"
}
```

## Identities

```json
{
  "base_sha": "289e4c1fa4c2406bd687091cda8f6938033aef0e",
  "base_sut_digest": "sha256:1f5a79e1699df8f4e575cc79acb95e33235bd94945ee34d96e674b4f3264732b",
  "candidate_sha": "84383d0ff077ee58b63e16401c3be2fe0805b003",
  "candidate_sut_digest": "sha256:6aa934faa7c7d037595000f2a224163292b56c2ea9aff31ca388ee0be5d7896b",
  "closure_digest": "sha256:eb4125caabc455dbdaf3213f621ae16f53220e262ec7046303aa06cc735b02df",
  "corpus_sha_full": "289e4c1fa4c2406bd687091cda8f6938033aef0e",
  "data": {
    "adjudications": {
      "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
      "path": "eval/adjudications.jsonl",
      "records": 2324
    },
    "fixtures": "sha256:f8bfdb4c93bd5a52986125edd15b1692586cd0e64ddac6eda6098be569597e8e",
    "pinned_probes": "sha256:c03b7180a7d95bc9b9f20f38a1a1d5f7c790403bc8f8c06c3c1afbe8feec1fa7"
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
  "harness_sha": "84383d0ff077ee58b63e16401c3be2fe0805b003",
  "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
  "oracle_init": {
    "cargo": {
      "features": "all"
    }
  },
  "policy_digest": "sha256:52c1f89b5310a480d050ba590e70f98a8a7a9e1dda270c2225f28b74abeca685",
  "population_digest": "sha256:921dca8858bc6a7e13eab0b54ec75e7301035dcba19ac41750ce4fd1645dce45",
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
    "289e4c1fa4c2406bd687091cda8f6938033aef0e"
  ],
  "snapshot_digest": "sha256:a64826a869c1dd5d68c384976a1318c057ef238312d804f0a4646f249cec9ea0"
}
```

## Identity legs

```json
{
  "base": {
    "closure_digest": "sha256:eb4125caabc455dbdaf3213f621ae16f53220e262ec7046303aa06cc735b02df",
    "corpus_sha_full": "289e4c1fa4c2406bd687091cda8f6938033aef0e",
    "data": {
      "adjudications": {
        "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
        "path": "eval/adjudications.jsonl",
        "records": 2324
      },
      "fixtures": "sha256:f8bfdb4c93bd5a52986125edd15b1692586cd0e64ddac6eda6098be569597e8e",
      "pinned_probes": "sha256:c03b7180a7d95bc9b9f20f38a1a1d5f7c790403bc8f8c06c3c1afbe8feec1fa7"
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
    "harness_sha": "84383d0ff077ee58b63e16401c3be2fe0805b003",
    "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
    "oracle_init": {
      "cargo": {
        "features": "all"
      }
    },
    "policy_digest": "sha256:52c1f89b5310a480d050ba590e70f98a8a7a9e1dda270c2225f28b74abeca685",
    "population_digest": "sha256:921dca8858bc6a7e13eab0b54ec75e7301035dcba19ac41750ce4fd1645dce45",
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
      "289e4c1fa4c2406bd687091cda8f6938033aef0e"
    ],
    "snapshot_digest": "sha256:a64826a869c1dd5d68c384976a1318c057ef238312d804f0a4646f249cec9ea0"
  },
  "candidate": {
    "closure_digest": "sha256:eb4125caabc455dbdaf3213f621ae16f53220e262ec7046303aa06cc735b02df",
    "corpus_sha_full": "289e4c1fa4c2406bd687091cda8f6938033aef0e",
    "data": {
      "adjudications": {
        "digest": "sha256:d58c21ff834352d3bdd098564ce46c7298d5dfeb93ee77cc74a1f805a7371f45",
        "path": "eval/adjudications.jsonl",
        "records": 2324
      },
      "fixtures": "sha256:f8bfdb4c93bd5a52986125edd15b1692586cd0e64ddac6eda6098be569597e8e",
      "pinned_probes": "sha256:c03b7180a7d95bc9b9f20f38a1a1d5f7c790403bc8f8c06c3c1afbe8feec1fa7"
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
    "harness_sha": "84383d0ff077ee58b63e16401c3be2fe0805b003",
    "oracle": "rust-analyzer 1.94.0 (4a4ef493 2026-03-02)",
    "oracle_init": {
      "cargo": {
        "features": "all"
      }
    },
    "policy_digest": "sha256:52c1f89b5310a480d050ba590e70f98a8a7a9e1dda270c2225f28b74abeca685",
    "population_digest": "sha256:921dca8858bc6a7e13eab0b54ec75e7301035dcba19ac41750ce4fd1645dce45",
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
      "289e4c1fa4c2406bd687091cda8f6938033aef0e"
    ],
    "snapshot_digest": "sha256:a64826a869c1dd5d68c384976a1318c057ef238312d804f0a4646f249cec9ea0"
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
- [x] `identities_equal`
- [x] `probe_sets_equal`
- [x] `adjudications_match`
- [x] `adjudications_digest_keys`
- [x] `oracle_answers_joined`
- [x] `oracle_changed_within_threshold`
- [x] `no_new_sut_errors`
- [x] `unexplained_zero`
- [x] `no_recall_loss`
- [x] `no_lost_matched_oracle_sites`
- [x] `matrix_no_regression`
- [x] `pinned_no_regression`

## Probe classes

```json
{
  "callees:src/ast.rs:2121": "carried_same",
  "callees:src/ast/tree_access.rs:8": "carried_same",
  "callees:src/call_graph.rs:11069": "carried_same",
  "callees:src/cpg/callback_identity_tests.rs:506": "carried_same",
  "callees:src/cpg/callee_identity_tests.rs:19": "carried_same",
  "callees:src/data_flow/queries.rs:44": "carried_same",
  "callees:src/executable_owner/integration/tests.rs:4": "carried_same",
  "callees:src/frameworks/python/flask.rs:226": "carried_same",
  "callees:src/main.rs:20": "carried_same",
  "callees:src/main.rs:490": "carried_same",
  "callees:src/main.rs:54": "carried_same",
  "callees:src/mcp/daemon/log.rs:167": "carried_same",
  "callees:src/mcp/daemon/wire_support.rs:68": "carried_same",
  "callees:tests/lang/python/inherited_receiver_test.rs:45": "carried_same",
  "callees:tests/mcp/daemon_process_test.rs:238": "carried_same",
  "callers:src/ast.rs:2121": "carried_same",
  "callers:src/ast/tree_access.rs:8": "carried_same",
  "callers:src/call_graph.rs:11069": "carried_same",
  "callers:src/cpg/callback_identity_tests.rs:506": "carried_same",
  "callers:src/cpg/callee_identity_tests.rs:19": "carried_same",
  "callers:src/data_flow/queries.rs:44": "carried_same",
  "callers:src/executable_owner/integration/tests.rs:4": "carried_same",
  "callers:src/frameworks/python/flask.rs:226": "carried_same",
  "callers:src/main.rs:20": "carried_same",
  "callers:src/main.rs:490": "carried_same",
  "callers:src/main.rs:54": "carried_same",
  "callers:src/mcp/daemon/log.rs:167": "carried_same",
  "callers:src/mcp/daemon/wire_support.rs:68": "carried_same",
  "callers:tests/lang/python/inherited_receiver_test.rs:45": "carried_same",
  "callers:tests/mcp/daemon_process_test.rs:238": "carried_same"
}
```

## Class counts

```json
{
  "carried_changed/content_change": 0,
  "carried_changed/sut_change": 0,
  "carried_same": 30,
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
  "base": {
    "callees": {
      "C-method": {
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
      "C-name": {
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
      "Q-scoped": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.5101091596030786,
          0.9999999999999999
        ],
        "recall": [
          1.0,
          0.5101091596030786,
          0.9999999999999999
        ],
        "tp": 4
      },
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          0.0,
          0.0,
          1.0
        ],
        "recall": [
          0.0,
          0.0,
          1.0
        ],
        "tp": 0
      }
    },
    "callers": {
      "C-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "recall": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "tp": 20
      },
      "C-name": {
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
      "Q-scoped": {
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
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "recall": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "tp": 17
      }
    }
  },
  "candidate": {
    "callees": {
      "C-method": {
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
      "C-name": {
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
      "Q-scoped": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.5101091596030786,
          0.9999999999999999
        ],
        "recall": [
          1.0,
          0.5101091596030786,
          0.9999999999999999
        ],
        "tp": 4
      },
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          0.0,
          0.0,
          1.0
        ],
        "recall": [
          0.0,
          0.0,
          1.0
        ],
        "tp": 0
      }
    },
    "callers": {
      "C-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "recall": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "tp": 20
      },
      "C-name": {
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
      "Q-scoped": {
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
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "recall": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "tp": 17
      }
    }
  }
}
```

## Raw measured metrics

```json
{
  "base": {
    "callees": {
      "C-method": {
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
      "C-name": {
        "fn": 1,
        "fp": 6,
        "precision": [
          0.45454545454545453,
          0.21271271487637833,
          0.7199084642140241
        ],
        "recall": [
          0.8333333333333334,
          0.4364971749742326,
          0.969946630588185
        ],
        "tp": 5
      },
      "Q-scoped": {
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
      "U-free": {
        "fn": 0,
        "fp": 11,
        "precision": [
          0.8035714285714286,
          0.6815539226635541,
          0.8866140437532375
        ],
        "recall": [
          1.0,
          0.921348400123914,
          0.9999999999999999
        ],
        "tp": 45
      },
      "U-method": {
        "fn": 0,
        "fp": 2,
        "precision": [
          0.0,
          0.0,
          0.6576197760453506
        ],
        "recall": [
          0.0,
          0.0,
          1.0
        ],
        "tp": 0
      }
    },
    "callers": {
      "C-method": {
        "fn": 117,
        "fp": 0,
        "precision": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "recall": [
          0.145985401459854,
          0.09653351214793943,
          0.21474882779321708
        ],
        "tp": 20
      },
      "C-name": {
        "fn": 1,
        "fp": 0,
        "precision": [
          1.0,
          0.8513451233948545,
          1.0
        ],
        "recall": [
          0.9565217391304348,
          0.7900884474454931,
          0.9922833339420488
        ],
        "tp": 22
      },
      "Q-scoped": {
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
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "recall": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "tp": 17
      }
    }
  },
  "candidate": {
    "callees": {
      "C-method": {
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
      "C-name": {
        "fn": 1,
        "fp": 6,
        "precision": [
          0.45454545454545453,
          0.21271271487637833,
          0.7199084642140241
        ],
        "recall": [
          0.8333333333333334,
          0.4364971749742326,
          0.969946630588185
        ],
        "tp": 5
      },
      "Q-scoped": {
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
      "U-free": {
        "fn": 0,
        "fp": 11,
        "precision": [
          0.8035714285714286,
          0.6815539226635541,
          0.8866140437532375
        ],
        "recall": [
          1.0,
          0.921348400123914,
          0.9999999999999999
        ],
        "tp": 45
      },
      "U-method": {
        "fn": 0,
        "fp": 2,
        "precision": [
          0.0,
          0.0,
          0.6576197760453506
        ],
        "recall": [
          0.0,
          0.0,
          1.0
        ],
        "tp": 0
      }
    },
    "callers": {
      "C-method": {
        "fn": 117,
        "fp": 0,
        "precision": [
          1.0,
          0.8388748398148705,
          1.0
        ],
        "recall": [
          0.145985401459854,
          0.09653351214793943,
          0.21474882779321708
        ],
        "tp": 20
      },
      "C-name": {
        "fn": 1,
        "fp": 0,
        "precision": [
          1.0,
          0.8513451233948545,
          1.0
        ],
        "recall": [
          0.9565217391304348,
          0.7900884474454931,
          0.9922833339420488
        ],
        "tp": 22
      },
      "Q-scoped": {
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
      "U-free": {
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
      "U-method": {
        "fn": 0,
        "fp": 0,
        "precision": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "recall": [
          1.0,
          0.8156818626193445,
          1.0
        ],
        "tp": 17
      }
    }
  }
}
```

## Measured metric rows

```json
{
  "base": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 1,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 0
        },
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              0.8,
              0.375534627029995,
              0.9637758917743693
            ],
            "tp": 4
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "C-name": {
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 1,
            "precision": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "recall": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 7,
        "raw": {
          "fn": 1,
          "fp": 6,
          "precision": [
            0.45454545454545453,
            0.21271271487637833,
            0.7199084642140241
          ],
          "recall": [
            0.8333333333333334,
            0.4364971749742326,
            0.969946630588185
          ],
          "tp": 5
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
            0.5101091596030786,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.5101091596030786,
            0.9999999999999999
          ],
          "tp": 4
        },
        "exact_tier": {
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
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 2,
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
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 6,
          "oracle_confirmed": 3,
          "oracle_unconfirmed": 3
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
            "fp": 8,
            "precision": [
              0.8431372549019608,
              0.7198955311470749,
              0.9183077682173217
            ],
            "recall": [
              0.9555555555555556,
              0.8517248911534567,
              0.987725873403887
            ],
            "tp": 43
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 11,
        "raw": {
          "fn": 0,
          "fp": 11,
          "precision": [
            0.8035714285714286,
            0.6815539226635541,
            0.8866140437532375
          ],
          "recall": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "tp": 45
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 2,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            0.0,
            0.0,
            1.0
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              0.0,
              0.0,
              1.0
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          },
          "raw": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "pending": 2,
        "raw": {
          "fn": 0,
          "fp": 2,
          "precision": [
            0.0,
            0.0,
            0.6576197760453506
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
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
            0.8388748398148705,
            1.0
          ],
          "recall": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "tp": 20
        },
        "exact_tier": {
          "raw": {
            "fn": 117,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.145985401459854,
              0.09653351214793943,
              0.21474882779321708
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          },
          "raw": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          }
        },
        "pending": 117,
        "raw": {
          "fn": 117,
          "fp": 0,
          "precision": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "recall": [
            0.145985401459854,
            0.09653351214793943,
            0.21474882779321708
          ],
          "tp": 20
        },
        "shortfall": 0
      },
      "C-name": {
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
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.8513451233948545,
              1.0
            ],
            "recall": [
              0.9565217391304348,
              0.7900884474454931,
              0.9922833339420488
            ],
            "tp": 22
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 1,
        "raw": {
          "fn": 1,
          "fp": 0,
          "precision": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "recall": [
            0.9565217391304348,
            0.7900884474454931,
            0.9922833339420488
          ],
          "tp": 22
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
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
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
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
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
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "exact_tier": {
          "raw": {
            "fn": 4,
            "fp": 0,
            "precision": [
              1.0,
              0.7719046248681998,
              1.0
            ],
            "recall": [
              0.7647058823529411,
              0.5273820168822574,
              0.9044495574456308
            ],
            "tp": 13
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "shortfall": 0
      }
    }
  },
  "candidate": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 1,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 0
        },
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              0.8,
              0.375534627029995,
              0.9637758917743693
            ],
            "tp": 4
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "C-name": {
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 1,
            "precision": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "recall": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 7,
        "raw": {
          "fn": 1,
          "fp": 6,
          "precision": [
            0.45454545454545453,
            0.21271271487637833,
            0.7199084642140241
          ],
          "recall": [
            0.8333333333333334,
            0.4364971749742326,
            0.969946630588185
          ],
          "tp": 5
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
            0.5101091596030786,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.5101091596030786,
            0.9999999999999999
          ],
          "tp": 4
        },
        "exact_tier": {
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
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 2,
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
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 6,
          "oracle_confirmed": 3,
          "oracle_unconfirmed": 3
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
            "fp": 8,
            "precision": [
              0.8431372549019608,
              0.7198955311470749,
              0.9183077682173217
            ],
            "recall": [
              0.9555555555555556,
              0.8517248911534567,
              0.987725873403887
            ],
            "tp": 43
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 11,
        "raw": {
          "fn": 0,
          "fp": 11,
          "precision": [
            0.8035714285714286,
            0.6815539226635541,
            0.8866140437532375
          ],
          "recall": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "tp": 45
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 2,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            0.0,
            0.0,
            1.0
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              0.0,
              0.0,
              1.0
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          },
          "raw": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "pending": 2,
        "raw": {
          "fn": 0,
          "fp": 2,
          "precision": [
            0.0,
            0.0,
            0.6576197760453506
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
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
            0.8388748398148705,
            1.0
          ],
          "recall": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "tp": 20
        },
        "exact_tier": {
          "raw": {
            "fn": 117,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.145985401459854,
              0.09653351214793943,
              0.21474882779321708
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          },
          "raw": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          }
        },
        "pending": 117,
        "raw": {
          "fn": 117,
          "fp": 0,
          "precision": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "recall": [
            0.145985401459854,
            0.09653351214793943,
            0.21474882779321708
          ],
          "tp": 20
        },
        "shortfall": 0
      },
      "C-name": {
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
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.8513451233948545,
              1.0
            ],
            "recall": [
              0.9565217391304348,
              0.7900884474454931,
              0.9922833339420488
            ],
            "tp": 22
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 1,
        "raw": {
          "fn": 1,
          "fp": 0,
          "precision": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "recall": [
            0.9565217391304348,
            0.7900884474454931,
            0.9922833339420488
          ],
          "tp": 22
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
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
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
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
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
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "exact_tier": {
          "raw": {
            "fn": 4,
            "fp": 0,
            "precision": [
              1.0,
              0.7719046248681998,
              1.0
            ],
            "recall": [
              0.7647058823529411,
              0.5273820168822574,
              0.9044495574456308
            ],
            "tp": 13
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "shortfall": 0
      }
    }
  }
}
```

## Full-sample metrics (appendix)

```json
{
  "base": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 1,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 0
        },
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              0.8,
              0.375534627029995,
              0.9637758917743693
            ],
            "tp": 4
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "C-name": {
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 1,
            "precision": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "recall": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 7,
        "raw": {
          "fn": 1,
          "fp": 6,
          "precision": [
            0.45454545454545453,
            0.21271271487637833,
            0.7199084642140241
          ],
          "recall": [
            0.8333333333333334,
            0.4364971749742326,
            0.969946630588185
          ],
          "tp": 5
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
            0.5101091596030786,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.5101091596030786,
            0.9999999999999999
          ],
          "tp": 4
        },
        "exact_tier": {
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
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 2,
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
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 6,
          "oracle_confirmed": 3,
          "oracle_unconfirmed": 3
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
            "fp": 8,
            "precision": [
              0.8431372549019608,
              0.7198955311470749,
              0.9183077682173217
            ],
            "recall": [
              0.9555555555555556,
              0.8517248911534567,
              0.987725873403887
            ],
            "tp": 43
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 11,
        "raw": {
          "fn": 0,
          "fp": 11,
          "precision": [
            0.8035714285714286,
            0.6815539226635541,
            0.8866140437532375
          ],
          "recall": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "tp": 45
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 2,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            0.0,
            0.0,
            1.0
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              0.0,
              0.0,
              1.0
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          },
          "raw": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "pending": 2,
        "raw": {
          "fn": 0,
          "fp": 2,
          "precision": [
            0.0,
            0.0,
            0.6576197760453506
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
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
            0.8388748398148705,
            1.0
          ],
          "recall": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "tp": 20
        },
        "exact_tier": {
          "raw": {
            "fn": 117,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.145985401459854,
              0.09653351214793943,
              0.21474882779321708
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          },
          "raw": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          }
        },
        "pending": 117,
        "raw": {
          "fn": 117,
          "fp": 0,
          "precision": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "recall": [
            0.145985401459854,
            0.09653351214793943,
            0.21474882779321708
          ],
          "tp": 20
        },
        "shortfall": 0
      },
      "C-name": {
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
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.8513451233948545,
              1.0
            ],
            "recall": [
              0.9565217391304348,
              0.7900884474454931,
              0.9922833339420488
            ],
            "tp": 22
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 1,
        "raw": {
          "fn": 1,
          "fp": 0,
          "precision": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "recall": [
            0.9565217391304348,
            0.7900884474454931,
            0.9922833339420488
          ],
          "tp": 22
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
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
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
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
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
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "exact_tier": {
          "raw": {
            "fn": 4,
            "fp": 0,
            "precision": [
              1.0,
              0.7719046248681998,
              1.0
            ],
            "recall": [
              0.7647058823529411,
              0.5273820168822574,
              0.9044495574456308
            ],
            "tp": 13
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "shortfall": 0
      }
    }
  },
  "candidate": {
    "callees": {
      "C-method": {
        "candidate_tier": {
          "count": 1,
          "oracle_confirmed": 1,
          "oracle_unconfirmed": 0
        },
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              0.8,
              0.375534627029995,
              0.9637758917743693
            ],
            "tp": 4
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "C-name": {
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
        "exact_tier": {
          "raw": {
            "fn": 1,
            "fp": 1,
            "precision": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "recall": [
              0.8333333333333334,
              0.4364971749742326,
              0.969946630588185
            ],
            "tp": 5
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 7,
        "raw": {
          "fn": 1,
          "fp": 6,
          "precision": [
            0.45454545454545453,
            0.21271271487637833,
            0.7199084642140241
          ],
          "recall": [
            0.8333333333333334,
            0.4364971749742326,
            0.969946630588185
          ],
          "tp": 5
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
            0.5101091596030786,
            0.9999999999999999
          ],
          "recall": [
            1.0,
            0.5101091596030786,
            0.9999999999999999
          ],
          "tp": 4
        },
        "exact_tier": {
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
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 2,
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
        },
        "shortfall": 0
      },
      "U-free": {
        "candidate_tier": {
          "count": 6,
          "oracle_confirmed": 3,
          "oracle_unconfirmed": 3
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
            "fp": 8,
            "precision": [
              0.8431372549019608,
              0.7198955311470749,
              0.9183077682173217
            ],
            "recall": [
              0.9555555555555556,
              0.8517248911534567,
              0.987725873403887
            ],
            "tp": 43
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          },
          "raw": {
            "fn": 0,
            "fp": 1,
            "precision": [
              0.6666666666666666,
              0.2076595988078242,
              0.9385080559790691
            ],
            "recall": [
              1.0,
              0.3423802239546494,
              1.0
            ],
            "tp": 2
          }
        },
        "pending": 11,
        "raw": {
          "fn": 0,
          "fp": 11,
          "precision": [
            0.8035714285714286,
            0.6815539226635541,
            0.8866140437532375
          ],
          "recall": [
            1.0,
            0.921348400123914,
            0.9999999999999999
          ],
          "tp": 45
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 2,
          "oracle_confirmed": 0,
          "oracle_unconfirmed": 2
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            0.0,
            0.0,
            1.0
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "exact_tier": {
          "raw": {
            "fn": 0,
            "fp": 0,
            "precision": [
              0.0,
              0.0,
              1.0
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          },
          "raw": {
            "fn": 0,
            "fp": 3,
            "precision": [
              0.0,
              0.0,
              0.5614970356393196
            ],
            "recall": [
              0.0,
              0.0,
              1.0
            ],
            "tp": 0
          }
        },
        "pending": 2,
        "raw": {
          "fn": 0,
          "fp": 2,
          "precision": [
            0.0,
            0.0,
            0.6576197760453506
          ],
          "recall": [
            0.0,
            0.0,
            1.0
          ],
          "tp": 0
        },
        "shortfall": 0
      }
    },
    "callers": {
      "C-method": {
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
            0.8388748398148705,
            1.0
          ],
          "recall": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "tp": 20
        },
        "exact_tier": {
          "raw": {
            "fn": 117,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.145985401459854,
              0.09653351214793943,
              0.21474882779321708
            ],
            "tp": 20
          }
        },
        "function": {
          "corrected": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          },
          "raw": {
            "fn": 66,
            "fp": 0,
            "precision": [
              1.0,
              0.8388748398148705,
              1.0
            ],
            "recall": [
              0.23255813953488372,
              0.15589108885641095,
              0.33209585422319776
            ],
            "tp": 20
          }
        },
        "pending": 117,
        "raw": {
          "fn": 117,
          "fp": 0,
          "precision": [
            1.0,
            0.8388748398148705,
            1.0
          ],
          "recall": [
            0.145985401459854,
            0.09653351214793943,
            0.21474882779321708
          ],
          "tp": 20
        },
        "shortfall": 0
      },
      "C-name": {
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
            "fn": 1,
            "fp": 0,
            "precision": [
              1.0,
              0.8513451233948545,
              1.0
            ],
            "recall": [
              0.9565217391304348,
              0.7900884474454931,
              0.9922833339420488
            ],
            "tp": 22
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 1,
        "raw": {
          "fn": 1,
          "fp": 0,
          "precision": [
            1.0,
            0.8513451233948545,
            1.0
          ],
          "recall": [
            0.9565217391304348,
            0.7900884474454931,
            0.9922833339420488
          ],
          "tp": 22
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
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
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
        "function": {
          "corrected": {
            "fn": 0,
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
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
            "fp": 0,
            "precision": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "recall": [
              1.0,
              0.5101091596030786,
              0.9999999999999999
            ],
            "tp": 4
          }
        },
        "pending": 0,
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
        },
        "shortfall": 0
      },
      "U-method": {
        "candidate_tier": {
          "count": 4,
          "oracle_confirmed": 4,
          "oracle_unconfirmed": 0
        },
        "corrected": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
        },
        "exact_tier": {
          "raw": {
            "fn": 4,
            "fp": 0,
            "precision": [
              1.0,
              0.7719046248681998,
              1.0
            ],
            "recall": [
              0.7647058823529411,
              0.5273820168822574,
              0.9044495574456308
            ],
            "tp": 13
          }
        },
        "function": {
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
          "raw": {
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
          }
        },
        "pending": 0,
        "raw": {
          "fn": 0,
          "fp": 0,
          "precision": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "recall": [
            1.0,
            0.8156818626193445,
            1.0
          ],
          "tp": 17
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
