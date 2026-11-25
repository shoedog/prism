# Stage D row requirements

The 326 rows follow fixture order. Each requirement identifies an individual write target in the recorded file. The saved Node 24.15.0
observations supply the truth for the ordinary roots exercised by each call, including calls that construct
their own root. [Fixture metadata](../../../../../tests/proto_writes_matrix/rows.jsonl) ·
[Recorded execution](../../../../../tests/proto_writes_matrix/truth/truth.jsonl) ·
[Truth contract](DESIGN.md#7-truth-by-execution-stages-t-and-e)

The sink column is a requirement with the listed attacker names supplied as reached inputs. A witness is a
selector base in that row’s `attacker_names`. A dash in that column means the intersection is empty. A
sufficient guard silences even a matching witness. The seeded integration cases specify actual propagation,
including `sk11` from a source-only seed. Product measurements begin in Stage I.
[Seeded sink cases](../../../../../tests/proto_writes_matrix/sink_rows.md)

| Recorded truth and scope | Fires | Silent | Total |
|---|---:|---:|---:|
| In scope, `pollutes` | 138 | 16 | 154 |
| In scope, `safe` | 47 | 92 | 139 |
| Outside lane scope, `safe` with a scope note | 22 | 11 | 33 |
| **All rows** | **207** | **119** | **326** |

Of the 119 silent rows, 96 require no fact, 14 have a sufficient guard and nine have a fact with an
insufficient guard and no matching selector base. All 33 scope rows have individual decisions. The named
selector `req` fires in `t3-051`. S2 independently tests its selector source range. The receiver-only
control `t3-017` stays silent.
[Losses and scope](DESIGN.md#11-accepted-losses-and-scope)

Match the designated AST target by file and half-open UTF-8 byte span before deduplicating findings by line.
For `pw04` line 2 the outer reduce-result member occupies **[31, 114)** and must itself produce
`path_walk`, guard `none` and a matching selector. The inner callback store at [67, 71) gets its own
decision. The outer assertion must fail if only the inner fact survives. Across all 326 designated lines
this is the only pair of qualifying targets on one line. The inner replacement in `t2-110` is a one-level
store. For chained assignments, loop bindings and destructuring, bind the asserted member target
individually, including the second pattern target on its designated line in `t2-127`.

| Row ID | Truth | Required fact class | Required guard | Required sink | Selector witness | Accepted loss or scope |
|---|---|---|---|---|---|---|
| pw01 | pollutes | path_walk | none | fires | keys | — |
| pw02 | pollutes | path_walk | none | fires | parts, k | — |
| pw03 | pollutes | path_walk | none | fires | p | — |
| pw04 | pollutes | path_walk | none | fires | keys, k | — |
| pw05 | pollutes | recursive_merge | none | fires | keys | — |
| pw06 | pollutes | path_walk | none | fires | k | — |
| pw07 | pollutes | path_walk | none | fires | k | — |
| pw08 | pollutes | path_walk | none | fires | k | — |
| pw09 | safe | path_walk | none | silent | — | — |
| pw10 | pollutes | path_walk | none | fires | s | — |
| pw11 | pollutes | path_walk | none | fires | s | — |
| pw12a | pollutes | path_walk | none | fires | ch | — |
| pw12b | safe | path_walk | none | fires | ch | R-ITER |
| nk01 | pollutes | nested_keys | none | fires | k1 | — |
| nk02 | safe | none | none | silent | — | — |
| nk03 | safe | none | none | silent | — | — |
| nk04 | safe | nested_keys | none | silent | — | — |
| nk05 | safe | none | none | silent | — | — |
| nk06 | safe | none | none | silent | — | — |
| nk07 | safe | none | none | silent | — | — |
| nk08a | safe | nested_keys | none | fires | i | R-ITER |
| nk08b | safe | nested_keys | none | fires | i | R-ITER |
| nk08c | safe | nested_keys | none | fires | x | R-ITER |
| nk09a | safe | none | none | silent | — | — |
| nk09b | safe | none | none | silent | — | — |
| nk09c | safe | none | none | silent | — | — |
| nk10 | safe | none | none | silent | — | — |
| nk11 | pollutes | nested_keys | none | fires | k | — |
| nk12 | safe | none | none | silent | — | — |
| nk13 | safe | none | none | silent | — | — |
| nk14 | pollutes | nested_keys | none | fires | y | — |
| nk15 | safe | none | none | silent | — | — |
| rm01 | pollutes | recursive_merge | none | fires | key | — |
| rm02 | pollutes | recursive_merge | none | fires | key | — |
| rm03 | pollutes | recursive_merge | none | fires | key | — |
| rm04 | safe | none | none | silent | — | — |
| rm05 | safe | none | none | silent | — | — |
| np01 | safe | none | none | silent | — | — |
| np02 | safe | none | none | silent | — | — |
| np03 | safe | none | none | silent | — | — |
| g01 | safe | recursive_merge | sufficient | silent | key | — |
| g02 | pollutes | recursive_merge | partial | fires | key | — |
| g03 | pollutes | recursive_merge | none | fires | key | — |
| g04 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g05 | pollutes | path_walk | none | fires | k | — |
| g06 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g07 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g08 | pollutes | path_walk | none | fires | k | — |
| g09 | pollutes | path_walk | none | fires | k | — |
| g10 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g11 | pollutes | path_walk | partial | fires | k | — |
| g12 | pollutes | path_walk | partial | fires | k | — |
| g13 | pollutes | path_walk | partial | fires | k | — |
| g14 | safe | path_walk | partial | fires | k | R-COERCE |
| g15 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g16 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g17 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g18 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g19 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g20 | pollutes | path_walk | partial | fires | k | — |
| g21 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g22 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g23 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g24 | safe | recursive_merge | partial | fires | key | R-CHILD |
| g25 | safe | recursive_merge | partial | fires | key | R-CHILD |
| g26 | safe | path_walk | sufficient | silent | k | — |
| g27 | pollutes | none | none | silent | — | M-CALL-WRITE |
| g28 | safe | path_walk | sufficient | silent | name | — |
| g29 | pollutes | path_walk | partial | fires | k | — |
| g30 | pollutes | path_walk | none | fires | k | — |
| g31 | pollutes | path_walk | none | fires | k | — |
| g32 | safe | path_walk | partial | fires | k | R-KEY-PROOF |
| g33 | pollutes | nested_keys | partial | fires | k1 | — |
| g34a | pollutes | path_walk | none | fires | k | — |
| g34b | pollutes | path_walk | none | fires | k | — |
| hw01 | pollutes | none | none | silent | — | M-HELPER |
| at01 | pollutes | path_walk | none | fires | id | — |
| at02 | pollutes | path_walk | none | fires | section | — |
| rp01 | pollutes | recursive_merge | none | fires | key | — |
| rp02 | pollutes | recursive_merge | none | fires | key | — |
| ra01 | pollutes | recursive_merge | none | fires | name | — |
| ra02 | pollutes | recursive_merge | none | fires | key | — |
| rr01 | pollutes | recursive_merge | none | fires | key | — |
| ic01 | pollutes | path_walk | none | fires | k | — |
| ic02 | pollutes | path_walk | none | fires | k | — |
| ic03 | pollutes | path_walk | none | fires | k | — |
| fc01 | pollutes | none | none | silent | — | M-STORAGE |
| dc01 | pollutes | path_walk | none | fires | k | — |
| im01 | pollutes | none | none | silent | — | M-STORAGE |
| oa01 | pollutes | none | none | silent | — | M-CALL-WRITE |
| rs01 | pollutes | none | none | silent | — | M-CALL-WRITE |
| rs02 | pollutes | none | none | silent | — | M-CALL-WRITE |
| t2-001 | pollutes | path_walk | none | fires | key | — |
| t2-002 | pollutes | path_walk | none | fires | key | — |
| t2-003 | pollutes | path_walk | none | fires | key | — |
| t2-004 | pollutes | path_walk | none | fires | key | — |
| t2-005 | pollutes | nested_keys | none | fires | selector | — |
| t2-006 | pollutes | nested_keys | none | fires | selector | — |
| t2-007 | pollutes | path_walk | none | fires | key | — |
| t2-008 | pollutes | none | none | silent | — | M-HELPER |
| t2-009 | pollutes | recursive_merge | none | fires | keys | — |
| t2-010 | pollutes | nested_keys | none | fires | selector | — |
| t2-011 | safe | none | none | silent | — | — |
| t2-012 | safe | none | none | silent | — | — |
| t2-013 | pollutes | nested_keys | none | fires | second | — |
| t2-014 | safe | nested_keys | none | fires | selector | O-APP |
| t2-015 | pollutes | nested_keys | none | fires | selector | — |
| t2-016 | safe | none | none | silent | — | — |
| t2-017 | pollutes | nested_keys | none | fires | index | — |
| t2-018 | safe | none | none | silent | — | — |
| t2-019 | pollutes | nested_keys | none | fires | selector | — |
| t2-020 | pollutes | nested_keys | none | fires | selector | — |
| t2-021 | pollutes | nested_keys | none | fires | selector | — |
| t2-022 | pollutes | nested_keys | none | fires | selector | — |
| t2-023 | pollutes | none | none | silent | — | M-CALL-WRITE |
| t2-024 | pollutes | none | none | silent | — | M-CALL-WRITE |
| t2-025 | safe | path_walk | partial | fires | key | R-KEY-PROOF |
| t2-026 | pollutes | path_walk | partial | fires | first, key | — |
| t2-027 | pollutes | path_walk | none | fires | key | — |
| t2-028 | pollutes | path_walk | partial | fires | key | — |
| t2-029 | safe | path_walk | partial | fires | key | R-KEY-PROOF |
| t2-030 | pollutes | nested_keys | none | fires | selector | — |
| t2-031 | pollutes | none | none | silent | — | M-LOOKUP |
| t2-032 | pollutes | nested_keys | none | fires | selector | — |
| t2-033 | pollutes | nested_keys | none | fires | key | — |
| t2-034 | safe | none | none | silent | — | — |
| t2-035 | pollutes | nested_keys | none | fires | selector | — |
| t2-036 | pollutes | nested_keys | none | fires | selector | — |
| t2-037 | pollutes | path_walk | none | fires | keys | — |
| t2-038 | pollutes | recursive_merge | none | fires | keys | — |
| t2-039 | pollutes | path_walk | none | fires | key | — |
| t2-040 | pollutes | path_walk | none | fires | key | — |
| t2-041 | pollutes | path_walk | none | fires | selector | — |
| t2-042 | pollutes | nested_keys | none | fires | selector | — |
| t2-043 | pollutes | none | none | silent | — | M-LOOKUP |
| t2-044 | pollutes | nested_keys | none | fires | selector | — |
| t2-045 | safe | nested_keys | none | fires | index | R-COERCE |
| t2-046 | pollutes | nested_keys | none | fires | selector | — |
| t2-047 | pollutes | nested_keys | none | fires | selector | — |
| t2-048 | safe | path_walk | sufficient | silent | key | — |
| t2-049 | pollutes | nested_keys | partial | fires | selector | — |
| t2-050 | pollutes | recursive_merge | partial | fires | path | — |
| t2-051 | pollutes | path_walk | none | fires | keys | — |
| t2-052 | pollutes | path_walk | none | fires | key | — |
| t2-053 | pollutes | path_walk | none | fires | key | — |
| t2-054 | pollutes | none | none | silent | — | M-LOOKUP |
| t2-055 | pollutes | nested_keys | none | fires | selector | — |
| t2-056 | pollutes | nested_keys | none | fires | key | — |
| t2-057 | safe | none | none | silent | — | — |
| t2-058 | pollutes | nested_keys | none | fires | selector | — |
| t2-059 | pollutes | nested_keys | none | fires | selector | — |
| t2-060 | pollutes | nested_keys | none | fires | selector | — |
| t2-061 | pollutes | nested_keys | none | fires | selector | — |
| t2-062 | pollutes | nested_keys | none | fires | selector | — |
| t2-063 | pollutes | nested_keys | partial | fires | first, second | — |
| t2-064 | pollutes | path_walk | none | fires | key | — |
| t2-065 | pollutes | recursive_merge | none | fires | key | — |
| t2-066 | pollutes | path_walk | none | fires | key | — |
| t2-067 | pollutes | nested_keys | none | fires | name | — |
| t2-068 | safe | none | none | silent | — | — |
| t2-069 | pollutes | nested_keys | none | fires | selector | — |
| t2-070 | pollutes | none | none | silent | — | M-CALL-WRITE |
| t2-071 | pollutes | none | none | silent | — | M-CALL-WRITE |
| t2-072 | safe | path_walk | partial | fires | key | R-KEY-PROOF |
| t2-073 | safe | path_walk | sufficient | silent | key | — |
| t2-074 | pollutes | nested_keys | none | fires | selector | — |
| t2-075 | pollutes | path_walk | partial | fires | key | — |
| t2-076 | safe | none | none | silent | — | — |
| t2-077 | pollutes | nested_keys | none | fires | section | — |
| t2-078 | safe | none | none | silent | — | — |
| t2-079 | safe | none | none | silent | — | — |
| t2-080 | pollutes | nested_keys | none | fires | section | — |
| t2-081 | pollutes | path_walk | none | fires | key | — |
| t2-082 | pollutes | path_walk | none | fires | key | — |
| t2-083 | safe | path_walk | sufficient | silent | key | — |
| t2-084 | safe | none | none | silent | — | — |
| t2-085 | pollutes | path_walk | none | fires | key | — |
| t2-086 | pollutes | none | none | silent | — | M-STORAGE |
| t2-087 | pollutes | recursive_merge | none | fires | key | — |
| t2-088 | pollutes | path_walk | none | fires | key | — |
| t2-089 | pollutes | path_walk | none | fires | key | — |
| t2-090 | pollutes | nested_keys | none | fires | key | — |
| t2-091 | safe | none | none | silent | — | — |
| t2-092 | safe | none | none | silent | — | — |
| t2-093 | safe | none | none | silent | — | — |
| t2-094 | pollutes | nested_keys | none | fires | section | — |
| t2-095 | pollutes | nested_keys | none | fires | section | — |
| t2-096 | safe | nested_keys | none | fires | section | O-STRING |
| t2-097 | safe | none | none | silent | — | — |
| t2-098 | safe | none | none | silent | — | — |
| t2-099 | safe | none | none | silent | — | — |
| t2-100 | pollutes | nested_keys | none | fires | section | — |
| t2-101 | pollutes | path_walk | none | fires | section | — |
| t2-102 | safe | nested_keys | none | fires | section | O-APP |
| t2-103 | safe | nested_keys | none | fires | section | O-APP |
| t2-104 | safe | nested_keys | none | fires | section | O-APP |
| t2-105 | pollutes | nested_keys | none | fires | section | — |
| t2-106 | safe | none | none | silent | — | O-APP |
| t2-107 | safe | none | none | silent | — | O-APP |
| t2-108 | safe | none | none | silent | — | — |
| t2-109 | safe | none | none | silent | — | — |
| t2-110 | safe | nested_keys | none | fires | section | O-APP |
| t2-111 | safe | nested_keys | none | fires | section | R-TARGET |
| t2-112 | safe | none | none | silent | — | O-APP |
| t2-113 | safe | none | none | silent | — | — |
| t2-114 | safe | nested_keys | partial | fires | label | O-APP |
| t2-115 | safe | none | none | silent | — | — |
| t2-116 | safe | none | none | silent | — | — |
| t2-117 | safe | none | none | silent | — | — |
| t2-118 | safe | path_walk | sufficient | silent | section | — |
| t2-119 | safe | path_walk | none | fires | section | R-TARGET |
| t2-120 | safe | nested_keys | sufficient | silent | key | — |
| t2-121 | safe | path_walk | sufficient | silent | section | — |
| t2-122 | safe | nested_keys | none | fires | section | O-APP |
| t2-123 | safe | nested_keys | none | fires | section | O-APP |
| t2-124 | safe | path_walk | none | fires | section | R-TARGET |
| t2-125 | safe | none | none | silent | — | — |
| t2-126 | safe | none | none | silent | — | O-APP |
| t2-127 | safe | nested_keys | none | fires | section | R-TARGET |
| t2-128 | safe | none | none | silent | — | O-APP |
| t2-129 | safe | nested_keys | none | fires | section | O-APP |
| t2-130 | safe | nested_keys | none | fires | section | R-TARGET |
| t2-131 | safe | path_walk | partial | fires | key | O-APP |
| t2-132 | safe | nested_keys | partial | fires | key | O-APP |
| t2-133 | safe | nested_keys | none | fires | section | O-APP |
| t2-134 | safe | nested_keys | none | fires | section | O-APP |
| t2-135 | safe | nested_keys | none | fires | section | O-APP |
| t2-136 | safe | nested_keys | none | fires | section | O-APP |
| t2-137 | safe | none | none | silent | — | O-APP |
| t2-138 | safe | none | none | silent | — | O-APP |
| t2-139 | safe | nested_keys | partial | fires | section | O-APP |
| t2-140 | safe | none | none | silent | — | O-APP |
| t2-141 | safe | nested_keys | partial | fires | section | R-KEY-PROOF |
| t2-142 | safe | nested_keys | none | fires | section | O-APP |
| t2-143 | safe | nested_keys | none | fires | section | O-APP |
| t2-144 | safe | path_walk | none | fires | section | O-APP |
| t2-145 | safe | nested_keys | none | fires | section | O-APP |
| t2-146 | safe | nested_keys | sufficient | silent | section | — |
| t2-147 | safe | none | none | silent | — | — |
| t2-148 | safe | none | none | silent | — | O-APP |
| t2-149 | safe | none | none | silent | — | O-APP |
| t2-150 | safe | none | none | silent | — | — |
| t2-151 | safe | path_walk | none | fires | key | R-SCHEMA |
| t2-152 | safe | nested_keys | sufficient | silent | section | — |
| t2-153 | safe | path_walk | sufficient | silent | key | — |
| t2-154 | safe | none | none | silent | — | O-APP |
| t2-155 | safe | none | none | silent | — | — |
| t2-156 | safe | nested_keys | none | fires | section | R-TARGET |
| t2-157 | safe | nested_keys | none | silent | — | — |
| t2-158 | safe | none | none | silent | — | — |
| t2-159 | safe | none | none | silent | — | — |
| t2-160 | safe | none | none | silent | — | — |
| t2-161 | safe | none | none | silent | — | — |
| t2-162 | safe | path_walk | none | fires | selected | O-APP |
| t2-163 | safe | nested_keys | none | silent | — | — |
| t2-164 | safe | none | none | silent | — | — |
| t2-165 | safe | none | none | silent | — | — |
| t2-166 | safe | nested_keys | sufficient | silent | side | — |
| t2-167 | safe | path_walk | partial | fires | parts, key | R-KEY-PROOF |
| t2-168 | safe | path_walk | partial | fires | key | R-KEY-PROOF |
| t2-169 | safe | nested_keys | none | fires | result | R-ASYNC |
| t2-170 | safe | nested_keys | sufficient | silent | section | — |
| t2-171 | safe | none | none | silent | — | — |
| t2-172 | safe | nested_keys | none | fires | edit | R-TARGET |
| t2-173 | safe | none | none | silent | — | — |
| t2-174 | safe | nested_keys | none | fires | position | R-ITER |
| t2-175 | safe | nested_keys | none | silent | — | — |
| t2-176 | safe | none | none | silent | — | — |
| t2-177 | safe | none | none | silent | — | — |
| t2-178 | safe | none | none | silent | — | — |
| t3-001 | safe | nested_keys | none | fires | section | R-TARGET |
| t3-002 | safe | recursive_merge | none | fires | key | R-TARGET |
| t3-003 | safe | none | none | silent | — | — |
| t3-004 | safe | none | none | silent | — | — |
| t3-005 | safe | none | none | silent | — | — |
| t3-006 | safe | none | none | silent | — | — |
| t3-007 | safe | nested_keys | none | fires | group | R-TEMPLATE |
| t3-008 | safe | none | none | silent | — | — |
| t3-009 | safe | none | none | silent | — | — |
| t3-010 | safe | none | none | silent | — | — |
| t3-011 | safe | nested_keys | none | fires | name | R-KEY-PROOF |
| t3-012 | safe | nested_keys | none | fires | name | R-KEY-PROOF |
| t3-013 | safe | nested_keys | partial | fires | section | R-KEY-PROOF |
| t3-014 | safe | path_walk | none | silent | — | — |
| t3-015 | safe | none | none | silent | — | — |
| t3-016 | safe | path_walk | none | silent | — | — |
| t3-017 | safe | path_walk | none | silent | — | — |
| t3-018 | safe | path_walk | none | silent | — | — |
| t3-019 | safe | none | none | silent | — | — |
| t3-020 | safe | none | none | silent | — | — |
| t3-021 | safe | none | none | silent | — | — |
| t3-022 | safe | path_walk | partial | fires | key | R-COERCE |
| t3-023 | safe | path_walk | partial | fires | key | R-PAIR |
| t3-024 | safe | path_walk | partial | fires | keys | R-KEY-PROOF |
| t3-025 | pollutes | nested_keys | none | fires | section | — |
| t3-026 | pollutes | recursive_merge | none | fires | key | — |
| t3-027 | safe | none | none | silent | — | — |
| t3-028 | pollutes | recursive_merge | none | fires | key | — |
| t3-029 | pollutes | nested_keys | none | fires | node | — |
| t3-030 | pollutes | nested_keys | none | fires | font | — |
| t3-031 | pollutes | nested_keys | none | fires | node | — |
| t3-032 | pollutes | nested_keys | none | fires | index | — |
| t3-033 | pollutes | nested_keys | none | fires | index | — |
| t3-034 | pollutes | nested_keys | none | fires | position | — |
| t3-035 | safe | none | none | silent | — | — |
| t3-036 | pollutes | nested_keys | none | fires | startIdx | — |
| t3-037 | pollutes | nested_keys | none | fires | group | — |
| t3-038 | safe | none | none | silent | — | — |
| t3-039 | pollutes | nested_keys | none | fires | type | — |
| t3-040 | pollutes | nested_keys | none | fires | group | — |
| t3-041 | pollutes | nested_keys | none | fires | group | — |
| t3-042 | pollutes | nested_keys | none | fires | section | — |
| t3-043 | pollutes | nested_keys | none | fires | section | — |
| t3-044 | pollutes | nested_keys | none | fires | catalog | — |
| t3-045 | pollutes | nested_keys | none | fires | name | — |
| t3-046 | pollutes | nested_keys | none | fires | section | — |
| t3-047 | pollutes | path_walk | none | fires | section | — |
| t3-048 | pollutes | path_walk | none | fires | section | — |
| t3-049 | pollutes | path_walk | none | fires | id | — |
| t3-050 | pollutes | path_walk | none | fires | section | — |
| t3-051 | pollutes | path_walk | none | fires | req | — |
| t3-052 | pollutes | recursive_merge | none | fires | key | — |
| t3-053 | pollutes | recursive_merge | none | fires | key | — |
| t3-054 | pollutes | path_walk | partial | fires | pending, key | — |
| t3-055 | pollutes | path_walk | none | fires | names | — |
| t3-056 | safe | path_walk | partial | fires | key | R-KEY-PROOF |
