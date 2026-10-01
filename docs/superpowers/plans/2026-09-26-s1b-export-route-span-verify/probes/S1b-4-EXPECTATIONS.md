# S1b-4 r2 expectations, 2026-10-01 (pre-run)

READ: base is main 915fca43, with S1b-3b. MEASURED: the original generator
produces 287 scenarios. ASSUMPTION: the prototype should preserve the original
287 summaries except the namespace rows enumerated below; real-corpus counts
are not assumed from the old empty X/R/T expectations.

READ: E5 and Option K keep base behavior. A qualifier alias can contain callable
members even when the qualifier itself is not a function; neither a lexical
auditor nor JsBinding::Refused("not_callable") establishes absence of member
value flow. Do not use that result to claim zero right member edges lost.

ASSUMPTION: proven namespace imports use export identity: C62, C80, C81 and C148
lose only decoys; C82's ordinary call refuses WrappedExportNonJsx and its JSX
site resolves; C83 adds the renamed export; C131/C132 keep eligible exports at
NameOnly; C133 refuses the unrelated directory's member. C130 is written and
must stay at base. C128 already changed in S1b-1 and is a preservation control.
C84/C85 are non-goals and stay at base. C112/C129 lose namespace authority and
continue through the existing receiver ladder, unless a parse refusal applies.

ASSUMPTION: new C190–C220 have JSX and TSX twins. Clean direct/renamed/star
exports resolve to the registered exported span, never nested or other-directory
decoys. Written qualifiers, alias qualifiers, may-call export terminals,
unproven positions and non-namespace imports keep base. A pure type import and
`import x = require()` never become namespace proof. `export * as inner` is an
object: `ns.inner()` gets no callable authority; nested `ns.inner.f()` keeps
base (no new namespace-object traversal).

ASSUMPTION (pre-run C220): `make()` returns its nested function f; the exported
`const f = make()` therefore carries that callable, and base R3 resolves to
that same nested f. Literal reuse of the D4 table would remove a right edge:
D4 deliberately maps this alias to UnprovenLocal. OQ-S1b4 must settle the
namespace-only preservation policy before a prototype is treated as accepted.

ASSUMPTION: RP2-c (all four grammar/extension twins) changes UnknownName to
Exact import_qualified targeting lib's f@1, with member g resolved via its
StringValue export fact. The other 42 RP scenarios retain base rows.

ASSUMPTION: parameter-default and computed-key positions bind the outer import
according to the landed E_TABLE; a body-only declaration must not suppress them.
Unsealed recovery refuses under E6; an unrelated sealed body error keeps proof.
Every expectation is checked against the same-environment base before attribution.

## Post-run disposition, 2026-10-01 (registration above retained)

READ: controller resolved OQ-S1b4-1 under existing OQ12/Option K: Alias
terminals keep base through namespace-only opacity, including named/star barrels;
original D4 rows/counters stay unchanged. Prototype custody is available and
controller WIP39faa3aa captures its completed source body.
MEASURED: the registered 349 controls reach their correct columns; all 46 changed
rows (18 original, 28 new) are listed individually in ../S1b-4-CONTROLS.md.
C206/C208 actually change from base refusal to the correct Exact outer import,
not just preservation guards. C202/C203 already refuse on base; sealed C204
keeps proof. C220 preserves its independently evaluated callable source identity.
RP2-c is GREEN in all four twins, with other42 sections unchanged. X/R/T have
zero changed rows and zero lost targets under complete-key/multiset comparison.
Full measurements, limits and exclusions are in ../S1b-4-MEASUREMENTS.md; F is
controller-only and pending. No preregistered expectation was rewritten into a
post-hoc baseline.
