**Current status:** Historical receipts below. Step1b is planner-complete; the current union census/measurements, gates, controller command and limits are in REPORT-step1.md, MEASUREMENTS-step1.json and PROBE-LOG-step1b.md. The old readiness STOP is superseded by the controller ruling and serial PASS.

# E5 full census — base 40ad8390 versus uncommitted cache115 prototype

All counts are JS/TS/TSX Exact `free_single` targets. Every site is accounted for in three independent axes. W = WRONG, C = CORRECT, U = UNDECIDED, I = INADMISSIBLE. Unlisted combinations are zero. Native parse/binding failures retain I; producer syntax still supplies the independent callee-form axis. Raw and after-override verdicts are identical (zero overrides).

585 roots requested: X, T, and all 583 SecBench roots. 581 admitted: X, T, SB579. Four base producer exclusions are paired on head; they are not zero-yield roots. Full wall-time receipts are in the evidence directory.

## X

| Callee form | Own-file binding | Base W | C | U | I | Head W | C | U | I |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| function_declaration | local_declaration | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| variable_direct_function | local_declaration | 15 | 0 | 11 | 0 | 15 | 0 | 11 | 0 |
| variable_direct_function | none | 8 | 0 | 23 | 37 | 8 | 0 | 23 | 36 |
| variable_wrapper_call | local_declaration | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |

## T

| Callee form | Own-file binding | Base W | C | U | I | Head W | C | U | I |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| function_declaration | local_declaration | 131 | 0 | 0 | 0 | 131 | 0 | 0 | 0 |
| function_declaration | none | 49 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| member_assignment | none | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| object_pair_value | known_global | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| object_pair_value | local_declaration | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| variable_direct_function | local_declaration | 16 | 0 | 0 | 0 | 16 | 0 | 0 | 0 |

## SB

| Callee form | Own-file binding | Base W | C | U | I | Head W | C | U | I |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| class_member | local_declaration | 0 | 0 | 0 | 13 | 0 | 0 | 0 | 13 |
| class_member | require_destructuring | 0 | 0 | 0 | 10 | 0 | 0 | 0 | 10 |
| function_declaration | known_global | 0 | 4 | 5 | 0 | 0 | 4 | 5 | 0 |
| function_declaration | local_declaration | 198 | 0 | 2 | 0 | 198 | 0 | 2 | 0 |
| function_declaration | none | 85 | 6 | 5 | 471 | 85 | 6 | 5 | 471 |
| member_assignment | known_global | 95 | 0 | 0 | 3 | 0 | 0 | 0 | 0 |
| member_assignment | local_declaration | 305 | 10 | 0 | 0 | 305 | 10 | 0 | 0 |
| member_assignment | none | 0 | 0 | 0 | 151 | 0 | 0 | 0 | 28 |
| object_pair_value | known_global | 31 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| object_pair_value | local_declaration | 12 | 0 | 13 | 2 | 12 | 0 | 13 | 0 |
| object_pair_value | none | 4 | 0 | 0 | 14 | 4 | 0 | 0 | 14 |
| object_shorthand_method | known_global | 0 | 0 | 0 | 173 | 0 | 0 | 0 | 0 |
| object_shorthand_method | local_declaration | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| object_shorthand_method | none | 0 | 0 | 0 | 34 | 0 | 0 | 0 | 34 |
| other:foreign:C | none | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 5 |
| other:foreign:Java | local_declaration | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 3 |
| other:identifier_assignment | local_declaration | 3 | 0 | 0 | 4 | 3 | 0 | 0 | 4 |
| other:identifier_assignment | none | 0 | 0 | 0 | 9 | 0 | 0 | 0 | 8 |
| other:named_function_expression | local_declaration | 80 | 0 | 2 | 0 | 80 | 0 | 2 | 0 |
| variable_direct_function | local_declaration | 650 | 0 | 0 | 0 | 650 | 0 | 0 | 0 |
| variable_direct_function | none | 94 | 0 | 0 | 144 | 94 | 0 | 0 | 123 |
| variable_wrapper_call | local_declaration | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| variable_wrapper_call | none | 0 | 0 | 0 | 21 | 0 | 0 | 0 | 21 |

## WRONG samples

WRONG-SAMPLES.jsonl includes up to ten deterministic source-bound samples in **every** nonzero WRONG cell (all when fewer than ten). Each retains the source site span, resolved FunctionId and physical callable bytes when available, native checker declarations, own-file binding evidence, primary reason, and candidate predicates. No private corpus is included.
