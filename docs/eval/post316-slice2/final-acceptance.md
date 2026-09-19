# Slice 2 final independent acceptance

**APPROVE — WRONG: 0 / SMELL: 1. Core review and final evidence reconciliation are complete, with the validation limits below.**

Source/test checkpoint: `7fc89c98bd3eaae99dc9dc959097b942ac96fe8c`, tree `dff38f065792cb6d7c20bb32c146b438bde8fa89`. Production matches reviewed `498fb6f68acaa7b87a5551b840967a9c28b50011`; test SHA256 `0a36d430d9f298f04f1eea3ba5582ce081b81feb076ca66b5c3fcb372f743ff3`. After final Rust receipt, a fresh diff of all src/Cargo/build paths against7fc89c98 was empty. Remaining custody edits are documentation only.

Two-round review converged: all G1–G4 closed. The sole nonblocking SMELL is repeated exact-bucket validation cost; no unacceptable performance outcome was shown. Round2 independently ran final16/16pass and identical final G1/G2/G3 on unchanged base: G2preservationpass, G1+G3behavioralRED with complete dialect deficits. The occurrence recovery and bounded downstream/return connectivity do not repair caller RD identity or promote its NameOnly labels. Legacy APIs remain line-based.

## Final verification reconciliation

| Gate | Verified result |
|---|---|
| Default Rust | 4,534 pass / 0 fail / 1 ignored |
| MCP Rust | 4,727 pass / 0 fail / 1 ignored |
| Widest MCP + detached-owner-audit | 4,750 pass / 0 fail / 1 ignored |
| Widest examples | 32 pass / 0 fail / 0 ignored |
| Clippy all-targets + MCP | Completed; 254 warning-prefixed lines including summary lines. None names the changed occurrence test file; no base attribution of all warnings is claimed. |
| fmt/diff checks | Independently rerun, both pass |
| Grammar | Shipped parser and baseline byte reproduction pass |
| Authority | 40 results / 0 failures |
| Node | 786 selected: 785 pass / 0 fail / 1 expected skip |
| Python | 896 eval tests + 44 adoption unit = 940 pass |
| Tier-A matrix | 159/159 ok |
| Tier-A quick | Report retained, **INVALID**; not a clean accuracy gate |

Rust counts were independently summed from every result block in each final log; all six supplied log/receipt hashes match. The only ignored Rust test is `resolution_test::slice_elem_variant_reserved`, reserved by its existing contract. Final receipt SHA256 `1ece02b4ac35fd7f1b9b6b42e5d2bf96e992ee44fa42f2a71475be26fc9a91d8` records the reviewed source. NonRust logs and hashes were separately reconciled in round2; frozen binaries are production-equivalent to the final test-only hardening.

## Explicit limits

- Quick is INVALID for corpus pin drift (`498fb6f68aca` versus pinned `20c8490591a3`) and C-method4/6 successful oracle probes. SUT error rate0 does not make the report valid. Its flip-candidate is not an attributed regression or rebaseline authority. No quick retry/rebaseline occurred.
- Three historical imported-props real-sites source-custody tests were excluded because their archive/provenance inputs are absent. One grammar-archive branch is an expected Node skip.
- Live `adoption/tests/test_prism_adoption.py` and the human-triggered full Tier-A multicorpus run were excluded.
- No pinned real JS/TS/TSX corpus was available among the four prepared corpus roots. G4's fixed three-dialect fixtures and Rust Tokio control establish bounded costs/preservation only. Tokio unique graph facts match; four identical edge multiplicities2→1 account for its edge-count delta. Index microtiming measures cfg(test) graph reconstruction, not production per-argument lookup throughput.
- This local acceptance authorizes no publication, merge, live adoption, production effect or broader corpus run.

The source-manifest checkpoint-tree typo was routed and corrected during documentation closeout; authoritative checkpoint tree is `7437d02af2f86994b9ed43fcaa3b813a3fa63fb5`. No third source-review round was consumed. Source review artifacts and logs remain under this review directory; worker/controller may copy them byte-exact into durable lane custody.
