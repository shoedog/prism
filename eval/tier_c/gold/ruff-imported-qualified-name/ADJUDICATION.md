# Adjudication — ruff-imported-qualified-name

repo: ruff  sha: 44f6d18  symbol: qualified_name (trait Imported<'a>, binding.rs:717-719)

## Task shape (per spec: D2/PRECISION, shallow — NOT a forwarder closure)

Unlike the match_annotation task, this is a flat receiver-disambiguation exercise: find every
real direct call site of `Imported::qualified_name()` (impls: `Import` binding.rs:740,
`SubmoduleImport` binding.rs:761, `FromImport` binding.rs:782, `AnyImport` binding.rs:810 —
tag-dispatch forwarding to the other three) across the whole ruff_linter/ruff_python_semantic
codebase, in a field dominated by same-name collisions.

## Generator of record

`git grep -nw qualified_name` at the SHA returns **873 hits across 225 files** (confirms the
corpus's own count exactly). That field is dominated by (a) the `QualifiedName` TYPE (different
capitalization, but every constructor/method/local-var mentioning it also contains the lowercase
word as a substring of things like `resolve_qualified_name`, `QualifiedNameBuilder`, etc. — none
of which are WORD-boundary hits on bare `qualified_name` itself, so they don't even enter the 873)
and (b) genuine same-name methods/fields on unrelated types. Rather than manually triaging 873
raw hits, I narrowed the **generator** to the METHOD-CALL-SYNTAX form specifically —
`git grep -n "\.qualified_name(" -- '*.rs' ':!*/tests/*' ':!*test*.rs'` — which returns exactly
**48 raw line-hits across 19 files**. This is a strictly more precise generator, not a
truncation: struct-FIELD access (`x.qualified_name`, no parens — the binding.rs:500/511/520
decoys) and `fn qualified_name` DEFINITIONS never match `\.qualified_name\(`, so nothing that
would matter is silently dropped; every remaining ambiguity is a genuine same-name-METHOD,
different-receiver collision, which is exactly the disambiguation the task calls "the hard
part." Each of the 48 raw hits was then read in context to determine the STATIC TYPE of the
value immediately left of the dot (receiver_evidence field per site).

## Receiver disambiguation — the full breakdown

Of 48 raw hits, **24 were phantom bait** (same-name method on an unrelated type), confirmed by
reading each receiver's declared/inferred type:

1. **`Module::qualified_name`** (definition.rs:63, the corpus's named decoy) — 7 call sites:
   `checker.module.qualified_name()` in statement.rs (×4: 633/777/835/921) and
   banned_module_level_imports.rs (×1: 99), `self.module.qualified_name()` in model.rs (×2:
   1073/1103). `checker.module` / `self.module` are both typed `Module<'a>`
   (definition.rs:54), NOT Imported/AnyImport.
2. **`NameImport::qualified_name`** (imports.rs:60, the corpus's named decoy) — 5 sites:
   `required_import.qualified_name()` in unnecessary_future_import.rs (×2: 112/125, where
   `required_import: &NameImport` via `type RequiredImports = BTreeSet<NameImport>`) and
   configuration.rs (×2: 1696/1735, `isort.required_imports: BTreeSet<NameImport>`), plus the
   `self.qualified_name()` HALF of the mixed line at imports.rs:75 (see below).
3. **ty-crate `Class`/`ClassLiteral`/`TypeAlias::qualified_name`** (a completely separate crate,
   `ty_python_semantic`, unrelated to `ruff_python_semantic`'s Imported trait) — 8 sites:
   class.rs:1013, diagnostic.rs:3687/3872/4040, display.rs:75/77/778/842.
4. **`AnyImport::qualified_name`'s own tag-dispatch impl body** (binding.rs:812-814) — 3 sites:
   `Self::Import(import) => import.qualified_name()` etc. — this IS one of the trait's "4 impls"
   (part of S's own definition surface), not an external caller. Excluded on the same principle
   as Task 1's "S's own implementation is not gold."
5. **`Import::member_name` / `SubmoduleImport::member_name`'s internal self-calls**
   (binding.rs:751, 772) — a SIBLING trait method calling `qualified_name` from within the SAME
   impl block, in the trait's own home file. Same "part of the definition, not a consumer"
   reasoning as #4 — **flagged as a judgment call**, see Uncertain sites below.

**24 remaining raw hits are real**, collapsing to **18 (file, symbol) gold entries across 11
files** (several files/functions have 2-3 raw call sites that collapse to one entry: e.g.
`runtime_import_in_type_checking_block` 3->1, `typing_only_runtime_import` 3->1, `unused_import`
3->1, `mark_uses_of_qualified_name` 2->1).

**One line needed sub-line disambiguation**: `imports.rs:75` —
`name == self.bound_name() && self.qualified_name() == *binding.qualified_name()` inside
`NameImport::matches(&self, name, binding: &AnyImport)` — `self.qualified_name()` is the DECOY
(self: &NameImport) but `binding.qualified_name()` on the SAME LINE is REAL (binding: &AnyImport).
Included as one gold site, with both halves documented in `reason`.

## Cross-check (LSP + prism, via `tier-c build-gold`)

- **prism's `nav_callers` on `qualified_name`** returned `AmbiguousSymbol`, listing the exact
  same 4 trait impls (binding.rs:740/761/782/810) PLUS the exact same 2 decoy definitions
  (definition.rs:63, imports.rs:60) I found by source-reading — independent structural
  confirmation of the decoy inventory (matches Task 1's cross-check pattern).
- **rust-analyzer (LSP) call-hierarchy**, seeded on one specific impl, returned **18 raw
  candidates**. 16 matched my gold list exactly (by file+symbol). 2 were sites I judged
  impl-internal and excluded (`binding.rs:772` member_name, `binding.rs:814` AnyImport's own
  dispatch) — LSP's raw call-hierarchy doesn't apply the "part of definition" convention, it
  just reports callers, so this is expected, not a disagreement to adjudicate away.
  **LSP MISSED `import_private_name.rs:183`** (the `From<&Import>` sibling of the
  `From<&FromImport>` impl it DID find at line 196) entirely — a genuine oracle_miss
  (LSP false-negative), only recovered by the exhaustive grep-per-hop sweep. Folded into gold
  per the design's Sec 5a LSP-fallibility handling (prism/grep-recovered real sites are credit,
  not penalized).

## D-membership

D-membership is keyed to S's own name ("qualified_name"). By construction, **every gold site's
file necessarily contains the literal text "qualified_name"** (that is how the grep-per-hop
generator found it), so **D1 is structurally impossible for this task** — every real site is
**D2** (name present, repo-wide total 873 >> the D2 threshold of 100). This matches the corpus's
own framing exactly ("d_member here is mostly D2"). Gold: 18/18 D2.

## Admission

|gold(real)| = 18 (within [8,60] ✓). D1=0, D2=18, (D1+D2)/|gold| = 1.0 ≥ 0.3 ✓. PASSES admission.

Note: my rigorously-verified 18-site count is well under the corpus's "~40 sites/~16 files"
starting estimate. Per instructions ("VERIFY the full set... not trust"), I traced every one of
the 48 raw `.qualified_name(` call-syntax hits to a concrete receiver type rather than accepting
the estimate; the gap is fully accounted for by the 24 phantom-bait hits documented above (the
"~40" estimate appears to have been a rough pre-verification guess across the full noise field,
not a source-checked count). I did not find evidence of missed real call sites: a broader sweep
for UFCS-style (`Imported::qualified_name(x)`) or inherent-`Binding`-method call forms found none
(see below).

## Dry-run (scorer sanity check)

```
perfect arm:      file_f1=1.0  d_recall=1.0  gold_size=17  d_gold_size=11  phantom=0
grep-S-only arm:  file_f1=0.0  d_recall=0.0  gold_size=17  d_gold_size=11  phantom=0
```
(`grep-S-only` = an off-arm that only found `binding.rs::qualified_name`, i.e. stopped at S's own
home file without disambiguating any receiver — scores zero, as expected, since binding.rs's
internal sites are correctly excluded from gold.)

**Note on gold_size=17 vs. 18 real sites in the JSON**: the scorer's `norm_symbol()` strips
everything before a `::` for site-key matching (by design, to tolerate arm symbol-spelling
variance). My two `import_private_name.rs` entries — `<ImportInfo as From<&FromImport>>::from`
and `<ImportInfo as From<&Import>>::from` — both normalize to bare `from` in the SAME file, so
they collapse to one scoring key. This does not affect the primary file-F1 metric (same file
either way) and is a property of the scorer's intentional normalization, not a gold.json defect;
flagged here for transparency rather than renamed away (the actual Rust identifier for both really
is `from`, being two trait-impl methods of the same name).

## Uncertain sites flagged for controller

1. **binding.rs:751/772 (`Import::member_name` / `SubmoduleImport::member_name`'s internal
   `self.qualified_name()` calls)** — excluded as "part of the trait's own definition/plumbing
   file," mirroring the Task 1 precedent (a trait's own impl bodies aren't gold). A stricter
   reading would include them (LSP's raw call-hierarchy DID surface :772 as a candidate, with
   no built-in notion of "definition-internal"). Does not change admission or file_f1 either way
   (same file, binding.rs, already excluded overall) but would add ~2 sites if the controller
   wants literal completeness over the "consumer, not producer" interpretive filter.
2. **imports.rs:75's split-line disambiguation** — a single source line carries both a decoy
   and a real call. I recorded it as ONE gold site with both halves documented; an alternative
   would be a stricter reading that the WHOLE LINE is definitionally ambiguous and should be
   dropped or hand-flagged rather than resolved unilaterally. I resolved it because the receiver
   types are unambiguous from local source (self: &NameImport vs binding: &AnyImport are both
   explicit in the function signature) — no genuine ambiguity, just co-location.
3. **`unused_import.rs` contributing 5 of the 18 gold entries** — this file is unusually
   qualified_name-heavy (a whole rule module built around ranking/matching imports by qualified
   name). Not a modeling uncertainty, just worth the controller's awareness that the gold set's
   file diversity (11 files) is smaller than its site count (18) might suggest, with
   `unused_import.rs` and `redefined_while_unused.rs`/`import_private_name.rs` each contributing
   multiple entries.
