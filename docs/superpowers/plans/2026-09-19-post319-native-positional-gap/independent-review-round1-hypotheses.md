# Independent planning review round 1 — hypotheses

## H1 — raw parameter extraction contract

- Hypothesis: every admitted function kind/language has one unambiguous native syntax seam for top-level parameter positions, and the proposed `parameters`/singular `parameter` rule preserves wrappers without recursively flattening patterns.
- Alternative: tree-sitter field/container shapes differ across JS/TS/TSX, so a generic named-child rule can count punctuation-free but non-parameter nodes or omit bare-arrow parameters.
- Falsifier: current grammar/API source provides an exact cross-language helper or an already tested uniform shape matching the proposed rule.
- Probe: inspect the current native parameter and occurrence seams plus the pinned grammar field handling; do not parse the public 12 sites.
- Result: supported for the bounded JS/TS/TSX kinds. `ParsedFile::find_parameters_node` exposes the direct `parameters` field, while the proposed observer separately handles singular `parameter`; the plan preserves each named top-level child and records wrappers instead of flattening them. No finding.

## H2 — launcher-to-worker request identity

- Hypothesis: the plan does not fully specify the authenticated retained-source request sent to the native worker, leaving file/source/site association and protocol validation ambiguous.
- Alternative: another frozen artifact defines the complete request schema and worker validation contract by reference.
- Falsifier: SPEC or its immutable referenced files define ordered fields, identity checks, selector association, and malformed-protocol refusal completely enough for independent implementation and tests.
- Probe: search all frozen planning artifacts for request/stdin/protocol definitions and compare them with the complete output contract.
- Result: hypothesis supported. The artifacts specify that one request carries authenticated retained strings and list worker failure controls, but they do not define the request schema, file/source/selector association, ordering, or exact protocol validation. This is a planning SMELL because independently conforming implementations can disagree before native parsing.

## H3 — exact 12-site projection custody

- Hypothesis: SITE-MANIFEST is an exact deterministic projection of the authenticated PR319 packet and parent manifest, with complete selected-member custody and no optional/rest/later-initializer admission.
- Alternative: selection prose and stored rows diverge, or member aggregates/hashes do not bind the complete selected closure.
- Falsifier: recomputation from the saved packet/parent manifest differs in any selector, ordinal array, member identity, count, or byte total.
- Probe 1: independent projection produced the same 12 sites, 11 members and 37,040-byte total, but byte-for-byte structural comparison returned false.
- Intermediate result: aggregate custody is supported, but exact projection equality is not yet established. A likely alternative is a field-name or ordering mismatch in the review script rather than a selector difference.
- Discriminator before probe 2: compare site arrays separately, then member arrays field-by-field. If sites differ, selection custody is falsified; if only a member field differs, inspect the parent-manifest schema mapping before assigning a finding.
- Probe 2 result: supported. The mismatch was in the review script: it read `script_kind` from the parent manifest even though the parent field is `extension`. After the explicit extension-to-script-kind mapping, the independently projected sites and members are structurally identical: 12 sites, 11 members, 37,040 bytes. The first probe was invalid for exact-equality evidence and is retained as such; no artifact finding.

## H4 — decision predicate evidence boundary

- Hypothesis: the proposed next-action predicate uses only native occurrence/slot observations and cannot be mistaken for entry/callee evidence; missing/null/ambiguous/recovered controls prevent promotion.
- Alternative: function-name metadata or compiler-supplied ordinals can cause a positive disposition without an independently observed native later binding gap.
- Falsifier: the predicate explicitly requires a clean uniquely matched named native function, a native binding uniquely contained in a selected later-required raw parameter, and absence of a native slot for that same ordinal, while keeping the outcome next-proof only.
- Probe: trace every predicate input to a specified emitted field and its negative controls.
- Result: falsified in two specified synthetic states. First, JavaScript `function take({x}, x){return x;}` yields a clean named candidate, `slots:null`, and binding `x` at later ordinal1; treating null as an empty assigned-slot set promotes it. Second, `function take([x], later){return later;}` with synthetic selection evidence claiming object ordinal0 yields `slots:[]` plus later binding ordinal1; the predicate never authenticates the claimed object pattern against raw native parameters. These are separate concrete wrong next-action outputs.
