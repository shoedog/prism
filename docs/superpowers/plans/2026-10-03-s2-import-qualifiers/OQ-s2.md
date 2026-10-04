# S2 decisions and controller follow-through

## Owner decision (2026-10-03)

**S2-O6: “Ship X gain, fail-closed.”** A qualifier class/object stays base when value identity escapes or member-write closure is unproved. Aliases, arguments/returns/stored/spread values, member writes/computed access, reflective helpers, renamed exports and default export with other uses refuse. The rule applies to every visible defining/importing file. A conservative lexical over-approximation is allowed; implement a whitelist of permitted uses.

S2-W1 was a demonstrated WRONG (100/100), reproduced on the frozen old prototype in the same environment and both grammars. Static oracle agreement was insufficient: `Alias===C` and replacement result 1 prove the known member write. The existing artifact now implements the owner's design and its RED/GREEN controls; see MEASUREMENTS/PROBES. The previous dispatch parking for an unanswered S2-O6 is superseded. Adoption still requires controller gates and review.

| ID | Current answer / remaining work |
|---|---|
| S2-O1 | Controller interim: adopt captured separate S2 relative envelope. Pending owner confirmation for adoption. Lane-P resolver unchanged. |
| S2-O2 | Controller interim: keep two hops per leg, up to four composed. Pending owner confirmation for adoption. |
| S2-O3 | Controller interim: keep T at base; a new ownership increment needs separate design/authorization. |
| S2-O4 | Controller measured F base aggregates: 974 low /32 callable-low; 26 instance methods and six default function members; zero static-class rows; 1,271 UNJOINABLE. Planner did not reverify or read F. Run updated wrapper for the head correctness gate. |
| S2-O5 | Controller interim: retain positional-proof mutation survivor as a disclosed coverage SMELL unless a realistic negative is found. Authoritative registry gate remains required. |

No instance-method, call-result, CommonJS, positive heap-alias, exclusion-inference or ambient-input expansion is included. No Git writes, F read, review dispatch, publication or merge is authorized for this planner.
