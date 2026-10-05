# S2 R4 dispatch — PKG must merge first

**STOP: R4 yield 0/0/0/0; X shortfall 132 from132 exceeds10; selection NONE.**

S2 implementation depends on lane PKG merging first. Intended dispatch: committed S2 proto61641bdb plus R4-src.patch, rebased onto PKG's merged main. R4-src.patch's exact parent is61641bdb + PKG03fa9c29, retained in combined-base.tar.gz. Do not stack old cumulative R2/R3/R3b patches;61641bdb already contains them. S2-only projection R4-s2-only.patch applies to61641bdb and excludes PKG implementation hunks/files; it still calls the PKG API and therefore is not a standalone compilable product. Recompose on the merged PKG tree and rerun all source-bound checks. R4-docs.patch applies toaa55a532.

This STOP archive is not a selected implementation or adoption authorization. Do not dispatch pending full gates/F/review/adoption until owner resolves the yieldSTOP. No policy choice was made. Source747 inputs and rebuilt canonical tools bind to this packet's manifest. Preserve immutable main4e592daa tools; target/release alone is not the controller binding.

Only ProvenUnresolved permits direct writer exclusion. Unsupported remains unavailable; resolved-forward/depth failures remain in model. Never bypass PKG compiler-option authority to recover yield. Read SPEC,REPAIR-R4,MEASUREMENTS,VERIFICATION and gap-revoking-writers.md/json. No Git writes, network/install, private F access, source restart, new T lane or auto-merge by repair worker. Tier-A quick skipped;full multi-corpus human-triggered. Git commits and any PKG round2 delta remain controller responsibilities.
