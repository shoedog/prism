# Detached constructor implementation plan

Authority: owner approved implementation of PR295's eight predicates, not production
resolution. Base4e88d33b. No installs, closure changes, React expansion or cache changes.

1. Expose the existing bounded compiler worker as an internal factory with an optional
   trusted inspection callback before the final snapshot. Normal CLI packet shape and
   closure policy remain unchanged. Separately version the detached facts envelope;
   never serialize an authority object or add a proven bit to old props_class.
2. In a new compiler pass, establish direct declaration/binder/substitution/property/
   class/member identity and refuse each unsupported grammar/effect predicate. Use
   the same Program/checker and original UTF anchors; no printed-name type authority.
3. Private Rust acquisition pins worker assets, runs bounded compiler work twice,
   compares reproduced facts and requires complete fixed closure. Independently own
   the complete Prism JS/TS input map and require exact canonical path/hash agreement
   with project Program sources. Reject unknown/omitted/extra or mismatched inputs.
4. Construct a private non-Deserialize proof only after mapping all anchors and the
   unique executable member into the owned Prism parser/graph facts. Hold the owning
   Arc epoch in the proof; identity-checked readout cannot consume another epoch's proof.
   No public proof setters, graph installation, persistent edges or resolver consumer.
5. Native tests cover input/anchor/mapping barriers. An explicit detached-owner-audit
   test feature runs real pinned compiler acquisition and two-genuine-epoch substitution
   tests; ordinary Prism gains no default compiler dependency. Run that feature locally
   alongside full default/MCP and Node compiler suites; report it separately from CI.

RED accounting: base has no constructor API. A minimal fail-closed scaffold
and executable failing test supplied the pre-implementation control (base plus test seam,
not a claim that the constructor existed on main). Existing public edge absence is a
compatibility control here because detached construction must not change that edge.
The captured failure is the genuine positive acquisition; the final expanded tests
cover its TS/TSX and epoch variants. The original scaffold output is retained but its
source was overwritten, not separately archived. Passing refusal controls remain
controls, not fabricated RED. The later omitted-index regression has its own failing
output on the actual pre-fix implementation. Two review rounds completed; one bounded
WRONG fixed in round 1, no further demonstrated wrong result in round 2.
