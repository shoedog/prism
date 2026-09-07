# Handoff — public dependency acquisition measurement

Publication: measurement4597f6b pushed in https://github.com/shoedog/prism/pull/266
against main; this docs-only closeout follows.

**Written:** 2026-09-06 · **By:** /root · **Provider:** codex
**Workspace:** /private/tmp/prism-acquire-w2FtSq/worktree · docs/callable-acquisition-measurement · **Measured state:** `[MEASURED]` base3001ca58; single install complete, admission declined on measured size/count/symlink barriers; no observer replay.
**Predecessor:** PR265, confirmed merged at3001ca58c04aa323a686968bbfb57eb7b927d730.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only, no agents — RESOLVED. (b) Original dirty Prism and clean application
preserved; separate task source/tools/cache/evidence — RESOLVED. (c) No running
install — RESOLVED. (d) Owner explicitly approved frozen-lock, script-disabled
public disposable install and pinned tool bootstrap; authority exercised once.
No private acquisition or observer implementation authorized.

## 1. Resume order

1. git -C /private/tmp/prism-acquire-w2FtSq/worktree status --short --branch.
2. Read predecessor acquisition spec and this ledger; never repeat an install
   attempt without checking whether install-result.json already exists.

**STOP conditions:** one install,15min,5GiB task data,3GiB free floor; unsupported
registry/credentials/redirect trust; no source/config overrides, scripts or cleanup.
Two SELF-PASS rounds, NOT INDEPENDENT. No observer implementation authorized.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Source identity | done | `[MEASURED]` clean public0642e72c; package/config/lock hashes match plan; git archive copied tracked bytes to source/ |
| Tool bootstrap | done | `[MEASURED]` Node24.15.0; Yarn1.22.22 archive SHA512/SHA1 match TLS registry metadata; public/yarn-registry.json; archive/tools retained |
| Config/trust and full source verification | done | `[MEASURED]` source-before.json tracked-byte comparison; pinned source confirms no-default-rc and lifecycle flag; config-final.log has scripts disabled and isolated non-secret settings |
| Install | done | `[MEASURED]` install-result.json:24.436s,success; log confirms ignored scripts;1445 allowed tarball requests,0 guard denials; task data4044866771 bytes at exit |
| Source preservation | done | `[MEASURED]` source-before.json equals source-after.json; original clean and copied tracked bytes/executable bits unchanged |
| Compatibility | done | `[MEASURED]` installed-inventory.json and repeat match:830105343 source bytes,59799 files,9059 dirs,249 links; compiler23568832 bytes,125 files,14 dirs; admission declined, no replay |
| Archive integrity / review | done | `[MEASURED]` archive-verification-corrected.json:1445/1445 archives,1445 records,1442 URLs,0 missing/failures; two self-review rounds, no extension |
| Publication | done | `[MEASURED]` measurement4597f6b pushed; PR266 open against main |

## 3. Corrections to standing documents and memory

PR265 is merged and installation is now explicitly authorized. Previous approval-
pending statements are historical. No memory edits authorized. Predecessor
notice/roadmap and original pointer are reconciled with measured outcome.

## 4. Open work

No acquisition/publication work remains; owner review/merge is next.
Public-only evidence: /private/tmp/prism-acquire-w2FtSq/public-evidence.tgz with
adjacent checksum sidecar. Installed source/cache/tools remain intact separately.
Next requires approval for bounded acquisition design covering measured large-tree
budgets and canonical in-root link identity; do not automatically implement it.

## 5. Invariants and traps — do not do these

- Do not invoke manager in original source or let automatic bootstrap choose versions.
- Script-disabled install is not a build-complete application or Program proof.
- Never normalize links/prune packages/raise budgets to force admission.
- Preserve partial failed acquisition and logs; one attempt is the cap.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task/evidence root | /private/tmp/prism-acquire-w2FtSq |
| Disposable application | /private/tmp/prism-acquire-w2FtSq/source |
| Node | /Users/wesleyjinks/.local/share/mise/installs/node/24.15.0/bin/node |
| Node SHA256 | 3200fbd9f7fd4410426dd541e10d1ab829d3472f270d743c7fabd1696c03fe32 |
| Yarn archive SHA256 | c17d3797fb9a9115bf375e31bfd30058cac6bc9c3b8807a3d8cb2094794b51ca |
| Public source | 0642e72cfa2d9a71198200e52f37399384610ee3 |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED — two rounds, no extension · claim: "single isolated acquisition is measured without changing source or observer authority" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: admission.json, source-before/after.json, archive-verification-corrected.json and same-date readout; integrity/source assertions, no observer replay, probe setup error explicitly excluded.

**Questions the owner owes an answer to:** None within approved scope.
