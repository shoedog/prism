# Task 29 report — C/C++ per-declarator chain ownership

## Authority and scope

- Base: `39a6e9cb183c9b9681b999968fc380e62e28efa7`.
- Branch: `binding-enumeration-s4`; origin disabled; no push.
- Production scope: C/C++ binding enumeration, same-declaration fail-closed classification, C++ declarator descent, and cache version 133.
- C/C++ multi-declarator names now receive the same later-line labels as the equivalent sequence of single declarations when the declaration row proves ownership.
- No non-C/C++ production binding row changed.

## RED-first receipt

The isolated `git archive` of BASE_HEAD used its own target directory. The admissible RED command was `cargo test task29_ --lib`: 0 passed / 6 failed / 0 ignored / 2,276 filtered. The six failures cover C and C++ later-line equivalence, pointer/array wrappers, nested shadowing, same-line chain-mate reads, and C++ direct-init/most-vexing-parse. Receipt: `task29-red.log`.

Two earlier probes were inadmissible and did not update belief: one test expectation failed to compile because it used non-static strings, and one array assertion selected no own DefSite. Both probes were corrected before the recorded RED.

## Implementation partitions

- Execution: every proved declarator in a C/C++ declaration becomes its own `Other` definition.
- Bypass: the declaration rows retain ordinary CFG path behavior.
- Initializer visibility: later-line uses may become Exact/Killed; any use inside the same declaration node is forced to OwnershipUncertain, including multi-line declaration spans.
- Scope exit: nested-block chain declarations kill outer definitions inside the block and restore the outer definition after exit, matching split declarations.
- Combinations: pointer, array, parenthesized, and init-declarator wrappers use the existing introduced-identifier descent.
- C++ direct-initialization and most-vexing-parse shapes containing `function_declarator` remain `Uncertain{future:cpp-declarators}`.

## Gates

| Gate | Result |
|---|---|
| Task29 focused | 6 passed / 0 failed / 0 ignored |
| `cpg::reaching` | 360 passed / 0 failed / 6 ignored |
| Enumeration | 230 passed / 0 failed / 0 ignored |
| Cache tests | 77 passed / 0 failed / 0 ignored |
| Full all-target/all-feature, no-fail-fast | 7,195 passed / 153 failed / 14 ignored. All 25 controller-baseline failures are present; 128 additional host failures reproduce on same-environment base; zero HEAD-only failures. |
| Doc tests | 2 passed / 0 failed / 0 ignored |
| Phase-0 byte control | 1,645 / 1,645 identical |
| Item-2 byte control | 280 / 280 identical |
| Tier-A matrix | 251 ok / 0 regressed / 0 errors |
| Label delta | exactly 6 fixtures; 0 errors |
| Clippy | exit 101, 321 raw warning headers; identical to same-environment base. The sole deny-lint error is the untouched `freshness_policy_tests.rs:172 from_mode(0)`; Task29 adds zero warnings. |
| Formatting | `cargo fmt --all -- --check` passed |
| File cap | all Task29 implementation/test files <=600 weighted lines. Mandatory `src/cpg_cache.rs` is a pre-existing 1,638.5-line carrier; the version bump is line-neutral. |

The exhaustive clean-base control produced 7,183 passed / 159 failed / 14 ignored. Canonical failure-name comparison found zero HEAD-only failures. Its six base-only failures are `echo_misc_test::test_phantom_slice_runs`, `echo_misc_test::test_resonance_slice_runs`, three CLI phantom/resonance/threed tests, and `version_test::test_version_includes_git_sha_and_dirty_flag`; they are archive/environment control artifacts and are not attributed to Task29. HEAD adds the six Task29 tests, explaining the test-population difference.

The quick Tier-A corpus run is INVALID rather than a regression result: repeated SUT navigation timeouts exhausted the 600-second `prism-quick` corpus budget, and the subsequent Excalidraw run repeated the long-SUT behavior. Records: `task29-quick-invalid.json` and `task29-quick-invalid.md`.

### Exact fixture delta

1. `c/dfg_binding_c_chain_multiline`
2. `c/dfg_binding_c_chain_scope`
3. `c/dfg_binding_declarator_chain`
4. `cpp/dfg_binding_cpp_declarator_chain_uncertain`
5. `cpp/dfg_binding_cpp_direct_init_uncertain`
6. `cpp/dfg_binding_cpp_for_chain_scope_exit`

## Corpus census

The base and HEAD edge multisets have equal cardinality in every corpus. Duplicate edge identities were joined as multisets.

| corpus | base/head edges | changed | OU->Exact | OU->Killed | other | changed files |
|---|---:|---:|---:|---:|---:|---:|
| Redis | 413,789 / 413,789 | 67,685 | 12,571 | 54,957 | 157 | 94 |
| FRR | 109,081 / 109,081 | 5,184 | 3,328 | 1,836 | 20 | 97 |
| LevelDB | 20,332 / 20,332 | 252 | 80 | 23 | 149 | 30 |

The supplied FRR path `/Users/wesleyjinks/code/frrouting-test/lib` did not exist and was inadmissible. The read-only corpus was resolved to `/Users/wesleyjinks/code/frrouting-test/frr/lib`.

- Changed non-C/C++ edges: zero in all three corpora.
- New-precise candidate population audited: 72,795 edges across 203 parsed changed files.
- New Exact/Killed edges on same-declaration chain-mate reads: zero.
- Synthetic outer-definition regression `int x=source(); { int a=x, x=0; }`: detected by the auditor.
- Audit record: `task29-census/same-line-audit.json`.
- Complete machine-readable transition and by-file record: `task29-census/summary.json`.

### Other transitions

- Redis: Killed->Killed 10; NameOnly:alias_unstable->OU 94; NameOnly:sameline->OU 16; OU->NameOnly:cfg_incomplete 36; OU->OU 1.
- FRR: Exact->OU 2; NameOnly:alias_unstable->OU 3; NameOnly:sameline->OU 3; OU->NameOnly:cfg_incomplete 12.
- LevelDB: Exact->OU 136; Killed->OU 3; NameOnly:cfg_incomplete->OU 2; NameOnly:sameline->OU 8.

### Seed-29 random sample: 20 of 15,979 OU->Exact edges

| # | corpus | file | def->use | name |
|---:|---|---|---:|---|
| 1 | redis | sentinel.c | 1600->1618 | numslaves |
| 2 | redis | cluster_legacy.c | 400->568 | n |
| 3 | redis | rax.c | 1645->1656 | eq |
| 4 | redis | sort.c | 217->289 | j |
| 5 | frr | link_state.c | 2679->2685 | jobj |
| 6 | redis | sort.c | 355->383 | vectorlen |
| 7 | redis | module.c | 1195->1196 | j |
| 8 | redis | cluster_legacy.c | 2805->3141 | senderConfigEpoch |
| 9 | redis | redis-cli.c | 10710->10783 | reply |
| 10 | frr | typesafe.c | 103->105 | newsize |
| 11 | redis | rax.c | 840->931 | h |
| 12 | redis | rdb.c | 2560->2785 | o |
| 13 | redis | redis-benchmark.c | 1125->1148 | addr |
| 14 | redis | aof.c | 1784->1819 | status |
| 15 | leveldb | db/version_set.cc | 1412->1415 | new_start |
| 16 | redis | bitops.c | 1250->1470 | op |
| 17 | redis | config.c | 1610->1616 | was_empty |
| 18 | frr | log.c | 704->706 | pos |
| 19 | redis | redis-cli.c | 5164->5177 | i |
| 20 | redis | t_list.c | 1219->1236 | oldlen |

## Review

Review cap: 2 rounds.

- Round 1: one WRONG in the evidence tool. The first same-line audit compared edge endpoint lines and could miss an outer definition feeding a later same-declaration read. The audit was replaced by an AST/node-span/name auditor and given the synthetic outer-shadow regression above.
- Round 2: ACCEPT. Astra found no remaining WRONG or SMELL in production or evidence.

## Fail-closed shapes preserved

- C/C++ chain-mate reads anywhere inside the same declaration node: OwnershipUncertain, reason `declarator chain`, revisit `future:capture-identity`.
- C++ direct-initialization and most-vexing-parse/function-declarator declarations: Uncertain, revisit `future:cpp-declarators`.
- Unproved identities and residual ownership predicates continue to fail closed.
- Nested shadowing is proved by cases for later-line use, scope exit, and same-declaration preservation.

## Per-file transition counts

## frr by-file counts

| file | transitions | changed |
|---|---:|---:|
| `admin_group.c` | OU->Exact=97, OU->Killed=18 | 115 |
| `agentx.c` | OU->Exact=33, OU->Killed=19, OU->NameOnly:cfg_incomplete=1 | 53 |
| `asn.c` | OU->Exact=18, OU->Killed=10 | 28 |
| `atomlist.c` | OU->Exact=44, OU->Killed=11 | 55 |
| `bfd.c` | OU->Exact=2 | 2 |
| `bitfield.h` | OU->Exact=28, OU->Killed=19 | 47 |
| `buffer.c` | NameOnly:alias_unstable->OU=2, OU->Exact=23, OU->Killed=4 | 29 |
| `checksum.c` | OU->Exact=71, OU->Killed=38 | 109 |
| `command.c` | OU->Exact=62, OU->Killed=18 | 80 |
| `command_graph.c` | OU->Exact=53, OU->Killed=10 | 63 |
| `command_match.c` | NameOnly:sameline->OU=2, OU->Exact=58, OU->Killed=16 | 76 |
| `command_py.c` | OU->Exact=8 | 8 |
| `csv.c` | OU->Exact=18, OU->Killed=3 | 21 |
| `darr.c` | OU->Exact=20, OU->Killed=7 | 27 |
| `defaults.c` | OU->Exact=75, OU->Killed=2 | 77 |
| `distribute.c` | OU->Exact=10, OU->Killed=6 | 16 |
| `elf_py.c` | OU->Exact=91, OU->Killed=24, OU->NameOnly:cfg_incomplete=3 | 118 |
| `event.c` | OU->Exact=11, OU->Killed=4 | 15 |
| `ferr.c` | OU->Exact=18, OU->Killed=5 | 23 |
| `filter_cli.c` | OU->Exact=2 | 2 |
| `filter_nb.c` | OU->Exact=38, OU->Killed=12 | 50 |
| `frr_pthread.c` | OU->Exact=3 | 3 |
| `frr_zmq.c` | OU->Exact=21, OU->Killed=13 | 34 |
| `frrcu.c` | OU->Exact=10, OU->Killed=5 | 15 |
| `frrstr.c` | OU->Exact=18, OU->Killed=2 | 20 |
| `grammar_sandbox.c` | OU->Exact=2 | 2 |
| `hash.c` | OU->Exact=10, OU->Killed=7 | 17 |
| `hook.c` | OU->Exact=28, OU->Killed=16 | 44 |
| `id_alloc.c` | OU->Exact=27, OU->Killed=14 | 41 |
| `if.c` | OU->Exact=32, OU->Killed=17 | 49 |
| `ipaddr.h` | OU->Exact=11, OU->Killed=9 | 20 |
| `jhash.c` | OU->Exact=53, OU->Killed=72 | 125 |
| `json.c` | OU->Exact=37, OU->Killed=18 | 55 |
| `keychain_cli.c` | OU->Exact=15, OU->Killed=8 | 23 |
| `lib_vty.c` | OU->Exact=10, OU->Killed=3 | 13 |
| `libfrr.c` | OU->Exact=67, OU->Killed=22 | 89 |
| `link_state.c` | OU->Exact=177, OU->Killed=181 | 358 |
| `log.c` | OU->Exact=19, OU->Killed=5 | 24 |
| `log_cli.c` | OU->Exact=5, OU->Killed=2 | 7 |
| `log_filter.c` | OU->Exact=6, OU->Killed=3 | 9 |
| `log_nb.c` | OU->Exact=20, OU->Killed=4 | 24 |
| `md5.c` | OU->Exact=4, OU->Killed=3 | 7 |
| `mgmt_be_client.c` | OU->Exact=18, OU->Killed=5 | 23 |
| `mgmt_msg.c` | NameOnly:alias_unstable->OU=1, OU->Exact=48, OU->Killed=15 | 64 |
| `module.c` | OU->Exact=22, OU->Killed=11 | 33 |
| `monotime.h` | OU->Exact=2 | 2 |
| `network.c` | OU->Exact=3 | 3 |
| `nexthop.c` | OU->Exact=18, OU->Killed=6 | 24 |
| `nexthop_group.c` | Exact->OU=2, OU->Exact=22, OU->Killed=9 | 33 |
| `northbound.c` | OU->Exact=17, OU->Killed=3 | 20 |
| `northbound_cli.c` | OU->Exact=7, OU->Killed=3 | 10 |
| `northbound_grpc.cpp` | OU->Exact=15, OU->Killed=1 | 16 |
| `northbound_notif.c` | OU->Exact=21, OU->Killed=13 | 34 |
| `northbound_oper.c` | OU->Exact=84, OU->Killed=53 | 137 |
| `northbound_sysrepo.c` | OU->Exact=3 | 3 |
| `ntop.c` | OU->Exact=30, OU->Killed=14 | 44 |
| `openbsd-tree.c` | OU->Exact=113, OU->Killed=41 | 154 |
| `openbsd-tree.h` | OU->Exact=8, OU->Killed=2 | 10 |
| `plist.c` | OU->Exact=56, OU->Killed=39 | 95 |
| `prefix.c` | OU->Exact=35, OU->Killed=14 | 49 |
| `printf/printf-pos.c` | OU->Exact=11, OU->Killed=1 | 12 |
| `printf/vfprintf.c` | OU->Exact=35, OU->Killed=16 | 51 |
| `privs.c` | OU->Exact=43, OU->Killed=18 | 61 |
| `ptm_lib.c` | OU->Exact=64, OU->Killed=32 | 96 |
| `pullwr.c` | OU->Exact=22, OU->Killed=11 | 33 |
| `qobj.c` | OU->Exact=2 | 2 |
| `resolver.c` | OU->Exact=10, OU->Killed=4 | 14 |
| `routemap.c` | OU->Exact=60, OU->Killed=31 | 91 |
| `routemap_northbound.c` | OU->Exact=3, OU->Killed=1 | 4 |
| `sbuf.c` | OU->Exact=5, OU->Killed=2 | 7 |
| `seqlock.c` | OU->Exact=27, OU->Killed=12 | 39 |
| `sha256.c` | OU->Exact=9, OU->Killed=1 | 10 |
| `skiplist.c` | OU->Exact=148, OU->Killed=127 | 275 |
| `sockunion.c` | OU->Exact=1 | 1 |
| `srcdest_table.c` | OU->Exact=48, OU->Killed=37 | 85 |
| `srv6.c` | OU->Exact=1 | 1 |
| `stream.c` | OU->Exact=4, OU->Killed=2 | 6 |
| `stream.h` | OU->Exact=3 | 3 |
| `strformat.c` | OU->Exact=8, OU->Killed=4 | 12 |
| `systemd.c` | OU->Exact=7, OU->Killed=3 | 10 |
| `table.c` | OU->Exact=38, OU->Killed=28 | 66 |
| `tc.c` | OU->Exact=3 | 3 |
| `termtable.c` | OU->Exact=60, OU->Killed=44 | 104 |
| `typerb.c` | OU->Exact=110, OU->Killed=39 | 149 |
| `typesafe.c` | OU->Exact=230, OU->Killed=136, OU->NameOnly:cfg_incomplete=2 | 368 |
| `vrf.c` | OU->Exact=58, OU->Killed=42 | 100 |
| `vty.c` | OU->Exact=26, OU->Killed=17 | 43 |
| `wheel.c` | OU->Exact=1 | 1 |
| `xref.c` | OU->Exact=12, OU->Killed=4 | 16 |
| `yang.c` | OU->Exact=13 | 13 |
| `yang_translator.c` | OU->Exact=23, OU->Killed=15 | 38 |
| `zclient.c` | OU->Exact=74, OU->Killed=234 | 308 |
| `zlog.c` | NameOnly:sameline->OU=1, OU->Exact=73, OU->Killed=49 | 123 |
| `zlog_5424.c` | OU->Exact=107, OU->Killed=50, OU->NameOnly:cfg_incomplete=4 | 161 |
| `zlog_5424_cli.c` | OU->Killed=3, OU->NameOnly:cfg_incomplete=2 | 5 |
| `zlog_live.c` | OU->Exact=60, OU->Killed=1 | 61 |
| `zlog_targets.c` | OU->Exact=52, OU->Killed=13 | 65 |

## leveldb by-file counts

| file | transitions | changed |
|---|---:|---:|
| `db/autocompact_test.cc` | Exact->OU=1 | 1 |
| `db/c.cc` | Exact->OU=4, OU->Exact=1 | 5 |
| `db/corruption_test.cc` | Exact->OU=1, OU->Exact=5 | 6 |
| `db/db_impl.cc` | Exact->OU=4, OU->Exact=3, OU->Killed=1 | 8 |
| `db/db_test.cc` | Exact->OU=33, OU->Exact=22, OU->Killed=21 | 76 |
| `db/dbformat_test.cc` | Exact->OU=1 | 1 |
| `db/fault_injection_test.cc` | Exact->OU=7, OU->Exact=2 | 9 |
| `db/filename.cc` | Exact->OU=7, NameOnly:sameline->OU=1 | 8 |
| `db/recovery_test.cc` | Exact->OU=1 | 1 |
| `db/repair.cc` | Exact->OU=2 | 2 |
| `db/skiplist_test.cc` | Exact->OU=3 | 3 |
| `db/table_cache.cc` | Exact->OU=2 | 2 |
| `db/version_edit_test.cc` | OU->Exact=3 | 3 |
| `db/version_set.cc` | Exact->OU=1, OU->Exact=24 | 25 |
| `db/version_set.h` | NameOnly:sameline->OU=1, OU->Exact=1 | 2 |
| `db/version_set_test.cc` | Exact->OU=2 | 2 |
| `db/write_batch.cc` | Exact->OU=7, OU->Exact=2 | 9 |
| `db/write_batch_test.cc` | Exact->OU=1, OU->Exact=9 | 10 |
| `table/block.cc` | Exact->OU=1, OU->Exact=6, OU->Killed=1 | 8 |
| `table/block_builder.cc` | Exact->OU=3 | 3 |
| `table/table.cc` | Exact->OU=2 | 2 |
| `table/table_builder.cc` | OU->Exact=2 | 2 |
| `table/table_test.cc` | Exact->OU=36 | 36 |
| `util/coding_test.cc` | Exact->OU=5 | 5 |
| `util/env_posix.cc` | Exact->OU=2 | 2 |
| `util/env_test.cc` | Exact->OU=4 | 4 |
| `util/env_windows.cc` | Exact->OU=2 | 2 |
| `util/logging_test.cc` | Killed->OU=3, NameOnly:sameline->OU=6 | 9 |
| `util/no_destructor_test.cc` | Exact->OU=4 | 4 |
| `util/status.cc` | NameOnly:cfg_incomplete->OU=2 | 2 |

## redis by-file counts

| file | transitions | changed |
|---|---:|---:|
| `acl.c` | OU->Exact=82, OU->Killed=19 | 101 |
| `adlist.c` | OU->Exact=9, OU->Killed=4 | 13 |
| `ae.c` | OU->Exact=35, OU->Killed=10 | 45 |
| `ae_epoll.c` | OU->Exact=7, OU->Killed=1 | 8 |
| `ae_evport.c` | OU->Exact=44, OU->Killed=16 | 60 |
| `ae_kqueue.c` | OU->Exact=16, OU->Killed=7 | 23 |
| `ae_select.c` | OU->Exact=11, OU->Killed=2 | 13 |
| `anet.c` | OU->Exact=61, OU->Killed=12 | 73 |
| `aof.c` | OU->Exact=118, OU->Killed=41 | 159 |
| `bitops.c` | OU->Exact=490, OU->Killed=316 | 806 |
| `blocked.c` | OU->Exact=9, OU->Killed=8 | 17 |
| `cli_common.c` | OU->Exact=20, OU->Killed=5 | 25 |
| `cluster.c` | OU->Exact=194, OU->Killed=66 | 260 |
| `cluster.h` | OU->Exact=7, OU->Killed=5 | 12 |
| `cluster_asm.c` | OU->Exact=27, OU->Killed=1 | 28 |
| `cluster_legacy.c` | NameOnly:alias_unstable->OU=1, OU->Exact=470, OU->Killed=353, OU->NameOnly:cfg_incomplete=1, OU->OU=1 | 826 |
| `cluster_slot_stats.c` | OU->Exact=14, OU->Killed=4 | 18 |
| `config.c` | OU->Exact=93, OU->Killed=69 | 162 |
| `crc64.c` | NameOnly:alias_unstable->OU=3, OU->Exact=73, OU->Killed=20 | 96 |
| `crccombine.c` | OU->Exact=4, OU->Killed=1 | 5 |
| `crcspeed.c` | OU->Exact=13 | 13 |
| `db.c` | OU->Exact=344, OU->Killed=102, OU->NameOnly:cfg_incomplete=2 | 448 |
| `debug.c` | OU->Exact=101, OU->Killed=93 | 194 |
| `defrag.c` | NameOnly:sameline->OU=1, OU->Exact=290, OU->Killed=26 | 317 |
| `dict.c` | NameOnly:alias_unstable->OU=7, OU->Exact=236, OU->Killed=168 | 411 |
| `ebuckets.c` | NameOnly:alias_unstable->OU=3, OU->Exact=194, OU->Killed=52 | 249 |
| `endianconv.c` | OU->Exact=32, OU->Killed=10 | 42 |
| `entry.c` | OU->Exact=10, OU->Killed=7 | 17 |
| `eval.c` | OU->Exact=23, OU->Killed=1 | 24 |
| `evict.c` | NameOnly:alias_unstable->OU=2, OU->Exact=39, OU->Killed=12 | 53 |
| `expire.c` | NameOnly:alias_unstable->OU=1, NameOnly:sameline->OU=4, OU->Exact=64, OU->Killed=9 | 78 |
| `fast_float_strtod.c` | NameOnly:alias_unstable->OU=6, OU->Exact=7 | 13 |
| `fwtree.c` | OU->Exact=4 | 4 |
| `gcra.c` | OU->Exact=9 | 9 |
| `geo.c` | OU->Exact=99, OU->Killed=4 | 103 |
| `geohash_helper.c` | OU->Exact=31, OU->Killed=4 | 35 |
| `hyperloglog.c` | NameOnly:alias_unstable->OU=9, OU->Exact=394, OU->Killed=129 | 532 |
| `intset.c` | OU->Exact=34, OU->Killed=4 | 38 |
| `iothread.c` | OU->Exact=16, OU->Killed=5 | 21 |
| `kvstore.c` | OU->Exact=10, OU->Killed=2 | 12 |
| `latency.c` | OU->Exact=17, OU->Killed=1 | 18 |
| `lazyfree.c` | OU->Exact=2 | 2 |
| `listpack.c` | NameOnly:alias_unstable->OU=3, NameOnly:sameline->OU=3, OU->Exact=888, OU->Killed=33727 | 34621 |
| `lolwut.c` | OU->Exact=7 | 7 |
| `memtest.c` | OU->Exact=62, OU->Killed=31 | 93 |
| `module.c` | NameOnly:alias_unstable->OU=11, OU->Exact=229, OU->Killed=63, OU->NameOnly:cfg_incomplete=2 | 305 |
| `modules/hellotype.c` | OU->Exact=22, OU->Killed=4 | 26 |
| `modules/helloworld.c` | NameOnly:alias_unstable->OU=1, OU->Exact=18, OU->Killed=1 | 20 |
| `monotonic.c` | OU->Exact=3 | 3 |
| `mstr.c` | OU->Exact=21, OU->Killed=3 | 24 |
| `mt19937-64.c` | OU->Exact=19, OU->Killed=5 | 24 |
| `multi.c` | OU->Exact=3, OU->Killed=1 | 4 |
| `networking.c` | OU->Exact=163, OU->Killed=353, OU->NameOnly:cfg_incomplete=4 | 520 |
| `notify.c` | OU->Exact=67, OU->Killed=296 | 363 |
| `object.c` | OU->Exact=21, OU->Killed=2 | 23 |
| `pqsort.c` | OU->Exact=112, OU->Killed=69 | 181 |
| `pubsub.c` | OU->Exact=2, OU->Killed=1 | 3 |
| `quicklist.c` | NameOnly:alias_unstable->OU=1, OU->Exact=153, OU->Killed=24 | 178 |
| `rax.c` | OU->Exact=189, OU->Killed=79, OU->NameOnly:cfg_incomplete=5 | 273 |
| `rdb.c` | OU->Exact=1022, OU->Killed=4139, OU->NameOnly:cfg_incomplete=2 | 5163 |
| `redis-benchmark.c` | NameOnly:alias_unstable->OU=5, OU->Exact=213, OU->Killed=357, OU->NameOnly:cfg_incomplete=1 | 576 |
| `redis-check-aof.c` | OU->Exact=22, OU->Killed=3 | 25 |
| `redis-check-rdb.c` | OU->Exact=55, OU->Killed=31 | 86 |
| `redis-cli.c` | NameOnly:alias_unstable->OU=12, OU->Exact=1391, OU->Killed=913, OU->NameOnly:cfg_incomplete=12 | 2328 |
| `replication.c` | NameOnly:alias_unstable->OU=2, OU->Exact=189, OU->Killed=174 | 365 |
| `script_lua.c` | OU->Exact=16, OU->Killed=4, OU->NameOnly:cfg_incomplete=3 | 23 |
| `sds.c` | OU->Exact=320, OU->Killed=1916 | 2236 |
| `sentinel.c` | NameOnly:sameline->OU=2, OU->Exact=220, OU->Killed=120 | 342 |
| `server.c` | NameOnly:alias_unstable->OU=8, NameOnly:sameline->OU=1, OU->Exact=182, OU->Killed=23 | 214 |
| `setcpuaffinity.c` | NameOnly:alias_unstable->OU=2, OU->Exact=25, OU->Killed=16 | 43 |
| `setproctitle.c` | OU->Exact=40, OU->Killed=24 | 64 |
| `sha1.c` | OU->Exact=124, OU->Killed=31 | 155 |
| `sha256.c` | OU->Exact=59, OU->Killed=5 | 64 |
| `siphash.c` | OU->Exact=11, OU->Killed=13 | 24 |
| `slowlog.c` | OU->Exact=15, OU->Killed=15 | 30 |
| `socket.c` | OU->Exact=4 | 4 |
| `sort.c` | NameOnly:alias_unstable->OU=1, OU->Exact=413, OU->Killed=288 | 702 |
| `sparsearray.c` | OU->Exact=43, OU->Killed=11 | 54 |
| `syncio.c` | OU->Exact=13, OU->Killed=3 | 16 |
| `syscheck.c` | OU->Exact=34, OU->Killed=40 | 74 |
| `t_array.c` | NameOnly:sameline->OU=4, OU->Exact=15 | 19 |
| `t_hash.c` | Killed->Killed=10, NameOnly:alias_unstable->OU=8, NameOnly:sameline->OU=1, OU->Exact=455, OU->Killed=178 | 652 |
| `t_list.c` | OU->Exact=83, OU->Killed=13 | 96 |
| `t_set.c` | NameOnly:alias_unstable->OU=3, OU->Exact=204, OU->Killed=134 | 341 |
| `t_stream.c` | OU->Exact=183, OU->Killed=397 | 580 |
| `t_string.c` | OU->Exact=205, OU->Killed=140 | 345 |
| `t_zset.c` | NameOnly:alias_unstable->OU=3, OU->Exact=529, OU->Killed=233 | 765 |
| `tls.c` | OU->Exact=19, OU->Killed=7 | 26 |
| `tracking.c` | OU->Exact=5 | 5 |
| `util.c` | OU->Killed=29, OU->NameOnly:cfg_incomplete=3 | 32 |
| `vector.c` | OU->Exact=17 | 17 |
| `ziplist.c` | NameOnly:alias_unstable->OU=1, OU->Exact=576, OU->Killed=9333, OU->NameOnly:cfg_incomplete=1 | 9911 |
| `zipmap.c` | NameOnly:alias_unstable->OU=1, OU->Exact=97, OU->Killed=75 | 173 |
| `zmalloc.c` | OU->Exact=40, OU->Killed=44 | 84 |

## Fix round 1

### Findings and disposition

- W1: C++ initializer reads were materialized as ordinary definitions. `normalize_cpp_pseudo_defs` now marks non-writing initializer occurrences alias-derived while retaining assignment/update targets; callable initializer values remain owned by the existing capture normalization.
- W2 hypothesis: the flattened CFG admitted a route between mutually exclusive C/C++ branch arms after Task 29 recovered a declarator chain. The separating alternative was a failure to recover the chain owner; the real Redis `sort.c` probe found `owner=true`, but the use at line 289 had no lexical arm because its containing statement starts at line 288. Canonical containing-statement lines establish the branch-arm comparison, and chain-recovered edges fail closed when either the source route or a candidate kill witness is in an incomparable arm.
- W2 disposition: **(c)**. A shared proof change was rejected because the reduced Redis loop showed that standalone C update definitions are absent from the shared definition set. The fix is confined to C/C++ chain-recovered labels; split declarations remain a follow-up population.
- S1: the behavior-only assertions in `cpp/fix1.rs` and `cpp/fix2.rs` are restored to `check_case`, including row/variant/ruling checks and the unchanged structured-binding case.
- Persisted labels changed, so cache version 133 was invalidated by version 134; the previous/current boundary fixtures were advanced with it.

### RED and gates

| Gate | Result |
|---|---|
| Fix-round RED at `d6ecd6e8` | 1 passed / 3 failed / 0 ignored; both W1 and W2 failed |
| Fix-round focused | 4 passed / 0 failed / 0 ignored |
| Restored Astra assertions | 19 passed / 0 failed / 0 ignored |
| `cpg::reaching` | 364 passed / 0 failed / 6 ignored |
| Cache module | 77 passed / 0 failed / 0 ignored |
| Host full HEAD | 7,327 passed / 25 failed / 14 ignored |
| Same-environment host base | 7,323 passed / 25 failed / 14 ignored |
| Host failure-name diff | zero; exactly the 25 controller carries |
| Doc tests | 2 passed / 0 failed / 0 ignored |
| Phase-0 byte control | 1,645 / 1,645 identical |
| Item-2 byte control | 280 / 280 identical |
| Tier-A matrix | 251 ok / 0 gap / 0 fail |
| Label delta | exactly the original 6 fixtures; 0 errors |
| Clippy | 280 warning headers / 0 errors, equal to base |
| Formatting | `cargo fmt --all -- --check` passed |

The six label-delta fixtures remain:

1. `c/dfg_binding_c_chain_multiline`
2. `c/dfg_binding_c_chain_scope`
3. `c/dfg_binding_declarator_chain`
4. `cpp/dfg_binding_cpp_declarator_chain_uncertain`
5. `cpp/dfg_binding_cpp_direct_init_uncertain`
6. `cpp/dfg_binding_cpp_for_chain_scope_exit`

All touched implementation/test files are at most 600 weighted lines except the mandatory line-neutral cache carrier `src/cpg_cache.rs`, a pre-existing 1,638.5-line file.

### Fresh corpus census

Base and HEAD edge multisets retain equal cardinality in every corpus.

| corpus | base/head edges | changed | OU->Exact | OU->Killed | other |
|---|---:|---:|---:|---:|---:|
| Redis | 413,789 / 413,789 | 54,139 | 8,454 | 40,637 | 5,048 |
| FRR | 109,081 / 109,081 | 4,435 | 2,655 | 1,389 | 391 |
| LevelDB | 20,332 / 20,332 | 1,259 | 50 | 1 | 1,208 |

The five Astra-confirmed edges all leave Exact/Killed:

- Redis `rdb.c n 1130->1360`: OwnershipUncertain.
- Redis `rdb.c n 1134->1360`: OwnershipUncertain.
- Redis `rdb.c o 2408->2702`: OwnershipUncertain.
- Redis `rdb.c o 2460->2702`: OwnershipUncertain.
- Redis `sort.c j 217->289`: OwnershipUncertain.

Fresh seed-29 sample: 20 of 11,159 OU->Exact edges.

| corpus | file | def->use | name |
|---|---|---:|---|
| FRR | `elf_py.c` | 603->606 | shdr |
| Redis | `crc64.c` | 348->345 | init_end |
| Redis | `sds.c` | 370->374 | sh |
| FRR | `openbsd-tree.c` | 139->145 | gparent |
| FRR | `prefix.c` | 304->311 | np |
| Redis | `redis-cli.c` | 5067->5094 | r1 |
| Redis | `db.c` | 2603->2617 | newdb |
| Redis | `zipmap.c` | 147->131 | p |
| Redis | `sds.c` | 284->315 | bufsize |
| Redis | `sha256.c` | 61->74 | d |
| Redis | `t_hash.c` | 1418->1427 | vptr |
| Redis | `bitops.c` | 129->117 | aux3 |
| Redis | `bitops.c` | 1287->1495 | j |
| Redis | `defrag.c` | 1012->1016 | s |
| Redis | `t_set.c` | 1408->1542 | cardinality |
| FRR | `typesafe.c` | 573->582 | moveto |
| Redis | `networking.c` | 950->956 | prev |
| Redis | `sds.c` | 899->932 | elements |
| Redis | `db.c` | 2555->2576 | db2 |
| Redis | `ziplist.c` | 2111->2112 | p |

### Latent split-declaration follow-up population

The no-loop AST audit counted 1,663 mutually exclusive-arm edges that were precise at BASE and remain precise for split declarations: Redis 1,328, FRR 323, LevelDB 12. Three examples are Redis `acl.c flags 1083->1099` (Exact), FRR `asn.c val 97->99` (Exact), and LevelDB `db/c_test.c r kill 112->use 113` (Killed). This population is not served by the chain-only fix and requires a separate shared route/update-definition design issue.

## Park round (DECISIONS §P.1)

Owner option A parks Task 29 activation fail-closed. The single named gate is `CHAIN_RECOVERY_ENABLED = false` in `src/cpg/reaching/binding_table.rs`; it retains the recovery implementation and binding rows while restoring BASE rulings and selection for C/C++ declarator chains. The recovery-only tests remain compiled and 18 are explicitly ignored with `chain recovery parked: DECISIONS §P.1 / #530`; active park tests and the six Tier-A fixtures assert BASE labels.

Fix 1's C++ non-writing initializer normalization is restricted to nodes belonging to declarator chains. The final census and 130-fixture label-delta run show zero movement against BASE, including zero single-declaration or non-C/C++ movement. Persisted DFG labels changed from the activation commit, so `CACHE_VERSION` advances from 134 to 135.

### RED and gates

| Gate | Result |
|---|---|
| R2 RED at `5e5003ae` | 5 / 5 copied probes mismatched frozen BASE labels |
| R2 parked result | 5 / 5 copied probes match frozen BASE whole-edge sets |
| Focused enumeration | 218 passed / 0 failed / 18 ignored |
| Full suite, exhaustive traversal | 7,311 passed / 25 failed / 32 ignored |
| Full failure-name diff vs same-environment BASE | zero; exactly the 25 controller carries |
| Doc tests | 2 passed / 0 failed / 0 ignored |
| Phase-0 byte control | 1,645 / 1,645 identical |
| Item-2 byte control | 280 / 280 identical |
| Tier-A matrix | 251 ok / 0 gap / 0 fail |
| Label delta vs `39a6e9cb` | 130 fixtures / 0 changed / 0 errors |
| Clippy | 280 warning headers / 0 errors, equal to base |
| Formatting / diff check | passed |

The ordinary full-suite command stopped after the library target at 3,052 passed / 21 failed / 29 ignored; `--no-fail-fast` traversed the same requested all-target/all-feature suite to obtain the complete totals and failure-set comparison above.

### Census vs BASE `39a6e9cb`

| corpus | base/head edges | changed |
|---|---:|---:|
| Redis `src` | 413,789 / 413,789 | 0 |
| FRR `lib` | 109,081 / 109,081 | 0 |
| LevelDB `db` | 12,597 / 12,597 | 0 |
| LevelDB `util` | 5,410 / 5,410 | 0 |
| LevelDB `table` | 2,325 / 2,325 | 0 |
| **Total** | **543,202 / 543,202** | **0** |

All touched implementation and test files are at most 600 weighted lines except `src/cpg_cache.rs`, the pre-existing cache-version carrier at 1,639.5 weighted lines; this round changes only its version boundary, comment, and boundary tests.
