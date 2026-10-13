# Alias-following variant matrix

1065 enumerated fixtures, each a small repo with one marked call site and an authored ground truth. The pinned TypeScript 5.9.3 checker can type the callee on 671 of them (the rest go through `any`, a parameter or a wrapper) and agrees with the authored truth on 657. The 14 that differ are shapes where the checker answers a different question: `destructured-props-default-overridden` (6): in JavaScript the checker types a defaulted destructured parameter by its default, so it names the default; the caller passes a different function; `named-js-suffix-dts-only` (1): the checker names the `.d.ts` signature, which has no body; there is no in-repo callable; `hook-result-two-slots` (6): in JavaScript the checker types the returned array literal as an array of a union and names its first member; the destructure takes the second slot; `named-js-suffix-literal-and-ts` (1): policy row: TypeScript binds `x.ts`; prism keeps the literal `x.js` as main does. Binaries: `main` = frozen base (main `bab0dfbd`), `e5` = E5 step-2b head, `i1` = the import-suffix branch `aa74eae0`.

Verdicts: CORRECT (Exact edge to the true callee), PARTIAL (Exact to some of a closed set), MISSING (no edge, a true in-repo callee exists), WRONG (Exact edge to something else), OVERCLAIM (Exact where the callee is only possible), MAY-NONE (no edge where the callee is only possible), NONE-OK (no edge, no in-repo callee), NO-RECORD (prism has no call-site record there).

## Totals by lookup step

**main**

| Step | Fixtures | CORRECT | PARTIAL | MISSING | WRONG | OVERCLAIM | MAY-NONE | NONE-OK | NO-RECORD |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| local | 788 | 72 | 8 | 489 | 114 | 6 | 41 | 28 | 30 |
| param-args | 156 |  |  | 144 |  |  | 12 |  |  |
| param-lookup | 6 |  |  |  |  |  | 6 |  |  |
| import | 31 | 21 |  | 4 |  |  |  | 6 |  |
| import-path | 34 | 12 |  | 19 |  |  |  | 3 |  |
| same-file | 28 | 24 |  |  |  |  |  |  | 4 |
| global | 11 | 9 |  |  | 2 |  |  |  |  |
| none | 11 | 2 |  | 6 | 3 |  |  |  |  |

**e5**

| Step | Fixtures | CORRECT | MISSING | WRONG | MAY-NONE | NONE-OK | NO-RECORD |
|---|---:|---:|---:|---:|---:|---:|---:|
| local | 788 | 22 | 607 | 42 | 57 | 30 | 30 |
| param-args | 156 |  | 144 |  | 12 |  |  |
| param-lookup | 6 |  |  |  | 6 |  |  |
| import | 31 | 20 | 5 |  |  | 6 |  |
| import-path | 34 | 12 | 19 |  |  | 3 |  |
| same-file | 28 | 24 |  |  |  |  | 4 |
| global | 11 | 5 | 4 |  |  | 2 |  |
| none | 11 | 2 | 6 |  |  | 3 |  |

**i1**

| Step | Fixtures | CORRECT | PARTIAL | MISSING | WRONG | OVERCLAIM | MAY-NONE | NONE-OK | NO-RECORD |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| local | 788 | 72 | 8 | 489 | 114 | 6 | 41 | 28 | 30 |
| param-args | 156 |  |  | 144 |  |  | 12 |  |  |
| param-lookup | 6 |  |  |  |  |  | 6 |  |  |
| import | 31 | 21 |  | 4 |  |  |  | 6 |  |
| import-path | 34 | 30 |  | 1 |  |  |  | 3 |  |
| same-file | 28 | 24 |  |  |  |  |  |  | 4 |
| global | 11 | 9 |  |  | 2 |  |  |  |  |
| none | 11 | 2 |  | 6 | 3 |  |  |  |  |

## Per shape

One row per shape, scope, invocation form and naming; `ts` and `js` are merged when they agree. Naming: `same` = the alias has the true callee's name; `trap` = the alias has the name of a different, repo-unique callable; `fresh` = the alias name matches nothing.

| Step | Shape | Scope | Form | Naming | Truth | main | e5 | i1 | Contract question |
|---|---|---|---|---|---|---|---|---|---|
| local | ident | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ident | method | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | arrow | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ident | method | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | arrow | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | method | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | arrow | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | jsx | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ident | fn | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | jsx | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ident | fn | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | outer | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | new | same | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ident | fn | new | trap | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ident | fn | new | fresh | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ident | fn | opt | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | opt | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident | fn | opt | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member | method | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | arrow | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member | method | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | arrow | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | method | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | arrow | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | jsx | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member | fn | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | jsx | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member | fn | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | outer | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | new | same | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ns-member | fn | new | trap | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ns-member | fn | new | fresh | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ns-member | fn | opt | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | opt | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member | fn | opt | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-destructure | method | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | arrow | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-destructure | method | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | arrow | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | method | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | arrow | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | fn | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | jsx | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-destructure | fn | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | jsx | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-destructure | fn | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | outer | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-destructure | fn | new | same | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ns-destructure | fn | new | trap | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ns-destructure | fn | new | fresh | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | obj-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | obj-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | obj-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | obj-member | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | chain2 | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | chain2 | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain2 | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | let-stable | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | let-stable | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-stable | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-reassigned | fn | call | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-reassigned | outer | call | same | closed set | PARTIAL via free_single | MISSING (LocalBindingUnproven) | PARTIAL via free_single |  |
| local | let-reassigned | fn | call | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-reassigned | outer | call | trap | closed set | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | let-reassigned | fn | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | let-reassigned | outer | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | var | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | var | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | var | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | fn | call | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | call | same | closed set | PARTIAL via free_single | MISSING (LocalBindingUnproven) | PARTIAL via free_single |  |
| local | conditional | fn | call | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | call | trap | closed set | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | conditional | fn | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | fn | jsx | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | jsx | same | closed set | PARTIAL via free_single | MISSING (LocalBindingUnproven) | PARTIAL via free_single |  |
| local | conditional | fn | jsx | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | jsx | trap | closed set | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | conditional | fn | jsx | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | outer | jsx | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | conditional | fn | new | same | closed set | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | conditional | fn | new | trap | closed set | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | conditional | fn | new | fresh | closed set | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | wrapper-call | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | jsx | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | jsx | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | outer | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | new | same | one callee | NO-RECORD | NO-RECORD | NO-RECORD | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | new | trap | one callee | NO-RECORD | NO-RECORD | NO-RECORD | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | wrapper-call | fn | new | fresh | one callee | NO-RECORD | NO-RECORD | NO-RECORD | wrapper call result (memo, useCallback, forwardRef): is the wrapped callable the callee? |
| local | bind | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | .bind result |
| local | bind | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | .bind result |
| local | bind | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | bind | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | .bind result |
| local | array-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | array-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | array-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | array-destructure | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | default-destructure | fn | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | default-destructure | outer | call | same | possible only | OVERCLAIM via free_single | MAY-NONE (LocalBindingUnproven) | OVERCLAIM via free_single |  |
| local | default-destructure | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | default-destructure | outer | call | trap | possible only | WRONG via free_single | MAY-NONE (LocalBindingUnproven) | WRONG via free_single |  |
| local | default-destructure | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | default-destructure | outer | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | fallback-or | fn | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | fallback-or | outer | call | same | possible only | OVERCLAIM via free_single | MAY-NONE (LocalBindingUnproven) | OVERCLAIM via free_single |  |
| local | fallback-or | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | fallback-or | outer | call | trap | possible only | WRONG via free_single | MAY-NONE (LocalBindingUnproven) | WRONG via free_single |  |
| local | fallback-or | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | fallback-or | outer | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | ns-member-paren | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member-paren | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member-paren | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-paren | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member-as | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member-as | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-as | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member-nonnull | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member-nonnull | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-nonnull | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | block-shadow | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | block-shadow | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | block-shadow | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | react-memo | module | call | same | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | fn | call | same | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | fn | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | module | call | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | fn | call | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-memo | module | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-memo | fn | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-memo | module | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-memo | fn | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-memo | module | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-memo | fn | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (memo, forwardRef, observer): is the wrapped callable the callee? |
| local | react-usecallback-ident | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (useCallback, useMemo): is the wrapped callable the callee? |
| local | react-usecallback-ident | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (useCallback, useMemo): is the wrapped callable the callee? |
| local | react-usecallback-ident | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | library wrapper (useCallback, useMemo): is the wrapped callable the callee? |
| local | react-usecallback-inline | fn | call | same | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | library wrapper around an inline callable |
| local | react-usecallback-inline | fn | call | trap | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | library wrapper around an inline callable |
| local | react-usecallback-inline | fn | call | fresh | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | library wrapper around an inline callable |
| local | react-usestate-setter | fn | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-usestate-setter | outer | call | trap | no in-repo callee | WRONG via free_single | NONE-OK (LocalBindingUnproven) | WRONG via free_single |  |
| local | react-usestate-setter | fn | call | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-usestate-setter | outer | call | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-usecontext-destructure | fn | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | context value: the default is visible, providers may not be |
| local | react-usecontext-destructure | outer | call | same | possible only | OVERCLAIM via free_single | MAY-NONE (LocalBindingUnproven) | OVERCLAIM via free_single | context value: the default is visible, providers may not be |
| local | react-usecontext-destructure | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | context value: the default is visible, providers may not be |
| local | react-usecontext-destructure | outer | call | trap | possible only | WRONG via free_single | MAY-NONE (LocalBindingUnproven) | WRONG via free_single | context value: the default is visible, providers may not be |
| local | react-usecontext-destructure | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | context value: the default is visible, providers may not be |
| local | react-usecontext-destructure | outer | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | context value: the default is visible, providers may not be |
| local | custom-hook-array-result | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | custom-hook-array-result | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | value returned by a same-file function (hook result) |
| local | custom-hook-array-result | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | custom-hook-array-result | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | value returned by a same-file function (hook result) |
| local | custom-hook-array-result | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | custom-hook-array-result | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | styled-template | module | jsx | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | styled-template | fn | jsx | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | styled-template | module | jsx | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | styled-template | fn | jsx | fresh | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | react-lazy | module | jsx | same | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def | lazy(() => import(...)): is the default export the callee? |
| local | react-lazy | module | jsx | trap | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def | lazy(() => import(...)): is the default export the callee? |
| local | react-lazy | module | jsx | fresh | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def | lazy(() => import(...)): is the default export the callee? |
| local | element-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | element access into a local array literal |
| local | element-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | element access into a local array literal |
| local | element-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | element-member | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | element access into a local array literal |
| local | deep-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | deep-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | deep-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | deep-member | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | declared-then-assigned | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | declared without a value, assigned once |
| local | declared-then-assigned | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | declared without a value, assigned once |
| local | declared-then-assigned | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | declared without a value, assigned once |
| local | declared-then-assigned | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | declared without a value, assigned once |
| local | declared-then-assigned | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | declared without a value, assigned once |
| local | declared-then-assigned | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | declared without a value, assigned once |
| local | for-of-element | fn | call | same | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | loop variable over a local array literal |
| local | for-of-element | fn | call | trap | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | loop variable over a local array literal |
| local | for-of-element | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | loop variable over a local array literal |
| local | object-param-destructure | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | object-param-destructure | outer | call | trap | possible only | WRONG via free_single | MAY-NONE (LocalBindingUnproven) | WRONG via free_single |  |
| local | object-param-destructure | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | object-param-destructure | outer | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| same-file | local-arrow | fn | call | same | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | module | call | same | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | outer | call | same | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | fn | call | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | module | call | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | outer | call | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | fn | jsx | same | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | outer | jsx | same | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | fn | jsx | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | outer | jsx | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | local-arrow | fn | new | same | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| same-file | local-arrow | fn | new | trap | one callee | NO-RECORD | NO-RECORD | NO-RECORD |  |
| local | ts-import-equals-member | module | call | same | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single |  |
| local | ts-import-equals-member | module | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ts-import-equals-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | hook-result-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | hook-result-destructure | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | value returned by a same-file function (hook result) |
| local | hook-result-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | hook-result-destructure | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | value returned by a same-file function (hook result) |
| local | hook-result-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | hook-result-destructure | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result) |
| local | await-import-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-default | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-default | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | await-import-default | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| local | ns-member-satisfies | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member-satisfies | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-satisfies | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | exported-ns-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | exported-ns-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | exported-ns-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident-of-reassigned-let | fn | call | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident-of-reassigned-let | outer | call | same | closed set | PARTIAL via free_single | MISSING (LocalBindingUnproven) | PARTIAL via free_single |  |
| local | ident-of-reassigned-let | fn | call | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident-of-reassigned-let | outer | call | trap | closed set | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ident-of-reassigned-let | fn | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ident-of-reassigned-let | outer | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain8 | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain8 | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain8 | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | chain9 | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | nine alias hops: one past the proposed eight-hop bound |
| local | chain9 | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | nine alias hops: one past the proposed eight-hop bound |
| local | chain9 | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | nine alias hops: one past the proposed eight-hop bound |
| local | alias-of-param | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | alias-of-param | outer | call | trap | possible only | WRONG via free_single | MAY-NONE (LocalBindingUnproven) | WRONG via free_single |  |
| local | alias-of-param | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | alias-of-param | outer | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | catch-binding | fn | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | catch-binding | fn | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | shadows-same-file-fn | fn | call | same | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn | outer | call | same | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn | fn | call | trap | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn | outer | call | trap | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn | fn | call | fresh | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn | outer | call | fresh | one callee | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | fn | call | same | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | outer | call | same | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | fn | call | trap | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | outer | call | trap | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | fn | call | fresh | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-props | outer | call | fresh | possible only | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | fn | call | same | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | outer | call | same | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | fn | call | trap | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | outer | call | trap | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | fn | call | fresh | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | shadows-same-file-fn-let | outer | call | fresh | closed set | WRONG via local_def | WRONG via local_def | WRONG via local_def |  |
| local | call-with-inline-fn | fn | call | same | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | call-with-inline-fn | outer | call | same | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | call-with-inline-fn | fn | call | trap | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | call-with-inline-fn | outer | call | trap | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | call-with-inline-fn | fn | call | fresh | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | call-with-inline-fn | outer | call | fresh | inline callable | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def | result of a call that takes an inline function: main binds the inline function (may-call ruling, 2026-09-27) |
| local | rhs-shadowed-at-site | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | rhs-shadowed-at-site | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | rhs-shadowed-at-site | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-via-alias-hop | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | ns-via-alias-hop | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | namespace object carried through an alias before the member access |
| local | hook-result-two-keys | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-keys | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-keys | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-keys | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-keys | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-keys | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable keys |
| local | hook-result-two-slots | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable slots |
| local | hook-result-two-slots | outer | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | value returned by a same-file function (hook result), second of two callable slots |
| local | hook-result-two-slots | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable slots |
| local | hook-result-two-slots | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single | value returned by a same-file function (hook result), second of two callable slots |
| local | hook-result-two-slots | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable slots |
| local | hook-result-two-slots | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | value returned by a same-file function (hook result), second of two callable slots |
| local | ns-member-exported-alias | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-exported-alias | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-exported-alias | outer | call | trap | one callee | WRONG via free_single | MISSING (LocalBindingUnproven) | WRONG via free_single |  |
| local | ns-member-exported-alias | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-exported-alias | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | ns-member-exported-alias | outer | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure | module | call | same | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| local | require-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure | module | call | trap | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| local | require-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure | module | call | fresh | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| local | require-inline-member | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-inline-member | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-inline-member | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-inline-member | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-inline-member | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-inline-member | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-destructure-parens | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | module | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | fn | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | module | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | fn | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | require-assign-destructure | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | call | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | call | same | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) |  |
| param-args | props-destructure-in-body | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | props-object-member | param | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | jsx | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | jsx | same | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | jsx | same | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) |  |
| param-args | props-destructure-in-body | param | jsx | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | inline-arrow-arg | param | call | same | inline callable | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | call | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | call | trap | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) |  |
| param-args | props-destructure-in-body | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | props-object-member | param | call | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | jsx | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | jsx | trap | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | jsx | trap | one callee | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) | MISSING (LocalBindingUnproven) |  |
| param-args | props-destructure-in-body | param | jsx | trap | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | inline-arrow-arg | param | call | trap | inline callable | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | call | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | call | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | props-destructure-in-body | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | props-object-member | param | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-one-caller | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-exported | param | jsx | fresh | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | exported callable: callers may be invisible |
| param-args | plain-two-callers | param | jsx | fresh | closed set | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | plain-member-arg | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | forwarded-twice | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | default-value | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | parameter default used at the only visible call site |
| param-args | destructured-props | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-renamed | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | destructured-props-default-unused | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | destructured parameter default used at the only visible call site |
| param-args | destructured-props-default-overridden | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | closure-param | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | props-destructure-in-body | param | jsx | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-args | inline-arrow-arg | param | call | fresh | inline callable | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-lookup | shadows-import | param | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| local | inner-shadows-import | fn | call | same | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| param-lookup | ns-receiver-is-parameter | param | call | same | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| param-lookup | inner-param-shadows-alias | param | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| import | named | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | renamed | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | renamed-to-decoy | module | call | trap | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | default | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | ns-qualified | module | call | n/a | one callee | CORRECT via import_qualified | CORRECT via import_qualified | CORRECT via import_qualified |  |
| import | barrel-named | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | barrel-star | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | barrel-renamed | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | named-exported-alias | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| none | object-literal-member-call | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | qualified callee through an object literal: outside the bare-name lookup |
| none | object-shorthand-member-call | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | qualified callee through an object literal: outside the bare-name lookup |
| none | props-member-call | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | qualified callee through props: outside the bare-name lookup |
| import | external-named-trap | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| import | external-default-trap | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| local | external-ns-member-alias-trap | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| import | named-jsx | module | jsx | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import | ns-qualified-jsx | module | jsx | n/a | one callee | CORRECT via import_qualified | CORRECT via import_qualified | CORRECT via import_qualified |  |
| import | external-jsx-trap | module | jsx | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| import-path | named-js-suffix | module | call | n/a | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| import-path | renamed-to-decoy-js-suffix | module | call | trap | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| import-path | default-js-suffix | module | call | n/a | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| import-path | ns-qualified-js-suffix | module | call | n/a | one callee | CORRECT via import_qualified | CORRECT via import_qualified | CORRECT via import_qualified |  |
| import-path | barrel-named-js-suffix | module | call | n/a | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| import-path | barrel-star-js-suffix | module | call | n/a | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| import-path | named-jsx-js-suffix | module | jsx | n/a | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| local | ns-member-alias-js-suffix | module | call | fresh | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| import-path | missing-export-js-suffix-trap | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| import-path | dot-index | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | dotdot-index | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | redundant-slash | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | dot-segment | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | mid-dotdot-segment | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | same-stem-two-dirs | module | call | trap | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member |  |
| import-path | same-stem-two-dirs-js-suffix | module | call | trap | one callee | ts: MISSING (UnknownName); js: CORRECT via import_member | ts: MISSING (UnknownName); js: CORRECT via import_member | CORRECT via import_member |  |
| same-file | hoisted-function | module | call | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| same-file | nested-shadows-import | module | call | trap | one callee | CORRECT via local_def | CORRECT via local_def | CORRECT via local_def |  |
| none | module-unbound-module-target | module | call | trap | no in-repo callee | WRONG via free_single | NONE-OK (LocalBindingUnproven) | WRONG via free_single |  |
| global | lib-global-trap | module | call | trap | no in-repo callee | WRONG via free_single | NONE-OK (LocalBindingUnproven) | WRONG via free_single |  |
| global | module-to-script-global | module | call | n/a | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single |  |
| global | script-to-script-global | module | call | n/a | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single |  |
| import | import-equals-require | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) |  |
| local | import-equals-exported-alias | module | call | n/a | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| global | namespace-body-script-global | module | call | n/a | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| import | super-extends-member | module | call | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | super(...) through an imported member |
| import-path | named-js-suffix-tsx-target | module | jsx | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | CORRECT via import_member |  |
| import-path | named-jsx-suffix-tsx-target | module | jsx | n/a | one callee | MISSING (UnknownName) | MISSING (UnknownName) | MISSING (UnknownName) | explicit .jsx suffix naming a .tsx file: not modeled by the import-suffix slice |
| import-path | named-js-suffix-literal-and-ts | module | call | n/a | one callee | CORRECT via import_member | CORRECT via import_member | CORRECT via import_member | policy row: both x.js and x.ts exist; prism keeps the literal file (as main does), TypeScript itself would bind x.ts |
| import-path | named-js-suffix-dts-only | module | call | trap | no in-repo callee | NONE-OK (UnknownName) | NONE-OK (UnknownName) | NONE-OK (UnknownName) |  |
| global | enum-merged-ns-script-global | module | call | n/a | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| global | nonexported-merged-ns-plus-import-script-global | module | call | n/a | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| global | decorated-class-expr-script-global | module | call | n/a | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single |  |
| global | single-ns-script-global | module | call | n/a | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single |  |
| none | merged-ns-module-target-trap | module | call | trap | no in-repo callee | WRONG via free_single | NONE-OK (LocalBindingUnproven) | WRONG via free_single |  |
| local | merged-ns-local-alias-unknown | module | call | trap | possible only | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) | MAY-NONE (UnknownName) |  |
| none | static-property-jsx | module | jsx | n/a | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single | indirect record through a static property (E5 review round 1): outside the bare-name lookup |
| none | static-property-call | module | call | n/a | one callee | CORRECT via free_single | CORRECT via free_single | CORRECT via free_single | indirect record through a static property (E5 review round 1): outside the bare-name lookup |
| import | super-extends-member-named-super | module | call | same | one callee | CORRECT via free_single | MISSING (LocalBindingUnproven) | CORRECT via free_single | super(...) through an imported member (E5 review round 2, constructed) |

## Population per census class

Recorded product sites per binding class from the census (X = Excalidraw, T = TypeScript compiler, F = private corpus, aggregate counts only), with what main does there today (C correct / W wrong / other) and the matrix shapes that cover the class. A class with sites and no shape is a hole in the matrix.

| Census class | X sites (C/W) | T sites (C/W) | F sites (C/W) | Matrix shapes |
|---|---|---|---|---|
| global_ambient_library | 122 (0/0) | 48 (0/4) | 112 (0/0) | lib-global-trap |
| import.default | 233 (88/0) | 0 | 646 (5/0) | default, external-default-trap, default-js-suffix |
| import.dynamic_import | 1 (0/0) | 0 | 0 | await-import-member, await-import-default |
| import.dynamic_import.destructured | 1 (0/0) | 0 | 4 (4/0) | await-import-destructure |
| import.equals | 0 | 10 (0/3) | 0 | import-equals-exported-alias, ts-import-equals-member, import-equals-require |
| import.named | 5977 (2347/0) | 14391 (0/0) | 5215 (74/0) | named, barrel-named, barrel-star, named-jsx, named-exported-alias, external-named-trap, external-jsx-trap, named-js-suffix, barrel-named-js-suffix, barrel-star-js-suffix, named-jsx-js-suffix, dot-index, dotdot-index, redundant-slash, dot-segment, mid-dotdot-segment, same-stem-two-dirs, same-stem-two-dirs-js-suffix |
| import.renamed | 7 (1/0) | 29 (0/0) | 28 (0/0) | renamed, renamed-to-decoy, barrel-renamed, renamed-to-decoy-js-suffix |
| import.require.destructured | 17 (0/0) | 0 | 9 (4/0) | require-destructure, require-destructure-parens, require-assign-destructure |
| local.const.arrow | 1537 (1501/0) | 78 (78/0) | 447 (431/0) | local-arrow |
| local.const.call_result | 66 (13/0) | 210 (0/0) | 276 (100/1) | wrapper-call, react-memo, react-usecallback-ident, react-usecallback-inline, react-lazy, bind, call-with-inline-fn |
| local.const.conditional | 0 | 32 (0/1) | 4 (0/0) | conditional, fallback-or |
| local.const.destructure.await | 6 (0/0) | 0 | 0 | await-import-destructure |
| local.const.destructure.call_result | 199 (0/8) | 36 (0/0) | 819 (0/70) | hook-result-destructure, custom-hook-array-result, react-usestate-setter, react-usecontext-destructure, hook-result-two-keys, hook-result-two-slots |
| local.const.destructure.identifier | 24 (0/11) | 130 (0/126) | 222 (0/37) | ns-destructure, object-param-destructure, props-destructure-in-body, default-destructure, shadows-same-file-fn-props |
| local.const.destructure.none | 0 | 4 (0/0) | 0 | for-of-element |
| local.const.element_member | 1 (0/0) | 3 (0/0) | 2 (0/0) | element-member |
| local.const.function_expression | 15 (15/0) | 0 | 3 (3/0) | local-arrow |
| local.const.identifier | 5 (0/0) | 3 (0/0) | 5 (0/0) | ident, chain2, chain8, chain9, alias-of-param, ident-of-reassigned-let, rhs-shadowed-at-site |
| local.const.identifier_member | 7 (0/0) | 47 (0/0) | 4 (0/1) | ns-member, ns-member-paren, ns-member-as, ns-member-nonnull, ns-member-exported-alias, obj-member, props-object-member, require-member, ns-member-satisfies, exported-ns-member, shadows-same-file-fn, ns-via-alias-hop, ns-receiver-is-parameter |
| local.const.none | 5 (0/0) | 3 (0/0) | 1 (0/0) | for-of-element |
| local.const.other | 2 (0/0) | 7 (0/1) | 258 (0/0) | styled-template, array-destructure |
| local.const.other_member | 2 (0/0) | 18 (0/0) | 0 | deep-member, require-inline-member |
| local.let.never_reassigned.none | 0 | 1 (0/0) | 0 | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.arrow | 0 | 1 (0/0) | 0 | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.call_result | 0 | 7 (0/0) | 0 | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.function_expression | 1 (0/0) | 0 | 0 | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.identifier | 2 (0/0) | 4 (0/0) | 1 (0/0) | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.none | 6 (0/0) | 6 (0/0) | 2 (0/0) | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.let.reassigned.other | 10 (0/0) | 0 | 0 | let-stable, let-reassigned, declared-then-assigned, shadows-same-file-fn-let |
| local.var.never_reassigned.call_result | 0 | 2 (0/0) | 0 | var |
| local.var.never_reassigned.destructure.call_result | 0 | 12 (0/0) | 0 | var |
| local.var.never_reassigned.destructure.conditional | 0 | 12 (0/0) | 0 | var |
| local.var.never_reassigned.destructure.identifier | 0 | 64 (0/17) | 0 | var |
| local.var.never_reassigned.none | 0 | 30 (0/30) | 0 | var |
| local.var.never_reassigned.other | 16 (0/0) | 0 | 0 | var |
| local.var.reassigned.identifier | 0 | 7 (0/0) | 0 | var |
| local.var.reassigned.none | 0 | 2 (0/0) | 0 | var |
| local.var.reassigned.other | 5 (0/0) | 0 | 0 | var |
| parameter.destructured | 201 (0/0) | 61 (0/0) | 39 (0/0) | destructured-props, destructured-props-renamed |
| parameter.destructured.default | 0 | 5 (0/0) | 12 (0/0) | destructured-props-default-unused, destructured-props-default-overridden |
| parameter.plain | 120 (0/0) | 844 (0/0) | 65 (0/0) | plain-one-caller, plain-exported, plain-two-callers, plain-member-arg, forwarded-twice, closure-param, inline-arrow-arg, shadows-import |
| parameter.plain.default | 3 (0/0) | 21 (0/0) | 3 (0/0) | default-value |
| same_file.function | 460 (456/0) | 17059 (17018/0) | 61 (53/0) | hoisted-function, nested-shadows-import |
| unresolved | 875 (0/0) | 10 (0/0) | 369 (0/0) | module-unbound-module-target, module-to-script-global, script-to-script-global, namespace-body-script-global, enum-merged-ns-script-global, nonexported-merged-ns-plus-import-script-global, decorated-class-expr-script-global, single-ns-script-global, merged-ns-module-target-trap |

Classes with sites and no fixture: none.

Shapes with no census class (constructed, or outside the bare-name census): block-shadow, catch-binding, external-ns-member-alias-trap, inner-param-shadows-alias, inner-shadows-import, merged-ns-local-alias-unknown, missing-export-js-suffix-trap, named-js-suffix-dts-only, named-js-suffix-literal-and-ts, named-js-suffix-tsx-target, named-jsx-suffix-tsx-target, ns-member-alias-js-suffix, ns-qualified, ns-qualified-js-suffix, ns-qualified-jsx, object-literal-member-call, object-shorthand-member-call, props-member-call, static-property-call, static-property-jsx, super-extends-member, super-extends-member-named-super.
