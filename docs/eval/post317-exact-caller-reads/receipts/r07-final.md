# R07 final receipt

- Candidate `14e86083c2c754ed412d440742bd5763be388e42`, tree `a0411db6d858fe34806376ce41d2621853848d8b`; sealed base `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`.
- Genuine v96 direct refusal then v97 rebuild/warm hit: **PASS 1/0**. Final candidate cache bin `96e94ebae7296f4474cd7c20371744326f22d01a2bcce473b4dfa923a6749a4e`, meta `d6c549b773607e8908dacd890b599dfbfaacb441870f5b065fed95c30a36f7ce`; raw log `candidate-core-cache-control.log`.
- Candidate fixed synthetic cost: **PASS 1/0**, 1122.65s; one warmup and five cold parse-plus-CPG samples each. Base receipt SHA `a7621448db7d56d2e9a598d671481ffbde78895c1db4b48c3f833523a3d128b0`; candidate raw log `candidate-core-cost.log`.

| Dialect | N | base median us | candidate median us | nodes | edges base→candidate | facts/missing pairs | cache bytes base→candidate |
|---|---:|---:|---:|---:|---|---|---|
| JS | 10 | 10215 | 17711 | 74 | 93→103 | 10/10 | 34092→36140 |
| JS | 100 | 605747 | 701400 | 704 | 903→1003 | 100/100 | 308142→328550 |
| JS | 1000 | 57649950 | 61578766 | 7004 | 9003→10003 | 1000/1000 | 3048642→3252650 |
| TS | 10 | 11596 | 12268 | 74 | 93→103 | 10/10 | 34092→36140 |
| TS | 100 | 646251 | 669005 | 704 | 903→1003 | 100/100 | 308142→328550 |
| TS | 1000 | 61497910 | 62131022 | 7004 | 9003→10003 | 1000/1000 | 3048642→3252650 |
| TSX | 10 | 12142 | 13063 | 74 | 93→103 | 10/10 | 34458→36526 |
| TSX | 100 | 663691 | 651051 | 704 | 903→1003 | 100/100 | 311478→332086 |
| TSX | 1000 | 60430657 | 59675920 | 7004 | 9003→10003 | 1000/1000 | 3081678→3287686 |

The required 1000-callable slowdown threshold is not breached: JS +6.8%, TS +1.0%, TSX -1.2%. Timings are descriptive; no concurrent Cargo build/test was observed during the sealed base run. Peak memory unavailable. Historical 3a2/1a835 evidence is superseded.
