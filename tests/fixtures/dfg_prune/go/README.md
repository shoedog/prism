# Go lexical-prune matrix

Rows 00–18 implement the slice-1 U/D contract; row 17 has separate select and type-switch inputs.
These sources are parsed as static fixtures, never executed as Go programs.

Round 2 adds row 19 (a loop-carried definition inside D's containing child, with a preceding
dominator), row 20 (fallthrough with and without a write in the destination case), and row 21
(field-base reassignment). Row 21 pins current fixed-storage path comparison: `p.f = 1` does
not reach the final read in the graph, while `p.f = 2` does. At runtime the pointers can refer
to different objects; that pointer identity is out of model under design §2.2. This is a
documented boundary, not a claim that the graph models that runtime flow.

`obligations.jsonl` identifies independently marked pairs. Changed U/D obligations are RED with
the original producer; preservation obligations retain the base pair. In particular row 04 keeps
its backward pair until O. Row 12 protects the outer binding's final read, while its
other binding/order expectations await S/O. A keep
obligation does not prevent U/D from removing unrelated pairs in the same source.

`expected.jsonl` enumerates the complete original pair set, with a keep/drop decision for every
pair. It was built from the original producer dump and hand-audited write-line / dominance-window
tables before running the changed producer. The integration test compares the complete surviving
set; the unit differential independently checks membership and survivor labels against the frozen
post-R main producer at cc78c2b1. Row 01 is exactly 79 to 14 pairs; row 03 adds a goto-condition read (80 to 15).

Row 15 uses `s *S; ptr := s` to construct the pointer alias twin supported by the existing
plain-identifier alias extractor. `ptr := &s` does not construct that twin today. Alias extraction
is unchanged by this slice.
