# prism-mcp memory readout

This is **coarse triage**: it ranks reduction candidates for an experiment and selects none.
Footprints, peaks, live heap and the tree-sitter tree heap (M1) are measured on this process; the M2
figure is a file size; the string figures (M3, "attributed") are estimates from component counts.
A value that is unavailable is shown as n/a, never as 0.

> **Read the last section first.** The "Steady state" column below is the footprint 5 s after the workload. On
> this host 19–23 % of that figure is not live heap (it is 24–30 % above the live heap), and in some runs
> that excess is gone by the time of the sample. The verdicts do not depend on it. Everything above the last section is generated
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

After a cache load `phys_footprint` was seen at two levels, and a run's settle time did not decide which. At
the first level the footprint exceeds the live heap (bytes the allocator reports in use) by 157–715 MiB. At
the second it is within 14 MiB of the live heap (slicing) or 74 MiB below it (prometheus). The live heap
itself is the same at both levels to within 0.1 MiB. What moves a process from the first level to the second
was not identified, and whether the process can force the move was not tried.

| Corpus | Footprint, 5 s settle (`run-1`, median of 3) | Footprint, 60 s settle (supplement, each of 3 runs) | Steady live heap | Footprint − live heap at 5 s | CPG cache file |
|---|---|---|---|---|---|
| excalidraw | 707.3 | 707.1, 707.1, 707.1 | 550.4 | 156.9 | 135.6 |
| prometheus | 3096.4 | **2307.8**, **2307.5**, 3094.8 | 2381.7 | 714.7 | 692.7 |
| ruff | 2919.2 | 2920.4, 2920.1, 2914.2 | 2363.5 | 555.7 | 519.0 |
| slicing | 2051.6 | 2053.0, **1591.9**, 2051.9 | 1577.9 | 473.7 | 350.9 |

Values in MiB; bold marks a run sampled at the second level.

What was observed:

- With the driver's 5 s settle, 35 of the 36 cache-loaded runs in `run-1` (warm, workload and refresh kinds)
  were sampled at the first level. The exception is slicing refresh #3 (1589.8 MiB before the refresh).
- With a 60 s settle, 4 of 13 runs were sampled at the second level: 3 of the 12 supplement runs, and one
  separate report-mode run on slicing (`idle-probe/a-stderr.txt`, 1568.0 MiB). A longer settle therefore
  does not make the figure repeatable.
- The move is not a fixed delay. In two slicing runs (`run-1` refresh #3 and supplement workload #2) the
  footprint was already at 1528 MiB at the sample taken right after the index load, against 1988–1990 MiB in
  the other slicing runs.
- The second level is not an artifact of report mode. A `prism-mcp` in normal serving mode on the slicing
  corpus, sampled from outside with `ps` and `footprint`, peaked at 2016 MiB and then sat at 1529 MiB for
  ten minutes without change. In a second such process the resident size fell from 2027 to 1597 MiB between
  19 and 20 s after start. With two normal-mode processes, whether report mode changes the timing is not
  known.
- **Overlap, disclosed.** The first normal-mode process stayed resident (about 1.5 GiB) while the second
  normal-mode process, the separate report-mode run, the three excalidraw supplement runs and the first two
  prometheus supplement runs were measured. Both prometheus second-level samples come from those two runs;
  the third prometheus run, measured alone, stayed at the first level. Supplement slicing #2 and `run-1`
  slicing refresh #3 moved with no other measurement process running. Another resident index is the one
  visible correlate of the move in this data; it is not shown to be a cause.
- Where the second level was observed, its footprint is within 1–3 % of the live heap (slicing 1568–1592
  against 1578; prometheus 2308 against 2382). It was not observed for excalidraw or ruff (0 of 12 each), so
  for those two "idle footprint ≈ live heap" is an inference from the other two corpora.
- The excess is not specific to the cache load. The CPG cache file is 74–97 % of the excess, which fits
  the freed file buffer, but the excess is equally 24–30 % of the live heap, which fits "proportional to
  index size". Cold builds read no cache file and still settle 58–440 MiB above their live heap (slicing: 116, 291
  and 348 MiB in three runs). After the refresh the excess is 113–116 MiB (excalidraw), 90–99 MiB
  (prometheus), 435–490 MiB (ruff) and 951–974 MiB (slicing), and in two of the prometheus refresh runs
  the footprint is about 2.2 GiB below the resident size with almost nothing compressed, so memory left the
  footprint within that stage.

What follows from it:

- **The verdicts hold under either base.** Against the steady live heap M1 is 22.5 % (excalidraw), 13.3 %
  (prometheus, which still meets the 200 MiB arm), 23.7 % (ruff) and 26.5 % (slicing); M3 is 2.1–2.9 %.
  M2's base is the peak through the index load, which did not depend on the level: the runs that were
  already at the second level by then peaked inside the range of the others.
- **The spec's pinned steady-state metric (`phys_footprint` after a 5 s settle, spec §6.1) cannot gate a C2
  experiment as it stands.** In these runs a candidate could appear to gain or lose 460–790 MiB depending
  only on the level each run was sampled at, and the rule "beat the baseline's max by the candidate's min"
  would decide on that accident.
- **The steady live heap is repeatable but is an allocation metric, not a physical-memory one.** Over the
  six cache-loaded workload runs per corpus it varied by at most 0.13 MiB (a cold-built heap varies more:
  0.2, 16.5, 21.6 and 12.7 MiB). Three limits:
  - For M1 it is circular. The M1 estimate is itself part of the live heap, so an M1 experiment judged on
    live heap would report the C1 estimate back and could not fail. It would not show that the host gets
    the memory back, which is the lane's goal and exactly what the excess above puts in doubt.
  - With a spread of 0.1 MiB the pinned noise rule passes on any change; a minimum effect size has to be
    pinned with it.
  - M2 does not depend on this choice: its base is the load peak.
- **A footprint is repeatable once its level is known.** The levels can be told apart from data every run
  already records (footprint minus live heap is at least 157 MiB at the first level and at most 14 MiB at
  the second), and within the first level the workload footprint varied by 0.3, 3.6, 6.2 and 2.1 MiB across
  runs. Tagging each run's level needs no harness change.
- **Releasing free memory may itself be a candidate.** If the first level persists in a serving daemon, the
  excess is 157–715 MiB after a load (19–23 % of the footprint, which would meet the rule on all four
  corpora) and about 0.95 GiB after a slicing refresh. Whether the process can release it on request was
  not tested, so this is unranked.
- For sizing, the idle cost of one cache-loaded index was close to its live heap where the second level was
  observed (slicing 1.5 GiB, prometheus 2.3 GiB). For excalidraw (0.5 GiB) and ruff (2.3 GiB) that is
  inferred, and no process that stayed at the first level was followed past 60 s. An index built in the
  process holds more (see below), and a refresh needs about twice the footprint.

### What the load transient is

On the two corpora where the second level was observed, the peak exceeds the idle footprint by 425–479 MiB
on slicing (21–23 % of a peak of 2017–2047 MiB; three runs, one of them `idle-probe/a-stderr.txt`) and by
803 MiB on prometheus (26 % of a 3111 MiB peak, two runs). That is larger than the cache file (351 and
693 MiB), so streaming the load (M2) can remove at most the file-sized part of it. The "observed load
overshoot" column (28–57 MiB) is small because in those runs the footprint had not come down when the next
sample was taken; it is a lower bound, as its header says.

### Comparison with the 2026-10-09 figures

The spec (§1) recorded 1,168 MB resident for an idle `prism-mcp` and a 1,620 MB peak for a warm load on the
slicing repository, then 1,232 files, 420k CPG nodes and 672k edges. Today's slicing corpus is 1,474 files,
508k nodes and 822k edges (about 21 % larger), and the cache format has moved to version 114.

| | 2026-10-09 | Scaled by 1.21 | Today |
|---|---|---|---|
| Idle, index built | 1,168 MB resident | 1,413 | 1,529 MiB footprint / 1,595 MiB resident (normal mode, second level, call-edge index not yet built) |
| Peak, warm load | 1,620 MB | 1,960 | 2,016 MiB (index load), 2,047 MiB (with the call-edge index) |

Today's figures are consistent with the earlier ones: the peak is 3 % above the scaled figure and the idle
figure 8–13 % above it, if the spec's "MB" means MiB. If it means decimal megabytes the gaps are about 8 %
and 13–18 %. The comparison is approximate in other ways too: scaling by node count is a rough correction,
the earlier figure is a resident size, and how long that process had been idle was not recorded. Compared
against the 5 s "Steady state" column instead (2,052 MiB) the gap would be 45 %, which is how the two
levels were found.

### Other observations

- **A no-change refresh needs about twice the footprint.** The refresh peak is 1.9–2.0 times the 5 s
  footprint on every corpus (1.4, 5.7, 5.7 and 3.9 GiB). The measured refresh found nothing stale, took
  10–35 s against 44–802 s for a cold build, and left the same live heap as a cache load, so it is
  consistent with loading a second index from the cache while the first is still held. **A refresh after
  files changed, which is what a daemon will mostly do, was not measured** and may peak higher. Slicing
  refresh #3 started at the second level and peaked at 4,008 MiB, the highest of the three, so on one run
  the starting level did not lower the peak.
- **Most of the index is not covered by any candidate.** Of the index live heap, 90–93 % (360–1,862 MiB) is
  "not attributed": it is neither inline node storage nor node strings. Tree-sitter trees are counted
  separately and are 13–27 % of the steady live heap. No current candidate addresses the rest, and finding
  one would need allocation-level attribution that this harness does not have.
- **An index built in the process holds more live heap than the same index loaded from the cache**: 80, 181,
  283 and 216 MiB more (excalidraw, prometheus, ruff, slicing; median cold against median warm at the settle
  stage). Not investigated.
- **A cold build of ruff took 13 minutes** (779–802 s, three runs) and the priming build, which also writes
  the cache, took 23 minutes (1,378 s), on 15 worker threads. Slicing and prometheus took about 2.3 minutes.
  This is a time observation from report mode, which counts every tree-sitter allocation; whether normal
  mode is as slow was not measured.

### Kubernetes was not run

The plan names five corpora. Kubernetes (`8d34ac5ef882`, 30,485 tracked files, roughly 17,600 source files
and 169 MiB of source by file extension) was left out because the four measured corpora project it past
this host's memory. Their first-build peak is 164–426 times the source size, and the Go corpus (prometheus)
is the 426. That projects a first build of 27–70 GiB and a live heap of 14–32 GiB on a host with 24 GiB
of memory and 5 GiB of its 6 GiB swap already in use. A fifth corpus could not change a verdict: M1 and M2
already meet the rule on four corpora where two are needed, and M3 meets it on none, so one more could give
it at most one. Running kubernetes needs a larger host or a subtree of the repository; that choice is the
owner's.

### Limits

- One host, one OS, one allocator. Nothing here was measured on Linux, where freed memory is returned on a
  different schedule and `heap_in_use_bytes` has a different definition (see `scripts/memreport/README.md`).
- Three repetitions per kind; the first-build peak is one run per corpus.
- M1 is a measured upper bound (trees that are re-parsed later would come back). M2 is a file size, an
  upper bound on the buffer that streaming removes. M3 and the "attributed" column are estimates from
  component counts.
- Host swap was 5.0–5.1 GB of 6.1 GB in use throughout. No warm or workload run had compressed pages at
  any stage, so the steady figures were not affected by compression. Two of three ruff cold runs, the
  prometheus and ruff priming runs, and one refresh run each on prometheus and ruff had more than 1 MiB
  compressed at a late stage (31–695 MiB). Footprints and peaks count compressed pages, so they are not understated;
  resident sizes in those runs are.
- The 60 s supplement (`settle60/`) ran only the workload kind, through the driver's own command builder
  and validator but outside `run`: it has no priming run and no `summary.json`, and it recorded swap use
  but not the fuller host state the driver records. Its script is stored with its raw reports.
- The normal-mode observations (`idle-probe/`) are two processes on one corpus, and they, the separate
  report-mode run and five of the twelve supplement runs were not measured one process at a time (see
  "Overlap, disclosed" above). `run-1` itself ran one process at a time.
- The rule in the spec says 200 MB; the tables apply 200 MiB. No row is close enough to the threshold for
  the difference to matter.
- The hand-written section was reviewed by two reviewers who recomputed its figures from the raw reports;
  what moves a process between the two levels is not established by the saved evidence.
