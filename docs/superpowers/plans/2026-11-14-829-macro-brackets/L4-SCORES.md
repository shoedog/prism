# #829 L4: design scoring before implementation (owner-refusal predicates)

Serves: choosing the redesign (TEST-FORM section 4, L4 / option E2) and TEST-FORM exit criterion 5
("the known-wrong candidates are rejected"). Binding decisions (`DECISIONS.md`): the truth gate is
STRICT (no dropped pair may be CONTRADICTED, no per-case waiver); recall is a reported guide, not a
threshold. Nothing in prism changed: the gates are still stubs, so "head" below is the base binary
of 2026-11-14 19:05 at `fae6690c`. Instrument: `scripts/dfg_prune/xscore.py` (usage and limits in
`scripts/dfg_prune/XARM.md`, "L4 scoring"); candidates in `xscore_candidates.py`; the whole scoring
run takes 11 s (it reads saved oracle results; it runs neither prism nor clang).

**Amendment notice (2026-11-15):** sections 1–8 are historical scoring records, not current soundness endorsements. Sol refuted candidate 10; current probes also refute 10n. Section 9 records candidate 12 and its higher recall cost.

## 1. Method

A candidate design is an owner-refusal predicate `owner, db -> None | reason`. For each pair the
head drops, the predicate decides on the pair's owner:
- admit: U and D stay on, the pair is dropped as on the T arm;
- refuse: U and D are off for that owner, the pair is kept as on the Tb baseline arm.

The oracle verdict of the pair is frozen data (the X arm ran the same base binary on the compiler's
token stream), so it does not depend on the candidate. Classes of a head-dropped pair and what each
score means:

| class | source | refused means | admitted means |
|---|---|---|---|
| MUSTKEEP | oracle CONTRADICTED, or a row that asserts `kept` | correct | **violation** (STRICT) |
| MUSTKEEP831 | a generator program whose macro body holds a top-level `;` (#831, outside the gate) | correct | reported apart |
| PINNED | pinned-hole row (A-inputs: #813 holes) | a hole the design closes | the hole stays, as pinned |
| BOUNDARY | guide row whose truth depends on the declared configuration (A-ext, A-config) | closed by the design | open, never a violation |
| CONFIRMED | oracle CONFIRMED (the drop is sound) | **recall cost** | the drop stays |
| UNJUDGED | XOFF / UNJUDGED (no basis) | reported apart | the drop stays |

What a predicate may use: the owner's own tokens and spans, whether the owner lies inside an open
macro argument group at file scope, its macro invocations and their argument groups, and per-macro
facts computed from the `#define` lines of the census inputs (bracket balance with digraphs,
loose comma, top-level and any `;`, control and declaration keywords, free names and their closure,
membership of the 16 listed external names). It never sees the clang expansion.

Populations:
- **L1**: the frozen rows of `tests/fixtures/dfg_prune/oracle/` (pair rows 284, inline rows 105,
  generator rows 98); the fixtures come from the export of the row tests (`.scratch829/exp2`, all
  284 row keys match). Only rows the head drops count (pair rows 113 must-keep of 119 asserted
  `kept`, inline 30 of 33, generator 98 of 98).
- **L2**: the bound-3 judged set of the generator (`.scratch829/gen/k3`, 30,081 programs with a
  verdict other than NODROP: 6,903 MUSTKEEP, 2,403 MUSTKEEP831, 16,330 CONFIRMED, 4,445 UNJUDGED),
  every program re-rendered from its saved spec, lexical variants and channels included. The bound-4
  batches (k4a, k4b, k4c; 8,566 MUSTKEEP, 5,956 MUSTKEEP831, 15,158 CONFIRMED; overlapping k3)
  were scored too: the same pattern, 0 violations for 4b, 6b, 8 and the ablations, 1,937 for 1, 2, 3,
  5 and 7 (all class E).
- **L3**: the saved base sweeps of the six corpora (`.scratch829/l3/<corpus>/base/rows.tsv`; sources
  re-read from the pinned checkouts). In-scope drops placed in a located owner: 37,777 of 37,795
  CONFIRMED, 50 CONTRADICTED, 3,361 UNJUDGED or XOFF; out-of-scope pairs (19,314) are not scored.

## 2. Candidates

| id | definition |
|---|---|
| 0 admit-all | nothing new: every owner admitted (the head; red on base) |
| 1 tentative | DESIGN section 6 as amended: refuse when the owner spells (a) a name of `BLOCK_MACROS` (the 16 names), (b) an in-repo macro whose closed facts are `unbalanced` (any bracket that does not nest, digraphs count, closed over the macros a body names), (c) an in-repo macro whose transitive free names hold a `BLOCK_MACROS` name, (d) an in-repo macro with a closed loose comma (G8), or (e) a macro invocation with a comma whose innermost enclosing bracket is `{` or `[` inside an in-repo macro's argument group, or a mismatched closer or digraph bracket there (G9). **It has no enclosing-group check** (confirmed in `cand_tentative`: `enclosing=False`). This is also candidate 4 of the brief. |
| 2 no-closure | 1 with own facts only: no propagation through referenced macros, no wrapper closure; the list stays |
| 3 spelling-only | 2 without the list: only the macros the owner spells directly, their own facts, G9 |
| 4b tentative+enclosing | 1 plus: the owner lies inside an open macro argument group at file scope (an unclosed `(` before it, or a `)` after it that nothing in the file opens: `BEGIN_FNS static int d; f() {..} )`) |
| 5 any-macro | refuse when the owner spells any in-repo macro name or one of the 16 listed names |
| 6 certificate | inertness certificate sketch: admit only when the owner is not inside an open group, spells no listed name, and every in-repo macro it spells (closed over what it names) is balanced, comma-tight, `;`-free, brace-free, control-free and declaration-keyword-free (`int`, `static`, `const`, ...: "expression-shaped"); reaches no listed name |
| 6b certificate+split | 6 plus the argument-group check G9 |
| 7 depth1 | 1 with brace-held commas counted only when the `{` or `[` is directly at the top level of the group (known wrong, G2-opus-02) |
| 8 fine-G8 (exploratory) | 4b with G8 replaced by "an in-repo macro with a loose comma is spelled inside an argument group of an in-repo macro at the owner, or inside a bracket of the body of a macro the owner reaches" |
| 9 refuse-all | every owner refused (the baseline arm; bounds recall at 100% lost) |
| A1..A4 | ablations of 4b: A1 without G8, A2 without G9, A3 without the list and wrapper closure, A4 only `unbalanced` plus the enclosing check |

## 3. Results

Cells are "violations / head-dropped must-keep pairs" unless stated. Violations are admitted
MUSTKEEP pairs; the first six columns must be 0 for a STRICT-acceptable design.

### Summary (must-keep pairs the design still drops, out of the pairs head drops; CONFIRMED drops lost)

| candidate | L1 pair rows | L1 inline rows | L1 generator rows | L2 bound 3 | L3 corpora (6) | pinned holes open (L1) | boundary rows open (L1) | L3 CONFIRMED lost |
|---|---|---|---|---|---|---|---|---|
| 0-admit-all | 113 / 113 | 30 / 30 | 98 / 98 | 6903 / 6903 | 50 / 50 | 23 / 23 | 41 / 41 | 0.0% (0 / 37777) |
| 1-tentative | 16 / 113 | 4 / 30 | 50 / 98 | 1947 / 6903 | 0 / 50 | 21 / 23 | 18 / 41 | 10.5% (3973 / 37777) |
| 2-no-closure | 24 / 113 | 5 / 30 | 50 / 98 | 1947 / 6903 | 0 / 50 | 22 / 23 | 35 / 41 | 10.3% (3879 / 37777) |
| 3-spelling-only | 24 / 113 | 5 / 30 | 50 / 98 | 1947 / 6903 | 0 / 50 | 22 / 23 | 41 / 41 | 10.3% (3879 / 37777) |
| 4b-tentative+enclosing | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 17 / 41 | 10.5% (3973 / 37777) |
| 5-any-macro | 16 / 113 | 4 / 30 | 50 / 98 | 1947 / 6903 | 0 / 50 | 19 / 23 | 14 / 41 | 89.8% (33907 / 37777) |
| 6-certificate | 0 / 113 | 3 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 13 / 41 | 54.7% (20658 / 37777) |
| 6b-certificate+split | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 13 / 41 | 54.7% (20658 / 37777) |
| 7-depth1 | 17 / 113 | 6 / 30 | 50 / 98 | 1947 / 6903 | 0 / 50 | 21 / 23 | 18 / 41 | 10.5% (3973 / 37777) |
| 8-fine-G8 | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 17 / 41 | 2.4% (912 / 37777) |
| 9-refuse-all | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 0 / 23 | 0 / 41 | 100.0% (37777 / 37777) |
| A1-4b-no-G8 | 13 / 113 | 1 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 17 / 41 | 0.9% (326 / 37777) |
| A2-4b-no-G9 | 1 / 113 | 11 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 21 / 23 | 17 / 41 | 10.5% (3973 / 37777) |
| A3-4b-no-list | 2 / 113 | 0 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 22 / 23 | 40 / 41 | 10.5% (3973 / 37777) |
| A4-4b-unbalanced-only | 16 / 113 | 12 / 30 | 0 / 98 | 0 / 6903 | 0 / 50 | 22 / 23 | 40 / 41 | 0.9% (326 / 37777) |


Column notes: L1 pinned holes (23 head-dropped rows) are 21 A-inputs rows (`#813`), one A-ext
witness (`g_terra_01` wrapper in a header) and one A-dialect witness (digit separator). No predicate
can close an A-inputs row, because the macro definition is in a file the census (and so the scorer)
does not read. The two rows a design can close are the A-ext witness (`g_terra_01` wrapper in a
header, closed by the list) and the A-dialect witness (inline `33 g2o03`: `OPEN`/`CLOSE` are
unbalanced under the C++ reading of `1'0`; every candidate that reads the macro bodies closes it,
because this lexer reads digit separators as C++ does and the real census ORs both dialects). The 41 boundary rows are A-ext guide rows
(listed and unlisted external names, external aliases of in-repo macros): the list closes 24 of
them (4b), nothing closes the rest, which stay the D2 disclosure. L2's 2,403 MUSTKEEP831 programs
(all with a top-level `;` in a macro body) are refused by every candidate that refuses
`unbalanced`, because every one of them also has an unbalanced body: the generator does not
isolate #831 (`int x = M, z = 3;`), so the #831 shape is not scored. **By construction (not
measured), 1, 4b, 7 and 8 have no rule for a macro whose only hazard is a top-level `;`; 6 and 6b
refuse it (`cert_semicolon`).**

### Per population

#### L1-pair

| candidate | must-keep dropped / total | #831 must-keep dropped / total | pinned open / total | boundary open / total | CONFIRMED lost / total | UNJUDGED lost / total |
|---|---|---|---|---|---|---|
| 0-admit-all | 113 / 113 | 0 / 0 | 22 / 22 | 10 / 10 | 0 / 30 (0.0%) | 0 / 101 |
| 1-tentative | 16 / 113 | 0 / 0 | 21 / 22 | 3 / 10 | 15 / 30 (50.0%) | 94 / 101 |
| 2-no-closure | 24 / 113 | 0 / 0 | 22 / 22 | 10 / 10 | 7 / 30 (23.3%) | 38 / 101 |
| 3-spelling-only | 24 / 113 | 0 / 0 | 22 / 22 | 10 / 10 | 3 / 30 (10.0%) | 10 / 101 |
| 4b-tentative+enclosing | 0 / 113 | 0 / 0 | 21 / 22 | 2 / 10 | 16 / 30 (53.3%) | 94 / 101 |
| 5-any-macro | 16 / 113 | 0 / 0 | 19 / 22 | 1 / 10 | 26 / 30 (86.7%) | 99 / 101 |
| 6-certificate | 0 / 113 | 0 / 0 | 21 / 22 | 0 / 10 | 26 / 30 (86.7%) | 99 / 101 |
| 6b-certificate+split | 0 / 113 | 0 / 0 | 21 / 22 | 0 / 10 | 26 / 30 (86.7%) | 99 / 101 |
| 7-depth1 | 17 / 113 | 0 / 0 | 21 / 22 | 3 / 10 | 15 / 30 (50.0%) | 94 / 101 |
| 8-fine-G8 | 0 / 113 | 0 / 0 | 21 / 22 | 2 / 10 | 16 / 30 (53.3%) | 94 / 101 |
| 9-refuse-all | 0 / 113 | 0 / 0 | 0 / 22 | 0 / 10 | 30 / 30 (100.0%) | 101 / 101 |
| A1-4b-no-G8 | 13 / 113 | 0 / 0 | 21 / 22 | 2 / 10 | 16 / 30 (53.3%) | 94 / 101 |
| A2-4b-no-G9 | 1 / 113 | 0 / 0 | 21 / 22 | 2 / 10 | 16 / 30 (53.3%) | 94 / 101 |
| A3-4b-no-list | 2 / 113 | 0 / 0 | 22 / 22 | 9 / 10 | 4 / 30 (13.3%) | 10 / 101 |
| A4-4b-unbalanced-only | 16 / 113 | 0 / 0 | 22 / 22 | 9 / 10 | 4 / 30 (13.3%) | 10 / 101 |

#### L1-inline

| candidate | must-keep dropped / total | #831 must-keep dropped / total | pinned open / total | boundary open / total | CONFIRMED lost / total | UNJUDGED lost / total |
|---|---|---|---|---|---|---|
| 0-admit-all | 30 / 30 | 0 / 0 | 1 / 1 | 31 / 31 | 0 / 33 (0.0%) | 0 / 7 |
| 1-tentative | 4 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 16 / 33 (48.5%) | 5 / 7 |
| 2-no-closure | 5 / 30 | 0 / 0 | 0 / 1 | 25 / 31 | 15 / 33 (45.5%) | 5 / 7 |
| 3-spelling-only | 5 / 30 | 0 / 0 | 0 / 1 | 31 / 31 | 13 / 33 (39.4%) | 4 / 7 |
| 4b-tentative+enclosing | 0 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 18 / 33 (54.5%) | 5 / 7 |
| 5-any-macro | 4 / 30 | 0 / 0 | 0 / 1 | 13 / 31 | 25 / 33 (75.8%) | 6 / 7 |
| 6-certificate | 3 / 30 | 0 / 0 | 0 / 1 | 13 / 31 | 11 / 33 (33.3%) | 5 / 7 |
| 6b-certificate+split | 0 / 30 | 0 / 0 | 0 / 1 | 13 / 31 | 19 / 33 (57.6%) | 6 / 7 |
| 7-depth1 | 6 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 15 / 33 (45.5%) | 5 / 7 |
| 8-fine-G8 | 0 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 15 / 33 (45.5%) | 5 / 7 |
| 9-refuse-all | 0 / 30 | 0 / 0 | 0 / 1 | 0 / 31 | 33 / 33 (100.0%) | 7 / 7 |
| A1-4b-no-G8 | 1 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 15 / 33 (45.5%) | 5 / 7 |
| A2-4b-no-G9 | 11 / 30 | 0 / 0 | 0 / 1 | 15 / 31 | 10 / 33 (30.3%) | 4 / 7 |
| A3-4b-no-list | 0 / 30 | 0 / 0 | 0 / 1 | 31 / 31 | 16 / 33 (48.5%) | 4 / 7 |
| A4-4b-unbalanced-only | 12 / 30 | 0 / 0 | 0 / 1 | 31 / 31 | 5 / 33 (15.2%) | 3 / 7 |

#### L1-gen

| candidate | must-keep dropped / total | #831 must-keep dropped / total | pinned open / total | boundary open / total | CONFIRMED lost / total | UNJUDGED lost / total |
|---|---|---|---|---|---|---|
| 0-admit-all | 98 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 1-tentative | 50 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 2-no-closure | 50 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 3-spelling-only | 50 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 4b-tentative+enclosing | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 5-any-macro | 50 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 6-certificate | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 6b-certificate+split | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 7-depth1 | 50 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 8-fine-G8 | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| 9-refuse-all | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| A1-4b-no-G8 | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| A2-4b-no-G9 | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| A3-4b-no-list | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |
| A4-4b-unbalanced-only | 0 / 98 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 (-) | 0 / 0 |

#### L2

| candidate | must-keep dropped / total | #831 must-keep dropped / total | pinned open / total | boundary open / total | CONFIRMED lost / total | UNJUDGED lost / total |
|---|---|---|---|---|---|---|
| 0-admit-all | 6903 / 6903 | 2403 / 2403 | 0 / 0 | 0 / 0 | 0 / 16330 (0.0%) | 0 / 4445 |
| 1-tentative | 1947 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4352 / 16330 (26.7%) | 3934 / 4445 |
| 2-no-closure | 1947 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4150 / 16330 (25.4%) | 3624 / 4445 |
| 3-spelling-only | 1947 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4150 / 16330 (25.4%) | 3624 / 4445 |
| 4b-tentative+enclosing | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4376 / 16330 (26.8%) | 4444 / 4445 |
| 5-any-macro | 1947 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 16306 / 16330 (99.9%) | 3935 / 4445 |
| 6-certificate | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 16158 / 16330 (98.9%) | 4445 / 4445 |
| 6b-certificate+split | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 16158 / 16330 (98.9%) | 4445 / 4445 |
| 7-depth1 | 1947 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4352 / 16330 (26.7%) | 3934 / 4445 |
| 8-fine-G8 | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 2767 / 16330 (16.9%) | 4444 / 4445 |
| 9-refuse-all | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 16330 / 16330 (100.0%) | 4445 / 4445 |
| A1-4b-no-G8 | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 2755 / 16330 (16.9%) | 4444 / 4445 |
| A2-4b-no-G9 | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4376 / 16330 (26.8%) | 4444 / 4445 |
| A3-4b-no-list | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 4376 / 16330 (26.8%) | 4444 / 4445 |
| A4-4b-unbalanced-only | 0 / 6903 | 0 / 2403 | 0 / 0 | 0 / 0 | 2755 / 16330 (16.9%) | 4444 / 4445 |

(L2's CONFIRMED column is not a recall estimate: the generator's macro bodies are adversarial by
construction.)

### Recall cost (L3, the corpora)

### L3 recall cost per corpus: CONFIRMED drops lost / total (UNJUDGED lost / total)

| candidate | snappy | leveldb | re2 | redis | cpython | protobuf |
|---|---|---|---|---|---|---|
| 0-admit-all | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 0 / 21605 (0.0%) (0 / 1820) | 0 / 15220 (0.0%) (0 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 1-tentative | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2367 / 15220 (15.6%) (27 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 2-no-closure | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2273 / 15220 (14.9%) (15 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 3-spelling-only | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2273 / 15220 (14.9%) (15 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 4b-tentative+enclosing | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2367 / 15220 (15.6%) (27 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 5-any-macro | 6 / 19 (31.6%) (0 / 0) | 311 / 473 (65.8%) (0 / 6) | 6 / 38 (15.8%) (20 / 142) | 20108 / 21605 (93.1%) (1608 / 1820) | 13464 / 15220 (88.5%) (938 / 1190) | 12 / 422 (2.8%) (25 / 203) |
| 6-certificate | 4 / 19 (21.1%) (0 / 0) | 311 / 473 (65.8%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 10692 / 21605 (49.5%) (1032 / 1820) | 9639 / 15220 (63.3%) (658 / 1190) | 12 / 422 (2.8%) (3 / 203) |
| 6b-certificate+split | 4 / 19 (21.1%) (0 / 0) | 311 / 473 (65.8%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 10692 / 21605 (49.5%) (1032 / 1820) | 9639 / 15220 (63.3%) (658 / 1190) | 12 / 422 (2.8%) (3 / 203) |
| 7-depth1 | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2367 / 15220 (15.6%) (27 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 8-fine-G8 | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 0 / 21605 (0.0%) (0 / 1820) | 912 / 15220 (6.0%) (23 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| 9-refuse-all | 19 / 19 (100.0%) (0 / 0) | 473 / 473 (100.0%) (6 / 6) | 38 / 38 (100.0%) (142 / 142) | 21605 / 21605 (100.0%) (1820 / 1820) | 15220 / 15220 (100.0%) (1190 / 1190) | 422 / 422 (100.0%) (203 / 203) |
| A1-4b-no-G8 | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 0 / 21605 (0.0%) (0 / 1820) | 326 / 15220 (2.1%) (23 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| A2-4b-no-G9 | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2367 / 15220 (15.6%) (27 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| A3-4b-no-list | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 1606 / 21605 (7.4%) (14 / 1820) | 2367 / 15220 (15.6%) (27 / 1190) | 0 / 422 (0.0%) (0 / 203) |
| A4-4b-unbalanced-only | 0 / 19 (0.0%) (0 / 0) | 0 / 473 (0.0%) (0 / 6) | 0 / 38 (0.0%) (0 / 142) | 0 / 21605 (0.0%) (0 / 1820) | 326 / 15220 (2.1%) (23 / 1190) | 0 / 422 (0.0%) (0 / 203) |


Owners (not pairs): the share of the head's `on` owners of the scoped files that a candidate
refuses (matched by prism's owner start line to the scorer's located function; the number in
brackets counts the owners that have at least one dropped pair):

### L3 owners refused / owners `on` on the head (pairs aside; owners with a dropped pair in brackets)

| candidate | snappy | leveldb | re2 | redis | cpython | protobuf |
|---|---|---|---|---|---|---|
| 0-admit-all | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 0 / 5062 (0.0%) [0 / 1688] | 0 / 5425 (0.0%) [0 / 1414] | 0 / 7295 (0.0%) [0 / 92] |
| 1-tentative | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 271 / 5425 (5.0%) [165 / 1414] | 8 / 7295 (0.1%) [0 / 92] |
| 2-no-closure | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 224 / 5425 (4.1%) [144 / 1414] | 8 / 7295 (0.1%) [0 / 92] |
| 3-spelling-only | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 224 / 5425 (4.1%) [144 / 1414] | 8 / 7295 (0.1%) [0 / 92] |
| 4b-tentative+enclosing | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 271 / 5425 (5.0%) [165 / 1414] | 8 / 7295 (0.1%) [0 / 92] |
| 5-any-macro | 53 / 222 (23.9%) [2 / 4] | 119 / 1384 (8.6%) [1 / 28] | 71 / 886 (8.0%) [5 / 21] | 3778 / 5062 (74.6%) [1448 / 1688] | 3896 / 5425 (71.8%) [1161 / 1414] | 1335 / 7295 (18.3%) [11 / 92] |
| 6-certificate | 39 / 222 (17.6%) [1 / 4] | 6 / 1384 (0.4%) [1 / 28] | 0 / 886 (0.0%) [0 / 21] | 1518 / 5062 (30.0%) [655 / 1688] | 1833 / 5425 (33.8%) [712 / 1414] | 324 / 7295 (4.4%) [8 / 92] |
| 6b-certificate+split | 39 / 222 (17.6%) [1 / 4] | 6 / 1384 (0.4%) [1 / 28] | 0 / 886 (0.0%) [0 / 21] | 1518 / 5062 (30.0%) [655 / 1688] | 1833 / 5425 (33.8%) [712 / 1414] | 328 / 7295 (4.5%) [8 / 92] |
| 7-depth1 | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 271 / 5425 (5.0%) [165 / 1414] | 4 / 7295 (0.1%) [0 / 92] |
| 8-fine-G8 | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 4 / 5062 (0.1%) [0 / 1688] | 122 / 5425 (2.2%) [87 / 1414] | 4 / 7295 (0.1%) [0 / 92] |
| 9-refuse-all | 222 / 222 (100.0%) [4 / 4] | 1384 / 1384 (100.0%) [28 / 28] | 886 / 886 (100.0%) [21 / 21] | 5062 / 5062 (100.0%) [1688 / 1688] | 5425 / 5425 (100.0%) [1414 / 1414] | 7295 / 7295 (100.0%) [92 / 92] |
| A1-4b-no-G8 | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 1 / 5062 (0.0%) [0 / 1688] | 98 / 5425 (1.8%) [71 / 1414] | 4 / 7295 (0.1%) [0 / 92] |
| A2-4b-no-G9 | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 271 / 5425 (5.0%) [165 / 1414] | 4 / 7295 (0.1%) [0 / 92] |
| A3-4b-no-list | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 195 / 5062 (3.9%) [104 / 1688] | 271 / 5425 (5.0%) [165 / 1414] | 8 / 7295 (0.1%) [0 / 92] |
| A4-4b-unbalanced-only | 0 / 222 (0.0%) [0 / 4] | 0 / 1384 (0.0%) [0 / 28] | 0 / 886 (0.0%) [0 / 21] | 1 / 5062 (0.0%) [0 / 1688] | 98 / 5425 (1.8%) [71 / 1414] | 0 / 7295 (0.0%) [0 / 92] |


Gate reasons of the lost CONFIRMED drops, all corpora and per corpus:

### Gate reasons of the refused CONFIRMED drops (L3, six corpora, pairs)

| candidate | reason: pairs |
|---|---|
| 0-admit-all | - |
| 1-tentative | loose_comma 3647, block_list 275, unbalanced 51 |
| 2-no-closure | loose_comma 3604, block_list 275 |
| 3-spelling-only | loose_comma 3604, unbalanced 275 |
| 4b-tentative+enclosing | loose_comma 3647, block_list 275, unbalanced 51 |
| 5-any-macro | any_macro 33907 |
| 6-certificate | cert_semicolon 11697, cert_declaration 7673, cert_loose_comma 695, block_list 275, cert_control 226, block_wrapper 51, cert_brace 41 |
| 6b-certificate+split | cert_semicolon 11697, cert_declaration 7673, cert_loose_comma 695, block_list 275, cert_control 226, block_wrapper 51, cert_brace 41 |
| 7-depth1 | loose_comma 3647, block_list 275, unbalanced 51 |
| 8-fine-G8 | loose_in_group 586, block_list 275, unbalanced 51 |
| 9-refuse-all | refuse_all 37777 |
| A1-4b-no-G8 | block_list 275, unbalanced 51 |
| A2-4b-no-G9 | loose_comma 3647, block_list 275, unbalanced 51 |
| A3-4b-no-list | loose_comma 3647, unbalanced 326 |
| A4-4b-unbalanced-only | unbalanced 326 |


### Gate reasons of the refused CONFIRMED drops per corpus (pairs)

| candidate | corpus | reason: pairs |
|---|---|---|
| 1-tentative | redis | loose_comma 1606 |
| 1-tentative | cpython | loose_comma 2041, block_list 275, unbalanced 51 |
| 2-no-closure | redis | loose_comma 1606 |
| 2-no-closure | cpython | loose_comma 1998, block_list 275 |
| 3-spelling-only | redis | loose_comma 1606 |
| 3-spelling-only | cpython | loose_comma 1998, unbalanced 275 |
| 4b-tentative+enclosing | redis | loose_comma 1606 |
| 4b-tentative+enclosing | cpython | loose_comma 2041, block_list 275, unbalanced 51 |
| 5-any-macro | snappy | any_macro 6 |
| 5-any-macro | leveldb | any_macro 311 |
| 5-any-macro | re2 | any_macro 6 |
| 5-any-macro | redis | any_macro 20108 |
| 5-any-macro | cpython | any_macro 13464 |
| 5-any-macro | protobuf | any_macro 12 |
| 6-certificate | snappy | cert_declaration 4 |
| 6-certificate | leveldb | cert_semicolon 311 |
| 6-certificate | redis | cert_declaration 6091, cert_semicolon 4078, cert_loose_comma 523 |
| 6-certificate | cpython | cert_semicolon 7296, cert_declaration 1578, block_list 275, cert_control 226, cert_loose_comma 172, block_wrapper 51, cert_brace 41 |
| 6-certificate | protobuf | cert_semicolon 12 |
| 6b-certificate+split | snappy | cert_declaration 4 |
| 6b-certificate+split | leveldb | cert_semicolon 311 |
| 6b-certificate+split | redis | cert_declaration 6091, cert_semicolon 4078, cert_loose_comma 523 |
| 6b-certificate+split | cpython | cert_semicolon 7296, cert_declaration 1578, block_list 275, cert_control 226, cert_loose_comma 172, block_wrapper 51, cert_brace 41 |
| 6b-certificate+split | protobuf | cert_semicolon 12 |
| 7-depth1 | redis | loose_comma 1606 |
| 7-depth1 | cpython | loose_comma 2041, block_list 275, unbalanced 51 |
| 8-fine-G8 | cpython | loose_in_group 586, block_list 275, unbalanced 51 |
| 9-refuse-all | snappy | refuse_all 19 |
| 9-refuse-all | leveldb | refuse_all 473 |
| 9-refuse-all | re2 | refuse_all 38 |
| 9-refuse-all | redis | refuse_all 21605 |
| 9-refuse-all | cpython | refuse_all 15220 |
| 9-refuse-all | protobuf | refuse_all 422 |
| A1-4b-no-G8 | cpython | block_list 275, unbalanced 51 |
| A2-4b-no-G9 | redis | loose_comma 1606 |
| A2-4b-no-G9 | cpython | loose_comma 2041, block_list 275, unbalanced 51 |
| A3-4b-no-list | redis | loose_comma 1606 |
| A3-4b-no-list | cpython | loose_comma 2041, unbalanced 326 |
| A4-4b-unbalanced-only | cpython | unbalanced 326 |

## 4. Must-keep violations by class

Every violation of every candidate belongs to one of four classes (rows named as in the frozen
data; the L2 generator keys are `effects|family|owner`):

| class | rows | missed by | why |
|---|---|---|---|
| **E. owner inside an open macro argument group** | L1 pair: opus3-01 (lambda local class method), 01b, 02 (c, cpp, stringizing x2), 03 (sibling x2), 04 (decl initializer x2), 05 (closer x2), 06 (class member), 07 (template), 08 (template argument commas), 09 (namespace); L1 inline: 29 `g2o01` (c, cc), 31 `g2o01b` (c, cc: group opened by a file-scope unbalanced macro, closed by a literal `)`); L1 generator: 50 rows; L2: 1,947 programs, 48 keys (`ARGSPLIT,PARAMS`, `PARAMS`, `BR_BAL,PARAMS`, `BR_BAL,CTRL,PARAMS` x enc1..enc4 x c, cxx, tpl) | 1, 2, 3, 5, 7 | the owner spells no macro: the macro is the group opener outside it, and its argument list contains the owner; the owner's own text is clean. Only a check on the owner's position can see it |
| **W. wrapper and chain, without closure** | L1 pair: `g2_sonnet_04` NumPy allow threads (parsed header, census text, c, cc; `NPY_BEGIN_ALLOW_THREADS` is a macro for `Py_BEGIN_ALLOW_THREADS`), `g_terra_01` census-text wrapper (c, cc), `row_g2_sonnet3_07` alias chain to an unbalanced macro (x2); L1 inline: 20 `b2` `brace_transitive_if_guard` | 2, 3 | the spelled macro is balanced; the unbalanced body is one or more hops away |
| **D. brace-held comma below a parenthesis** | L1 pair: opus3-01c (GNU nested function: `J({ int probe(..) {..} ... })`); L1 inline: 32 `g2o02` (c, cc: `SF(({ int a, b; }))`, the rescan strips the parenthesis) | 7 | the comma's innermost bracket is `{` at depth 2, so a depth-1 rule sees nothing |
| **S. argument split without G9** | L1 inline: `21 y1`, `23 a1t`, `33 g2o04` (digit separator hides a held comma), and with 4b minus G9 eight more (`a1`, `a10`..`a12`, `25 g2o06c`, `32 g2o02`) | 6 (as specified), A2 | a balanced, comma-tight, expression-shaped macro still reorders or drops a brace-held argument piece |

### Candidate 1 (tentative): any must-keep violation?

Yes, class E only, and nothing else in the scored data. 16 L1 pair rows, 4 L1 inline rows, 50 of
98 generator rows, 1,947 of 6,903 L2 programs (bound 4: 1,937 of 8,566); no L3 violation (all 50
CONTRADICTED cpython pairs, `Py_BEGIN_CRITICAL_SECTION`/`Py_END_...` around a plain block, sit
in owners that spell a listed name, and in-repo definitions of them that are unbalanced: 1 refuses
them by the list, 3 by `unbalanced`). The class is the whole of what
the generator's enclosing-group families (enc1 to enc4) and the round-3 enclosing hand rows model.
Adding the enclosing-group check (4b) removes every one of those: 0 violations in all
populations. What the scored data cannot say: 4b's soundness for shapes the generator and the hand
rows do not contain (external macros outside the list, A-inputs, the top-level `;` class #831,
macros defined under conditional compilation that the host build does not select).

### Exit criterion 5 (known-wrong candidates rejected)

**Met.** Every known-wrong candidate has at least one violated must-keep row, mechanism given:

| known-wrong candidate | violated must-keep rows (examples) | mechanism |
|---|---|---|
| 0 admit-all | all 113 + 30 + 98 + 6,903 + 50 | the head drops them (red on base) |
| 1 tentative (no enclosing check; brief candidate 4) | `ARGSPLIT,PARAMS\|enc1\|c` (generator row), opus3-03 opener inside a sibling owner, inline `29 g2o01`, 1,947 L2 programs | class E |
| 2 no closure | NumPy allow threads (a.c), sonnet3-07 alias chain to an unbalanced macro, `g_terra_01` census-text wrapper, inline `20 b2`; plus class E | class W |
| 3 spelling-only | the same L1 rows as 2 (the list adds nothing to them: the owner spells the in-repo wrapper, not a listed name) | class W |
| 5 any-macro | `ARGSPLIT,PARAMS\|enc1\|c`, opus3-03, inline `31 g2o01b` (50 + 16 + 4 + 1,947) | class E: "any macro token refuses" needs a macro token in the owner, the enclosing group's opener is outside it |
| 7 depth1 | opus3-01c `gnest`, inline `32 g2o02` (c, cc); plus class E | class D (G2-opus-02) |
| 6 certificate (as specified) | inline `21 y1`, `23 a1t`, `33 g2o04` | class S: no argument check |

The candidates not wrong on the scored data are 4b, 6b and 8 (0 violations everywhere). That is
a statement about this data, not a proof (section 6).

## 5. Findings for the design

1. **The enclosing-group check is the one missing premise of the tentative rule on the scored
   data, and it is nearly free**: 0 owners refused on snappy, leveldb, re2, redis, cpython and
   protobuf (4b and 1 refuse the same owners and pairs on every corpus; the `)`-after-the-owner
   form that class 31 needs, and the `#if` heuristic for the paren depth, refused none of the
   20,274 `on` owners of the scoped files). The check reads only file-scope parenthesis balance.
2. **G8 (loose comma) is where the recall goes, and it buys little in the scored rows.** 4b
   loses 10.5% of the sound drops in the six corpora (3,973 pairs); 3,647 of them (92%) are G8
   refusals (redis 1,606 pairs = 7.4%, 194 owners; cpython 2,041 pairs). Without G8 (A1) the loss
   falls to 0.9% (326 pairs: `block_list` 275, `unbalanced` 51, all in cpython) and the L1 must-keep
   misses are 13 pair rows (the include-census rows `g_sol_14`, `g_terra_14`, `terra3-02`, the
   rescan-split row `g_sol_03`) and 1 inline row (`23 a9` rescan split): every one is a macro
   whose own body holds a brace-held or top-level comma and that is spelled inside another macro's
   argument group. Candidate 8, which refuses a loose-comma macro only in that position (at the
   owner's own groups, or inside a bracket of a body the owner reaches), has no violation on
   any population and loses 2.4% (912 pairs, cpython 6.0%, redis 0%). Not a proof: the rule was
   tuned on these rows and is exploratory.
3. **G9 and the list are needed by the rows.** Without G9 (A2) 11 inline and 1 pair rows are
   violated (argument splits and `32 g2o02`); without the list and wrapper closure (A3) the two
   `g_terra_01` census-text wrapper rows are violated and 23 more of the 41 boundary rows reopen
   (40 open, against 17 for 4b: A-ext listed names, including `Py_BEGIN_ALLOW_THREADS` through a
   wrapper); `unbalanced` alone (A4)
   leaves 16 pair and 12 inline rows open.
4. **A closed-world certificate costs a lot of the recall.** 6b loses 54.7% of the sound drops
   (redis 49.5%, cpython 63.3%, leveldb 65.8%): `cert_semicolon` 11,697 pairs (statement
   macros), `cert_declaration` 7,673 (type macros such as `uint8`, `static` wrappers),
   `cert_loose_comma` 695. 6b has no violation and also refuses any macro with a top-level `;`
   (the #831 shape), which 1, 4b and 8 do not. The certificate's cost is in how narrowly
   "expression-shaped" is read here; a looser shape (allow type and declaration macros, which the
   scored rows do not need refused) is the measurable middle between 8 and 6b.
5. **"Any macro" is not a safe bound either** (class E) and costs 89.8% of the sound drops
   (redis 93.1%, cpython 88.5%, leveldb 65.8%, protobuf 2.8%).
6. **Stale-data note.** 4b and 8 have no L3 violation, but L3 holds only 50 CONTRADICTED pairs
   (cpython, one mechanism, `Py_BEGIN_CRITICAL_SECTION` in the GIL build); the corpora cannot
   discriminate between designs on soundness, the L1 and L2 rows do.

## 6. What the harness cannot model (where its census emulation differs from prism's)

- **Census input set.** L1 and L2 use the `xarm_core.census_inputs` approximation (C-like
  extension plus the literal include closure by path suffix; `eval/secbench` excluded); L3 uses
  every C-like file of the checkout (`xarm_l3.collect`, what the sweep copied), so a header with a
  non-C-like extension reached only through an include is not read there, and the loader's
  symlink, unreadable-file and `incomplete` handling is not emulated (a repository whose census is
  `incomplete` has U and D off for everything on the real side; the saved sweeps ran with a
  complete census).
- **Definition reading.** A line regex on spliced text finds `#define` lines and one lexer reads
  them with comments and string literals. There is no C versus C++ dialect pass, no comment-kept
  versus comment-stripped split (a define inside a block comment, an `#if 0` or a raw string
  counts, as in prism's comment-kept pass, but a define spanning a block comment across lines is
  cut at the line end), no soft-poison recovery and no raw-byte fallback for lines a lexer
  cannot read (the permissive scan of non-parsed texts). `free` excludes macro parameters for
  files with a parsed extension and includes them otherwise (the permissive over-approximation).
  `#undef` clears nothing and definitions are unioned by name, as in the design.
- **Owners.** There is no parser: functions are located by a token-level rule (a `{` after a `)`
  whose `(` follows a non-control identifier, outside another function body or directly in a
  class-like block, with both branches of `#if` read from the stack at the `#if`). A located
  function is matched to prism's owner by start line: 98 of 5,523 cpython and 207 of 7,502
  protobuf `on` owners are not matched (override, `= delete`, macro-statement pseudo owners of
  tree-sitter), none on redis, snappy, leveldb, re2; 18 CONFIRMED cpython pairs could not be
  placed in any function and are left out of the totals (15,220 of 15,238). A pair belongs to the
  innermost function holding the lower of its two lines. `spelled` is the owner's identifier
  tokens (prism's `spelled_identifiers`); comments and strings are not tokens.
- **Open group.** File-scope parenthesis balance with a first-branch-wins heuristic for
  `#if`/`#else`; an unmatched `)` after the owner (opener hidden in a macro) is read as a group
  too. The scorer cannot tell a real group from a stray parenthesis in an inactive branch.
- **Oracle and the arms.** Verdicts are the frozen X-arm verdicts of the base binary (one host
  configuration, `UNJUDGED` for inactive `#if` branches, stubbed empty outside headers); a refused
  owner is assumed to restore exactly the Tb baseline pairs of that owner and nothing else. Real
  refusal changes the cache (a CPG version bump), which no population exercises. The wrapper and
  closure semantics of external macros (A-ext) are only the 16 names.
- **Populations.** The generator's owner shape is `probe` with two sites; L2 recall is not
  representative. The #831 shape has no isolated row. Rows are scored per pair, so an owner with
  many pairs weighs more (leveldb: one owner holds 311 of the 473 sound pairs).
- **Sound-looking but unproven.** A candidate with 0 violations here is not known sound: the
  data holds known classes only.

## 7. Reproduction

```bash
python3 scripts/dfg_prune/xscore.py selftest
python3 scripts/dfg_prune/xscore.py run --out .scratch829/l4                  # 11 s, all populations
python3 scripts/dfg_prune/xscore.py run --pops l2 --l2dirs .scratch829/gen/k4a .scratch829/gen/k4b .scratch829/gen/k4c --out .scratch829/l4k4
python3 scripts/dfg_prune/xscore.py report .scratch829/l4 --md                # the tables above
python3 scripts/dfg_prune/xscore.py report .scratch829/l4 --violations --pops L1-pair,L1-inline
```

Inputs (git-excluded, local): `.scratch829/exp2` (row export), `.scratch829/gen/k3` and `k4*`
(`results.jsonl`), `.scratch829/l3/<corpus>/base` (`rows.tsv`, `T.json.gz`), and the six corpus
checkouts of `l3_corpora.json`. A new candidate is a function in `xscore_candidates.py` (or in a file
passed with `--candidates`).

## 8. Addendum (redesign phase): candidates 10 and B1 to B8

`xscore_candidates.py` now also holds the rule of `DESIGN-v2.md` (candidate 10), its variants (10x the 31-name list, 10n without shielded parentheses, 10i, 10f, 10s) and its ablations B1 to B8; `xscore_lex.py` records the paren groups of each macro body (`Facts.groups`), tells a plain `#else` from `#elif` and gives a candidate the owner's file index. The 15 candidates above score exactly as before (150 of 150 cells). Numbers, the premise-coverage reading of the ablations and the finding that **candidate 8 is unsound** (it admits a macro whose own body splits an argument, `#define HI F({ int a, b; })`: oracle CONTRADICTED, outside the generator's grammar) are in `DESIGN-v2.md` sections 2 and 9. Candidate 10 (with the hidden-opener half of G10 that owner decision OD-C added; ablation B8 is the rule without it): 0 must-keep violations on L1 (113, 30, 98), L2 bound 3 (6,903) and the bound-4 batches (8,566), L3 (50); recall cost 1.1% (399 of 37,777); 214 of 20,274 owners refused (B8: 0.9%, 332 pairs, 108 owners). Reproduce: `python3 scripts/dfg_prune/xscore.py run --out .scratch829/redesign/l4v2`.

## 9. Addendum: rejected-review repair, candidate 12 (2026-11-15)

**Recommendation: 12-all-roots**, pending OWNER-DECISION OD-A2. No shields; every identifier-following group and its chains are potential callees in both bodies and owners; keep the 31-name list and G10. This closes the unstable-predecessor class without assuming census absence means a stable name. The body scan needs no root-name lookup. `11-no-shield-list31` is the 10n+31 control; B9/B10 separately restore old owner/body roots.

The target 2.4% is **not met**: 12 costs 5.95%. Plain 10n at 2.59% is refuted by external `EXT(v) v` plus empty `E`, both satisfying the existing A-ext model, in owner and body contexts. The class must be refused under STRICT. A narrower parameter-flow/comma certificate is a separate proof project; no new A-ext restriction or per-case waiver is proposed. These corpora are a small convenience sample, not prevalence evidence.

### Executed commands and provenance

Checkout `b608adc590efeb376b7ce080a3d3782355cf44cd`; binary `slicing 4.0.0 (0273cf3324cb)`; Apple clang 21.0.0. `.scratch829/astra/provenance.json` and `MANIFEST.sha256` bind tools, sources and receipts. No Rust or fixture data changed; scorer results predict owner refusal. Corpus oracle verdicts are saved data, not regenerated clang/prism runs.

```bash
python3 scripts/dfg_prune/xscore.py selftest
python3 scripts/dfg_prune/xscore.py run --out .scratch829/astra/l4
python3 scripts/dfg_prune/xscore.py run --pops l2 --l2dirs .scratch829/gen/k4a .scratch829/gen/k4b .scratch829/gen/k4c --only 11-no-shield-list31,12-all-roots --out .scratch829/astra/l4k4
python3 .scratch829/astra/check_scores.py
```

Outputs: `selftest.log`, `l4.log`, `l4/scores.json`, `l4k4.log`, `l4k4/scores.json`, `check_scores.log`, `l4-probes.json`. Selftest passes; **12 focused candidate checks pass**; **290 historical score cells unchanged**. Default L2 rerenders 30,081 judged programs; bound-4 rerenders 33,538 (overlap is not deduplicated into a combined population).

Final consistency audit: one disclosed verification extension repaired two existing emulation omissions in `xscore_lex.py`: the group following `__VA_OPT__(...)` inherits a potential root, and named variadics force the body split fact. The same written design already required both. `vaopt_chain` is an additional actual CONTRADICTED pair (old scorer admits, corrected scorer refuses); the named-variadic full-program probe was already G1Error, so only its fact test adds coverage. All frozen default/bound-4 scores were rerun after the corrections; all 290 historical cells remain unchanged.

### Truth gates

| population | candidate 12 violations / must-keeps |
|---|---:|
| L1 pair | 0 / 113 |
| L1 inline | 0 / 30 |
| L1 generator | 0 / 98 |
| L2 bound 3 | 0 / 6,903 |
| L2 bound-4 batches | 0 / 8,566 |
| L3 six corpora | 0 / 50 |
| new review probes | **0 / 17** |

MUSTKEEP831 is separately 0 / 2,403 at bound 3 and 0 / 5,956 at bound 4. No scope expansion or waiver is inferred. Pinned holes remain 21 / 23 open; historical boundary guides 9 / 41 open (12 closes 32). These disclosed populations are not silently counted as passed must-keeps.

The new 17 pairs all have actual four-arm T/Tb/X/Xb = 0/1/1/1, owners on, accepted clang programs and runtime `probe(0)=41 probe(1)=2`. Candidate 10+31 admits 11; 10n+31 admits 8; B9 admits 5; B10 admits 3. Candidate 12 refuses all. Three additional insertion/named-variadic programs already have T G1Error and contribute **zero** new contradiction/RED credit. The source/header and exact pair table is DESIGN-v2 11.1; PLAN-v2 task 1a freezes these as tests. The admitted conditional compound-block control has three CONFIRMED dropped pairs; the corrected direct-expression conditional is clang-accepted and source G1Error. Stringize/paste, rescan, lookahead, empty expansion, alias/chain and insertion token receipts are in DESIGN-v2 section 3.

### Recall and owner ledger

| corpus | confirmed drops lost / located confirmed | recall cost | refused / located on owners |
|---|---:|---:|---:|
| redis | 7 / 21,605 | 0.032% | 2 / 5,062 |
| cpython | 2,215 / 15,220 | 14.55% | 284 / 5,425 |
| leveldb | 0 / 473 | 0% | 0 / 1,384 |
| protobuf | 24 / 422 | 5.69% | 140 / 7,295 |
| re2 | 0 / 38 | 0% | 5 / 886 |
| snappy | 0 / 19 | 0% | 1 / 222 |
| total | **2,246 / 37,777** | **5.95%** | **432 / 20,274** |

Reasons for lost pairs: cpython `block_list` 275, `unbalanced` 51, `exposed_hidden_opener` 67, `loose_in_owner_group` 1,822; redis loose-in-group 7; protobuf held-argument split 24. Other corpora lose no confirmed drops, though some owners with no such drops are refused. Relative to 10n+31: +1,267 pairs / +200 owners; relative to 10+31: +1,847 pairs / +218 owners. B10 equals 12 on all L3 pair cells: body-root broadening costs no additional measured pairs but is required by `body_identity`.

### Limits and exclusions

- 18 unlocated cpython CONFIRMED pairs excluded from recall; 366 unmatched owner starts excluded from owner denominator (cpython 98, leveldb 23, protobuf 207, re2 34, snappy 4). No L1 row was unlocated. No must-keep was excluded as unlocated.
- 3,361 L3 UNJUDGED/XOFF pairs are not truth or recall evidence; 58 of them are refused. Saved corpora use one host configuration and the existing stubbed-header treatment. Token-level owner/census emulation and lexing limitations remain as section 6; the new rule in Rust is unverified.
- Current probes use the same unchanged binary for all four arms. They prove the old source drop and the candidate's predicted refusal, not implemented GREEN. Alias rows are conservatively closed even where the older A-ext boundary would disclose them; the identity-wrapper witnesses require the repair without crossing that boundary.
- No full Rust suite, release rebuild, Tier-A, mutation execution, new generator family, corpus runtime cost or new header/GitHub prevalence survey was run in this design-only task. Tier-A is now explicitly in PLAN-v2. No commit/push; controller owns durable Git custody.

**Verification follow-up:** `python3 scripts/dfg_prune/xscore_revision_tests.py -v` passes all 20 regression tests. The identical assertions against HEAD's exported scorer and prior candidate 10+31 produce 11 failures / 9 passes, covering the changed behavior with tight/literal/outside/ordinary-vararg controls. All population scores above were rerun unchanged. See root `VERIFICATION.md` for exact commands and the environment-limited full Rust attempt (mandatory memguard exit 76 before Cargo; 0 Rust tests executed).
