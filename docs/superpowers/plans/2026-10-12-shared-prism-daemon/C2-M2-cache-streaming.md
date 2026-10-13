# C2-M2 — Stream the CPG cache file: experiment plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Find out, by a pinned baseline-against-candidate measurement, whether writing and reading `cpg-cache.bin` as a stream lowers the two peaks that size a host for one prism index (the cache-load peak and the first-build peak) without making either operation slower, and ship the change if it does.

**Architecture:** The candidate is three small changes in `src/cpg_cache.rs` plus one new file, `src/cpg_cache/stream.rs`: the save path encodes straight from the live graph into a buffered file; the load path decodes from a buffered file under a byte bound; and the load path moves decoded nodes and edges into the graph instead of cloning them. The file's bytes do not change, so `CACHE_VERSION` stays at 114 and an existing cache keeps loading. The measurement uses the C1 harness (`scripts/memreport/`), extended by a `build` kind, a `--kinds` selector and a `compare` command.

**Tech Stack:** Rust 2021, `bincode` 1.3.3 (`serialize_into`, `Options::with_limit`), `serde`, `petgraph`; Python 3 standard library for the driver.

**Spec:** `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §6.1 (pinned parameters, revised 2026-10-13), §6.2 (candidate M2 and the shipping gate), §11 items 8 and 9. Readout this plan starts from: `docs/analysis/2026-10-prism-mcp-memory-readout.md`.

## Global Constraints

- **The byte format of `cpg-cache.bin` does not change.** `CACHE_VERSION` stays 114. A test holds the streamed file equal, byte for byte, to `bincode::serialize(&CpgCache)`.
- **Behaviour on a damaged cache file does not change:** a missing, truncated or corrupt file is a miss, never a panic and never an allocation sized by the file's contents.
- M2 is judged on **peaks** (`peak_bytes`), per spec §6.1 as revised. Steady live heap must not rise; it is a check, not the gate.
- Three repetitions per kind, median with min and max, one measured process at a time, 5 s settle (spec §6.1).
- Four corpora at pinned commits: slicing `7db8f195` (the worktree `~/code/prism-c1-corpus`), excalidraw `0642e72cfa2d`, prometheus `505095b64b43`, ruff `44f6d1865fd2`. Kubernetes is not run (spec §11 item 9).
- Baseline and candidate are measured back to back on the same host, each from a release build, and differ only by this plan's source change.
- Before a measurement starts: `pgrep -fl 'prism-mcp|cargo|nextest|mutgate'` shows no build, test or other measurement. If it does, wait. The readout records host state before every run (the driver does this).
- `src/cpg_cache.rs` is on the review path too (`src/api/review.rs` saves and loads through the same two functions), so the full suite, the mutation gate and the Tier-A matrix check all apply, not only the MCP tests.
- Keep every new file under 600 lines. `src/cpg_cache.rs` is already 1,659 lines; this plan makes it shorter and adds nothing to it but calls.
- Logs go to files; read the file. Do not pipe cargo output into `grep` or `tail`.

## What C1 measured, and what the code does

From the readout (`run-1`, binary `7db8f195`), in MiB:

| Corpus | Cache file | Load peak | Idle after load (where seen) | Load transient | Cold peak (no cache write) | First-build peak (writes the cache, n=1) | Write transient (first-build − cold) |
|---|---|---|---|---|---|---|---|
| excalidraw | 135.6 | 731.9 | not observed | — | 835.6 | 1083.9 | 248 |
| prometheus | 692.7 | 3107.5 | 2308 | 803 | 3885.0 | 5183.0 | 1298 |
| ruff | 519.0 | 2878.3 | not observed | — | 3102.1 | 4288.2 | 1186 |
| slicing | 350.9 | 2016.5 | 1568–1592 | 425–479 | 2423.8 | 3092.4 | 668 |

The readout attributed the load transient to "the cache file buffer and other load-time temporaries" without identifying them. Reading the code identifies three, all in `src/cpg_cache.rs` at `e2701e89`:

1. **Load: the file's bytes.** `load_cache_with_topology` does `fs::read(&bin_path)` and then `bincode::deserialize(&data)`. `data` is a local of the whole function, so the file's bytes (the "Cache file" column) stay allocated until the function returns, which is after the graph is rebuilt.
2. **Load: a second copy of every node and edge.** `reconstruct_cpg(ser)` takes the decoded lists by value but iterates them by reference: `graph.add_node(node.clone())` and `edge.clone()`. Until `ser` is dropped at the end, every node (with its strings) and every edge exists twice. The graph also grows from empty, so its two backing vectors reallocate as they double.
3. **Save: a copy of the whole index, then the encoded bytes.** `save_cache_with_topology` builds an owned `CpgCache`: a cloned node list, a cloned edge list, `cpg.call_graph.clone()`, `cpg.dfg.clone()`. It then calls `bincode::serialize(&cache)`, which returns the whole file as one `Vec<u8>`, and only then writes it. At that moment the process holds the live index, a full copy of it, and the file. This is the larger transient (the last column) and the spec's M2 row does not mention it; it is why a first build peaks 30–38 % above a cold build that writes nothing.

Slicing fits the arithmetic: file 351 MiB plus duplicated nodes and edges (about 110 MiB by the readout's attribution counters: 69 MiB of inline node storage and about 40 MiB of node strings) is about 460 MiB, against a measured load transient of 425–479 MiB.

A fourth copy exists in `src/navigation/call_edge_cache.rs` (`index.clone()` on save, `fs::read` on load of the call-edge sidecar, 4.6–64 MiB). It is **out of scope**: in every priming run the lifetime peak at the call-edge stage equals the peak at the index stage, so the sidecar does not set a peak today.

## The candidate

One new file and three edits. The code is on the validated branch `scratch/c2-m2-validate` (commit recorded in the lane README); the central pieces are reproduced here because they are what a reviewer has to agree with.

### Save: encode from the live graph into the file

`src/cpg_cache/stream.rs` defines borrowed mirrors of the two cache structs, with the same fields in the same order, and two sequence adapters that encode the graph's nodes and edges exactly as the owned vectors encode:

```rust
#[derive(Serialize)]
pub(super) struct CpgCacheRef<'a> {
    pub(super) version: u32,
    pub(super) prism_version: &'a str,
    pub(super) grammar_fingerprint: &'a str,
    pub(super) cache_build_identity: &'a str,
    pub(super) git_sha: &'a str,
    pub(super) skip_policy_version: u32,
    pub(super) file_hashes: &'a BTreeMap<String, String>,
    pub(super) topology_key: &'a BTreeMap<String, String>,
    pub(super) has_type_db: bool,
    pub(super) graph: SerializedCpgRef<'a>,
}

/// The graph's nodes in index order, encoded as `Vec<CpgNode>` is.
pub(super) struct NodeSeq<'a>(pub(super) &'a DiGraph<CpgNode, CpgEdge>);

impl Serialize for NodeSeq<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.node_count()))?;
        for index in self.0.node_indices() {
            seq.serialize_element(&self.0[index])?;
        }
        seq.end()
    }
}
```

`EdgeSeq` is the same over `edge_indices()`, each element `&(source as u32, target as u32, &edge)`; in bincode a reference encodes as its referent, and a 3-tuple has no length prefix, so the bytes are those of `(u32, u32, CpgEdge)`. `write_cache_file` wraps a 1 MiB `BufWriter` in a byte counter (the metadata file records the size) and calls `bincode::serialize_into`. If it fails, the caller removes the temp file; the rename to `cpg-cache.bin` happens only after a complete write, as before.

The sequence length is passed explicitly because bincode refuses a sequence of unknown length.

### Load: decode from the file, bounded by the file's length

```rust
pub(super) fn read_cache_file(path: &Path) -> Option<CpgCache> {
    let file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let reader = BufReader::with_capacity(IO_BUFFER_BYTES, file);
    // `bincode::deserialize`'s configuration (fixed-width integers, trailing bytes allowed),
    // plus the bound.
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(len)
        .deserialize_from(reader)
        .ok()
}
```

**The bound is not optional.** bincode's slice decoder refuses a string whose length prefix exceeds the bytes that remain. Its stream decoder cannot know what remains, so it allocates what the prefix says and then reads. A damaged prefix of 2^62 would abort the process where today it is a cache miss. `with_limit(len)` makes the decoder refuse any read that would take the total past the file's own length, before allocating. The top-level `bincode::deserialize` uses fixed-width integers and allows trailing bytes; `DefaultOptions` alone would use variable-width integers and misread every file, which is why both are spelled out.

### Load: move, do not clone

```rust
let mut graph = DiGraph::with_capacity(ser.nodes.len(), ser.edges.len());
for node in ser.nodes {
    graph.add_node(node);
}
for (src, tgt, edge) in ser.edges {
    graph.add_edge(NodeIndex::new(src as usize), NodeIndex::new(tgt as usize), edge);
}
```

Order is unchanged, so every `NodeIndex` and `EdgeIndex` is what it was.

### What the candidate does not do

- It does not decode directly into the graph. `ser.nodes` and the graph's node vector still coexist while nodes are moved across, so the inline node storage (not the strings) is briefly held twice. Removing that needs a custom `Deserialize` for the graph; it is a follow-up only if the experiment shows the remaining transient matters.
- It does not memory-map the file. Mapping would keep the slice decoder's speed with no heap buffer, at the cost of a new dependency and `unsafe`. It is the **pre-registered fallback** if the stream reader fails the load-time criterion below and only then.
- It does not touch the call-edge sidecar (see above), the cache format, or what is cached.

## Preliminary result

One run of the prototype on slicing only, taken while the host was busy with unrelated builds (load average
40–70). It shows the direction and decides nothing. The baseline column is C1's `run-1` of the same day, a
different build of the same analyzer source. The binary measured is the prototype's first commit
(`c95161fd`), before the temp-file changes described below. Evidence:
`~/prism-evidence/mcp-memory/c2-m2/prelim-candidate/`. Values in MiB.

| slicing | Baseline (`run-1`) | Prototype | Difference |
|---|---|---|---|
| First-build peak (one build each) | 3092.4 | 2400.2 | −692 (−22 %) |
| Load peak (median of 3, range) | 2016.5 (2014.6–2017.9) | 1621.2 (1618.8–1623.4) | −395 (−20 %) |
| Footprint at the sample after the load | 1988–1990 | 1619–1623, equal to the peak | |
| Live heap after the load | 1524.1 | 1516.7 | −7.4 |
| Index load time, ms | 1038, 1046, 1051 | 1138, 1265, 3355 | not comparable on a busy host |
| Cache file, bytes | 367,898,078 | 367,898,078 | identical |

What it suggests, to be confirmed by the pinned experiment:

- The first-build peak falls to the level of a cold build that writes nothing (2,424 MiB median in `run-1`):
  the write transient is gone.
- The load peak falls by about the size of the cache file plus part of the duplicated nodes and edges.
- After the load the footprint equals the peak, so the load leaves nothing freed-but-still-counted behind.
  If that holds, the "higher footprint level" the readout found after a cache load mostly disappears with
  this change, which matters more to a host than the peak does. The experiment reports it; it is not a
  criterion.
- The live heap is slightly lower (the graph is built with exact capacity). That is why P5 below is
  one-sided.
- All three prototype load times are above the baseline's. The host was busy, so this proves nothing, but a
  stream decoder is expected to cost something, and P3 exists to find out how much.

Two things were added to the prototype after this run, because writing the cache now takes as long as
encoding it rather than one `write` call:

- **A temp file name unique to the writer** (`cpg-cache.bin.<pid>.<n>.tmp`). With the one shared name the
  code used, a second writer's `create` truncates the file under the first, which can then rename a file with
  a hole in it into place; a hole of zero bytes can decode. Several standalone servers building the same
  repository at once is exactly today's fan-out case. The call-edge sidecar already names its temp file this
  way.
- **Removal of temp files nobody has written to for 15 minutes**, at the start of each save. A unique name is
  never overwritten by the next writer, so without this every writer killed mid-save would leave a partial
  cache of hundreds of MiB behind for good.

## The experiment (pinned before it runs)

**Binaries.** Baseline: a release build of the commit the candidate branch starts from. Candidate: a release build of the candidate branch head. Both are copied to `~/prism-evidence/mcp-memory/c2-m2/bin/{baseline,candidate}/prism-mcp` before measuring, and each run's report carries its build identity, which the comparison checks.

**Runs.** For each binary, one driver run into a fresh directory:

```bash
python3 scripts/memreport/memreport.py run --bin <binary> \
  --corpus slicing=$HOME/code/prism-c1-corpus --corpus excalidraw=$HOME/code/bench-repos/excalidraw \
  --corpus prometheus=$HOME/code/bench-repos/prometheus --corpus ruff=$HOME/code/bench-repos/ruff \
  --workloads ~/prism-evidence/mcp-memory/c1/workloads \
  --kinds warm,workload,build --reps 3 --out ~/prism-evidence/mcp-memory/c2-m2/<baseline|candidate>-1
```

`warm` gives the load peak and the index load time; `workload` gives the steady live heap; `build` gives the first-build peak and the build time, each repetition into its own empty cache directory. `cold` and `refresh` are not run: neither is a criterion, and a cold build of ruff is 13 minutes. Order: baseline, then candidate, with nothing else running. Expected wall time: about 80 minutes per binary, most of it ruff's three first builds (about 17–23 minutes each).

**Budget.** Four hours of wall time for both runs. If ruff's first builds would exceed it, ruff's `build` kind is run once per binary and reported as "insufficient runs" for the first-build gate; the gate needs two corpora and the other three can supply them.

**Criteria**, applied by `memreport.py compare --experiment m2`:

| # | Metric | Rule |
|---|---|---|
| P1 | Load peak (`warm`, peak through the index load) | **Gate.** On at least two corpora: candidate max below baseline min, and the median falls by at least 100 MiB or 5 % of the baseline median. |
| P2 | First-build peak (`build`) | **Gate**, same form. |
| P3 | Index load time (`warm`) | **Not worse.** A corpus is *worse* only when the candidate median is above 1.05 times the baseline median **and** the two ranges do not overlap (candidate min above baseline max). The metric holds when no corpus is worse and every corpus has both series. The ratio of the medians is always printed. |
| P4 | Index build time (`build`) | **Not worse**, same form. |
| P5 | Steady live heap (`workload`) | **Not higher:** candidate median at most 1 MiB above the baseline median, on every corpus. A lower figure is fine: a graph built with exact capacity holds less slack. |

The not-worse rule was first written as "at most 1.05 times the baseline median" alone. The driver's review
measured the same build twice on this machine, once while other work was running: its load time differed by
15 %. A difference inside the overlap of the two ranges is not evidence in either direction, so the rule now
requires the ranges to separate, as the gates do. The price is that a regression smaller than the run-to-run
spread is not caught; the printed ratios and the recorded load average of each run are there so a reader can
see how tight the comparison was.

A `build` repetition counts only if it wrote both cache files (the driver checks their sizes, and the stderr
lines the binary prints when a save fails). The binary treats a failed save as a warning and exits 0, and a
candidate that fails to write would otherwise show the largest peak reduction of all.

A corpus measured on one side only makes a not-worse or not-higher verdict *incomplete*, never ok. A gate that
cannot reach two passing corpora because data is missing is *undecided*, not failed.

The spec's noise rule reads "a candidate must beat the baseline's max by its own min". Taken literally that passes whenever the two ranges touch. This plan uses the strict reading: the ranges must not overlap. Anything that passes the strict reading passes the literal one.

**Outcomes.**

- P1 and P2 pass, P3–P5 hold: the candidate ships as written.
- Any verdict is *incomplete* or *undecided*: nothing is decided. Find out why runs are missing, and measure again into a new directory.
- P2 passes and P1 fails or P3 fails: the save half ships alone (it does not affect load time), and the load half is retried once with the memory-mapped reader. That retry is the only one; a second failure closes the load half as "not worth it" in the readout.
- P2 fails: the candidate does not ship. The readout says so with the numbers.
- P5 fails: stop. A candidate that reads and writes a file differently has no reason to hold more live heap; that is a defect to find, not a result.

**What ships is decided by the table above, not by the preliminary result.**

## Shipping gate (spec §6.2): the fan-out check is waived for M2

Spec §6.2 requires, for every candidate, no regression on `uv run tier-a --quick`, on single-client navigation p50, and on fan-out latency with eight shims, the last of which needs slice S5a and so holds every candidate until then.

M2 changes how the cache file is read and written and nothing on the query path. The streamed file is byte-identical and the loaded graph is index-identical, both held by tests. **The owner accepted this on 2026-10-13 (spec §11 item 11):** M2 may ship on: the full test suite, the mutation gate, the Tier-A matrix check and `tier-a --quick`, the workload's response size and error count unchanged between baseline and candidate, and criteria P1–P5; and the fan-out gate, which measures query latency under concurrency, is waived for this candidate only. The M2 pull request merges when its reviews approve, CI is green and those conditions are met.

## Review Focus

1. **A damaged cache file.** A length prefix larger than the file, a file cut at any byte, and trailing bytes must behave as today (miss, miss, hit). The stream decoder's failure mode without the bound is an abort, which no test can catch after the fact; the test `a_length_prefix_larger_than_the_file_is_a_miss_without_allocating_it` is the one that would.
2. **Byte identity of the written file** across every field, including a graph with zero nodes or zero edges and maps with zero entries (an explicit sequence length of 0 must encode as the empty vector does).
3. **Index stability on load.** A node or edge that lands at a different index silently re-points every stored `NodeIndex`. The round-trip test compares each node and edge with its index.
4. **A failed write.** A full disk or an unwritable directory must leave the previous `cpg-cache.bin` in place and usable, and must not leave a partial file under the final name.
5. **Two writers, and a writer that dies.** Each writer now has its own temp file, so two processes saving one cache directory at once cannot truncate each other; the last rename wins and both files were complete. The review should confirm there is no other shared path, and that the 15-minute rule cannot remove the temp file of a writer that is alive but slow (a writer touches its file with every buffer flush, so the longest gap is the time to encode 1 MiB; a writer suspended for longer, for example across a laptop sleep, loses its temp file and its save fails at the rename, which costs a rebuild later and never a wrong hit).

## Tasks

### Task 1: Driver — selectable kinds, the `build` kind, `compare`

**Files:** `scripts/memreport/memreport.py`, `memreport_validate.py`, `memreport_fixtures.py`, new `memreport_compare.py` and tests, `README.md`.

**Interfaces:**
- Produces: `run --kinds LIST` (subset of `warm,cold,workload,refresh,build`; default the first four); kind `build` (fresh cache directory per repetition, admissible only when the report says neither cache was loaded; metrics `build_peak`, `build_ms`); metric `load_ms` on `warm`; `compare --experiment m2 --baseline DIR --candidate DIR --out FILE.md [--json FILE]`, which refuses two runs that differ in corpus commits, workload digests, repetitions, settle, thread count, environment or allocator, or that share a build identity, and applies criteria P1–P5.

Its full brief is `C2-M2-driver-brief.md` in this directory. It ships as its own pull request before the candidate, because the M1 experiment will use it too.

- [ ] **Step 1:** Implement from the brief, test-first, in a worktree of its own. Python only.
- [ ] **Step 2:** `python3 -m unittest discover -s scripts/memreport -p 'test_*.py'`. Expected: the 145 existing tests and the new ones pass.
- [ ] **Step 3 (controller):** smoke-run `--kinds warm,build --reps 1` with a real release binary on excalidraw. Expected: `build #1: ok`, `warm #1: ok`, the repetition's cache directory gone, `build_peak` above `peak`.
- [ ] **Step 4:** Review (Opus 5.5 and gpt-6.1-sol, cap two rounds), merge on approval and green CI.

### Task 2: The candidate

**Files:**
- Create: `src/cpg_cache/stream.rs`, `src/cpg_cache/stream_tests.rs`
- Modify: `src/cpg_cache.rs` (`mod stream;`, the body of `save_cache_with_topology`, the first lines of `load_cache_with_topology`, the first loop of `reconstruct_cpg`)

**Interfaces:**
- Consumes: `CpgCache`, `SerializedCpg`, `cache_bin_path`, `cache_meta_path` (private items of `cpg_cache`, visible to its child module).
- Produces: `stream::write_cache_file(path, &CpgCacheRef) -> io::Result<u64>`, `stream::read_cache_file(path) -> Option<CpgCache>`. No public API changes.

- [ ] **Step 1:** Start the branch from `scratch/c2-m2-validate` rebased onto `main`.
- [ ] **Step 2:** Run `cargo test --lib cpg_cache:: > <log> 2>&1`. Expected: the 16 existing tests of the module and the 13 new ones pass (29; measured on `c7c92790`). The new ones:
  - `the_streamed_file_equals_the_owned_encoding_byte_for_byte`
  - `a_graph_with_no_nodes_or_edges_streams_to_the_owned_encoding_and_loads_back`
  - `a_file_in_the_owned_encoding_loads_as_a_hit`
  - `a_round_trip_keeps_every_node_and_edge_at_its_index`
  - `the_metadata_records_the_size_of_the_file_written`
  - `a_truncated_file_is_a_miss`
  - `a_length_prefix_larger_than_the_file_is_a_miss_without_allocating_it`
  - `trailing_bytes_after_a_valid_cache_do_not_make_it_a_miss`
  - `a_save_leaves_no_temp_file_behind`
  - `every_writer_gets_its_own_temp_name`
  - `a_write_that_fails_part_way_leaves_nothing_at_its_path`
  - `a_dead_writers_temp_file_is_removed_and_a_live_writers_is_kept`
  - `a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable` (Unix)
- [ ] **Step 3: prove the tests bite.** After committing, one at a time, restoring with `git checkout -- <file>`. All eight were run on `c7c92790` and each failed as stated:
  - remove `.with_limit(len)` → the test binary aborts with "memory allocation of 4611686018427387904 bytes failed" in `a_length_prefix_larger_than_the_file…`;
  - remove `.with_fixint_encoding()` → six tests fail, among them the empty-graph and round-trip tests;
  - swap two fields in `CpgCacheRef` → six tests fail, among them the byte-identity tests;
  - iterate `node_indices().rev()` in `NodeSeq` → the byte-identity test, the index test and `v5_round_trip_preserves_byte_identity` fail;
  - drop the `remove_file` in `write_new_file` → `a_write_that_fails_part_way_leaves_nothing_at_its_path` fails;
  - return the fixed name from `unique_temp_path` → `every_writer_gets_its_own_temp_name` fails;
  - change the stale age comparison from `>=` to `>` → `a_dead_writers_temp_file_is_removed_and_a_live_writers_is_kept` fails;
  - skip `remove_stale_temp_files` in the save → the same test fails.
  Re-run them if the code changes in review.
- [ ] **Step 4:** Decide whether the stale-temp cleanup also covers the call-edge sidecar's temp files (`resolved-call-edge-index.bin.<pid>.<n>.tmp`), which have the same leak today. Default: no, it is a separate file's code and this plan leaves the sidecar alone; record it in the PR as a known leak.
- [ ] **Step 5:** Full verification: `cargo fmt --check`; `cargo test --features mcp`; `cargo check`; `cargo clippy --all-targets --features mcp` (no new warning in the touched files; the base prints about 258 lines); `python3 scripts/mutgate/mutgate.py`; `cargo build --release && (cd eval && uv run tier-a --matrix-only --allow-stale-sut)`.
- [ ] **Step 6:** Commit.

### Task 3: Measure

- [ ] **Step 1:** Build both release binaries, copy them into the evidence directory, record `shasum -a 256` of each and the two commits.
- [ ] **Step 2:** Check the host is quiet (Global Constraints). Run baseline, then candidate, with the command above.
- [ ] **Step 3:** `python3 scripts/memreport/memreport.py compare --experiment m2 --baseline …/baseline-1 --candidate …/candidate-1 --out docs/analysis/2026-10-c2-m2-cache-streaming-readout.md --json …/compare.json`.
- [ ] **Step 4:** Check the comparison against the raw reports by hand for one corpus and one metric of each role. Add a hand-written section: the outcome under "Outcomes" above, every excluded run and why, host state, what was not measured. Write `SHA256SUMS` over the evidence directory.

### Task 4: Pull request

- [ ] **Step 1:** Open the PR with the comparison's verdict table in the body. Review by Opus 5.5 and gpt-6.1-sol, cap two rounds; reviewers get the evidence directory and are asked to recompute.
- [ ] **Step 2:** Run `cd eval && uv run tier-a --quick --allow-stale-sut` on the candidate's release build and compare the workload's response size and error count between the two runs. Merge only when the reviews approve, CI is green and the criteria pass (the fan-out check is waived, spec §11 item 11).

## Self-review record

- **Spec coverage:** §6.2 row M2 ("stream the cache deserialization"; "peak reduction on two corpora and load time not worse") → the candidate's load half, P1, P3. The save half and P2, P4 go beyond the row; they are in the plan because the code reading found the larger transient there. §6.1 as revised (M2 judged on a peak; three repetitions; noise rule; four corpora) → "The experiment". §6.2 shipping gate → its own section, with the one departure raised as a question, not taken.
- **Not in this plan:** the call-edge sidecar; direct decoding into the graph; a memory-mapped reader (fallback only); the 13 s cache load on prometheus (below).
- **Observation recorded, not pursued:** a cache load of prometheus takes 13.2 s where ruff, with a file three quarters the size, takes 1.6 s and slicing 1.0 s. Something in that load is not proportional to file size. It is outside M2 and is listed in the lane README for whoever plans load-time work.
