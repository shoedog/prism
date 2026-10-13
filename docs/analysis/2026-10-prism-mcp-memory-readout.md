# prism-mcp memory readout

This is **coarse triage**: it ranks reduction candidates for an experiment and selects none.
Footprints, peaks, live heap and the tree-sitter tree heap (M1) are measured on this process; the M2
figure is a file size; the string figures (M3, "attributed") are estimates from component counts.
A value that is unavailable is shown as n/a, never as 0.

> **Read the last section first.** The "Steady state" column below is the footprint 5 s after the workload. On
> this host that figure still includes memory the process has already freed: it is 19–23 % above the live
> heap. The verdicts do not depend on it. Everything above the last section is generated
> by `scripts/memreport/memreport.py render`; the last section is written by hand.

## Provenance

- Binary: build identity `d9414cb7b8bcddf4c7df5747af180c396f9c8ae4d66cbdc3ca7becb5baac40af`, git `7db8f1951166`, CPG cache version 114, allocator system, binary_input_dirty false.
- Environment of the measured process: none.
- Worker threads (rayon pool) of the measured process: 15.
- 3 repetitions per kind, 5 s settle. Values are median (min–max, n) in MiB.

| Corpus | Commit | Dirty files | Admissible runs (warm/cold/workload/refresh) |
|---|---|---|---|
| excalidraw | `0642e72cfa2d` | 0 | 3/3/3/3 |
| prometheus | `505095b64b43` | 0 | 3/3/3/3 |
| ruff | `44f6d1865fd2` | 0 | 3/3/3/3 |
| slicing | `7db8f1951166` | 0 | 3/3/3/3 |

## Footprint

| Corpus | Steady state (cache-loaded) | Steady live heap | Peak through index load | Observed load overshoot (lower bound; 0 is uninformative) | Refresh peak | Cold peak (no cache write) | First-build peak (writes the cache, n=1) |
|---|---|---|---|---|---|---|---|
| excalidraw | 707.3 (707.0–707.3, n=3) | 550.4 (550.3–550.5, n=3) | 731.9 (728.1–733.3, n=3) | 31.8 (27.9–33.1, n=3) | 1399.6 (1396.5–1399.6, n=3) | 835.6 (835.0–836.1, n=3) | 1083.9 |
| prometheus | 3096.4 (3094.2–3097.8, n=3) | 2381.7 (2381.7–2381.7, n=3) | 3107.5 (3105.6–3111.4, n=3) | 42.4 (40.4–46.3, n=3) | 5836.0 (5827.2–5838.3, n=3) | 3885.0 (3850.8–3930.2, n=3) | 5183.0 |
| ruff | 2919.2 (2916.4–2920.2, n=3) | 2363.5 (2363.4–2363.5, n=3) | 2878.3 (2873.3–2879.3, n=3) | 56.5 (52.1–57.9, n=3) | 5860.7 (5602.6–5944.6, n=3) | 3102.1 (3062.7–3166.9, n=3) | 4288.2 |
| slicing | 2051.6 (2050.9–2052.7, n=3) | 1577.9 (1577.9–1577.9, n=3) | 2016.5 (2014.6–2017.9, n=3) | 28.3 (24.4–29.9, n=3) | 3976.7 (3972.9–4007.6, n=3) | 2423.8 (2344.5–2447.1, n=3) | 3092.4 |

## Attribution

| Corpus | Files | Source (MiB) | Visible tree nodes | Tree heap, measured (M1) | CPG nodes / edges | Repeated node strings, estimate (M3) | Unique-byte ratio | Index live heap | Attributed (estimate) | Not attributed |
|---|---|---|---|---|---|---|---|---|---|---|
| excalidraw | 628 | 6.6 | 1339707 | 124.1 (124.1–124.1, n=3) | 201596 / 265195 | 15.8 (15.8–15.8, n=3) | 0.018 | 402.6 (402.5–402.6, n=3) | 42.2 | 360.3 |
| prometheus | 975 | 12.2 | 3718473 | 316.5 (316.5–316.5, n=3) | 683581 / 1305691 | 49.8 (49.8–49.8, n=3) | 0.011 | 2001.1 (2001.1–2001.1, n=3) | 139.0 | 1862.1 |
| ruff | 4881 | 25.5 | 5789053 | 559.2 (559.2–559.2, n=3) | 661335 / 1224731 | 58.4 (58.4–58.4, n=3) | 0.025 | 1609.9 (1609.9–1609.9, n=3) | 145.7 | 1464.2 |
| slicing | 1474 | 13.6 | 3669542 | 418.9 (418.9–418.9, n=3) | 508205 / 821949 | 39.6 (39.6–39.6, n=3) | 0.024 | 1078.4 (1078.4–1078.4, n=3) | 106.4 | 972.0 |

## Decision rule

A candidate advances to its experiment when its estimate is at least 200 MiB or 15 % of its base
on at least two corpora. Each row shows the numbers the rule was applied to.

| Candidate | Corpus | Estimate (MiB) | Base (MiB) | Estimate / base | Arm met | n (estimate / base) | Note |
|---|---|---|---|---|---|---|---|
| M1 | excalidraw | 124.1 | 707.3 | 17.6 % | fraction | 3 / 3 |  |
| M1 | prometheus | 316.5 | 3096.4 | 10.2 % | absolute | 3 / 3 |  |
| M1 | ruff | 559.2 | 2919.2 | 19.2 % | both | 3 / 3 |  |
| M1 | slicing | 418.9 | 2051.6 | 20.4 % | both | 3 / 3 |  |
| M2 | excalidraw | 135.6 | 731.9 | 18.5 % | fraction | file / 3 |  |
| M2 | prometheus | 692.7 | 3107.5 | 22.3 % | both | file / 3 |  |
| M2 | ruff | 519.0 | 2878.3 | 18.0 % | both | file / 3 |  |
| M2 | slicing | 350.9 | 2016.5 | 17.4 % | both | file / 3 |  |
| M3 | excalidraw | 15.8 | 707.3 | 2.2 % | — | 3 / 3 |  |
| M3 | prometheus | 49.8 | 3096.4 | 1.6 % | — | 3 / 3 |  |
| M3 | ruff | 58.4 | 2919.2 | 2.0 % | — | 3 / 3 |  |
| M3 | slicing | 39.6 | 2051.6 | 1.9 % | — | 3 / 3 |  |

- **M1** estimate: measured live tree-sitter heap after the workload and settle, an upper bound on what dropping the trees frees (it includes a fixed ~54 KB query cache, and trees that are re-parsed later would come back). Base: steady-state footprint.
- **M2** estimate: size of the CPG cache file, an upper bound on the buffer that streaming the load would remove (one measurement per corpus). Base: peak through the index load.
- **M3** estimate: estimated heap of repeated node strings. Base: steady-state footprint.

| Candidate | Corpora meeting the rule | Advances |
|---|---|---|
| M1 | excalidraw, prometheus, ruff, slicing | yes |
| M2 | excalidraw, prometheus, ruff, slicing | yes |
| M3 | — | no |

## Excluded runs

None.

## What this run does and does not show

Run `run-1`, 2026-10-13, macOS 26.6.2 on an Apple M5 Pro with 24 GiB of memory, system allocator, release
build of `main` at `7db8f195`. All 48 measured runs and the four priming runs were admissible. Evidence:
`~/prism-evidence/mcp-memory/c1/` (`run-1/`, `settle60/`, `idle-probe/`).

### Verdict

- **M1 (drop retained tree-sitter trees) advances** on all four corpora: 124–559 MiB of live tree-sitter heap.
- **M2 (stream the cache load) advances** on all four corpora: the cache file is 136–693 MiB, 17–22 % of the
  peak through the index load.
- **M3 (intern repeated node strings) does not advance** on any corpus: 16–58 MiB, about 2 % of the base.
  The strings are highly repetitive (unique-byte ratio 0.011–0.025), but there are not enough bytes in them
  to matter.

### The "Steady state" column is not an idle figure

`phys_footprint` after a cache load has two states on this OS, and the process does not control when it moves
from one to the other. In the first state the footprint still counts memory that was freed during the load. In
the second the OS has taken that memory back. The live heap (bytes the allocator reports in use) is the same
in both states to within 0.1 MiB.

| Corpus | Footprint, 5 s settle (`run-1`, median of 3) | Footprint, 60 s settle (supplement, each of 3 runs) | Steady live heap | Footprint − live heap at 5 s | CPG cache file |
|---|---|---|---|---|---|
| excalidraw | 707.3 | 707.1, 707.1, 707.1 | 550.4 | 156.9 | 135.6 |
| prometheus | 3096.4 | **2307.8**, **2307.5**, 3094.8 | 2381.7 | 714.7 | 692.7 |
| ruff | 2919.2 | 2920.4, 2920.1, 2914.2 | 2363.5 | 555.7 | 519.0 |
| slicing | 2051.6 | 2053.0, **1591.9**, 2051.9 | 1577.9 | 473.7 | 350.9 |

Values in MiB; bold marks a run that was sampled in the second state.

What was observed:

- With the driver's 5 s settle, 35 of the 36 cache-loaded runs in `run-1` (warm, workload and refresh kinds)
  were sampled in the first state. The exception is slicing refresh #3 (1589.8 MiB before the refresh).
- With a 60 s settle, 3 of 12 runs were sampled in the second state. A longer settle therefore does not fix
  the measurement. In one slicing run the footprint was already at 1590.2 MiB one stage after the load, so
  the move is not a fixed delay either.
- A `prism-mcp` in normal serving mode (not report mode) on the slicing corpus, sampled from outside with
  `ps` and `footprint`, peaked at 2016 MiB and then sat at 1529 MiB for ten minutes without change. In a
  second such process the resident size fell from 2027 to 1597 MiB about 18 s after the load. Report mode is
  therefore not the cause.
- Where the second state was observed, its footprint is within 1–3 % of the live heap (slicing 1568–1592
  against 1578; prometheus 2308 against 2382). It was not observed for excalidraw or ruff, so for those two
  "idle footprint ≈ live heap" is an inference from the other two corpora, not a measurement.
- The excess at 5 s (157–715 MiB) tracks the size of the CPG cache file, which is 74–97 % of it. That fits
  the cache file buffer and other load-time temporaries being the freed memory, but the run does not
  identify the freed allocations, so this is an association and not a proof.

What follows from it:

- **The verdicts hold under either base.** Against the steady live heap M1 is 22.5 % (excalidraw), 13.3 %
  (prometheus, which still meets the 200 MiB arm), 23.7 % (ruff) and 26.5 % (slicing); M3 is 2.1–2.9 %.
  M2's base is a peak, which is a high-water mark and is not affected.
- **The spec's pinned steady-state metric (`phys_footprint` after a 5 s settle, spec §6.1) is not fit for a
  C2 experiment.** A candidate could appear to gain or lose 460–790 MiB depending only on which state each
  run was sampled in, and the rule "beat the baseline's max by the candidate's min" would then decide on
  that accident. The steady live heap varied by at most 0.2 MiB across the repetitions of a corpus and is
  the better primary metric. Changing a pinned parameter is the owner's decision; it is raised in the lane
  README.
- For sizing a host, the number that matters after the first minutes is close to the live-heap column:
  0.5 GiB (excalidraw), 2.3 GiB (prometheus), 2.3 GiB (ruff), 1.5 GiB (slicing) per resident index.

### What the load transient is

On the two corpora where the second state was observed, the peak exceeds the idle footprint by 425–479 MiB
on slicing (21–23 % of a peak of 2017–2047 MiB, two runs) and by 803 MiB on prometheus (26 % of a 3111 MiB
peak, two runs). That is larger than the cache file (351 and 693 MiB), so streaming the load (M2) can remove
at most the file-sized part of it.
The "observed load overshoot" column (28–57 MiB) is small only because the freed memory was still being
counted when the next sample was taken; it is a lower bound, as its header says.

### Comparison with the 2026-10-09 figures

The spec (§1) recorded 1,168 MB resident for an idle `prism-mcp` and a 1,620 MB peak for a warm load on the
slicing repository, then 1,232 files, 420k CPG nodes and 672k edges. Today's slicing corpus is 1,474 files,
508k nodes and 822k edges (about 21 % larger), and the cache format has moved to version 114.

| | 2026-10-09 | Scaled by 1.21 | Today |
|---|---|---|---|
| Idle, index built | 1,168 MB resident | 1,413 | 1,529 MiB footprint / 1,596 MiB resident (normal mode, second state, call-edge index not yet built) |
| Peak, warm load | 1,620 MB | 1,960 | 2,016 MiB (index load), 2,047 MiB (with the call-edge index) |

The peak agrees to 3 %. The idle figure agrees to 8–13 % once the second state is compared; that remainder
is not attributed. Compared against the 5 s "Steady state" column instead (2,052 MiB), the gap would be 45 %,
which is how the two-state behaviour was found. The 2026-10-09 figures are in the units the spec recorded;
how long that process had been idle was not recorded.

### Other observations

- **A refresh costs about a second index.** The refresh peak is 1.9–2.0 times the 5 s footprint on every
  corpus (1.4, 5.7, 5.7 and 3.9 GiB), close to the idle index plus a cold build. For a shared daemon this
  is the number that sizes the host, not the idle footprint. Most refresh runs started in the first state,
  so the peak a long-idle daemon would reach was not measured separately.
- **Most of the index is not covered by any candidate.** Of the index live heap, 90–93 % (360–1,862 MiB) is
  "not attributed": it is neither inline node storage nor node strings. Tree-sitter trees are counted
  separately and are 13–27 % of the steady live heap. No current candidate addresses the rest, and finding
  one would need allocation-level attribution that this harness does not have.
- **An index built in the process holds more live heap than the same index loaded from the cache**: 80, 181,
  283 and 227 MiB more (excalidraw, prometheus, ruff, slicing; cold against warm at the settle stage). Not
  investigated.
- **A cold build of ruff took 13 minutes** (779–802 s, three runs) and the priming build, which also writes
  the cache, took 23 minutes (1,378 s), on 15 worker threads. Slicing and prometheus took about 2.3 minutes.
  This is a time observation from report mode, which counts every tree-sitter allocation; whether normal
  mode is as slow was not measured.

### Kubernetes was not run

The plan names five corpora. Kubernetes (`8d34ac5ef882`, 30,485 tracked files, roughly 17,600 source files
and 169 MiB of source by file extension) was left out because the four measured corpora project it past
this host's memory. Their first-build peak is 164–426 times the source size, and the Go corpus (prometheus)
is the 426. That projects a first build of 27–70 GiB and a live heap of 14–32 GiB on a host with 24 GiB
of memory and 5 GiB of its 6 GiB swap already in use. The decision rule needs two corpora and has four.
Running kubernetes needs a larger host or a subtree of the repository; that choice is the owner's.

### Limits

- One host, one OS, one allocator. Nothing here was measured on Linux, where freed memory is returned on a
  different schedule and `heap_in_use_bytes` has a different definition (see `scripts/memreport/README.md`).
- Three repetitions per kind; the first-build peak is one run per corpus.
- M1 is a measured upper bound (trees that are re-parsed later would come back). M2 is a file size, an
  upper bound on the buffer that streaming removes. M3 and the "attributed" column are estimates from
  component counts.
- Host swap was 5.0–5.1 GB of 6.1 GB in use throughout. No warm or workload run had compressed pages at
  any stage, so the steady figures were not affected by compression. Two of three ruff cold runs, the
  prometheus and ruff priming runs, and one refresh run each on prometheus and ruff had 31–695 MiB
  compressed at a late stage. Footprints and peaks count compressed pages, so they are not understated;
  resident sizes in those runs are.
- The 60 s supplement (`settle60/`) ran only the workload kind, through the driver's own command builder
  and validator but outside `run`: it has no priming run and no `summary.json`. Its script is stored with
  its raw reports.
- The normal-mode observations (`idle-probe/`) are two processes on one corpus.
