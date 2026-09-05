# Python/JS receiver repair execution plan

1. Fetch/rebind main, audit PR #237 and source consumers. DONE: main `10d82ca`,
   tracked-tree identity with checkout `ea2965e` established by empty diff.
2. Enumerate constructor authority failures with accumulated regression matrices.
   DONE: initial JS 5/Python 4 RED; final exact-base control reproduces six JS,
   six Python, and twelve TS/TSX false Exact cases.
3. Implement slice A enclosing owner visibility with preservation controls. DONE.
4. Implement slice B structured initialization and loop-write fences, with reset,
   ended-scope, nested-region and no-write controls. DONE, including Python loops.
5. Complete finite typed-name scope audit; repair demonstrated bounded defects. DONE.
6. Add full/subset/incremental and cache parity; advance cache versions 60/29. DONE.
7. Review at most three rounds per slice, run full default/MCP and required gates,
   record exact evidence and exclusions, refresh handoff and snapshot explicit files.
   DONE: default 3,713/0/1; MCP 3,903/0/1; matrix 104/104; quick completed with
   zero oracle/SUT errors and sole invalid reason corpus pin drift. See review record.
8. Owner-authorized publication: DONE at `9a790419`, pushed to
   `fix/python-js-receiver-authority`, PR #238 open against main. Merge not requested.

No unrelated Item 2 integration, Java work, write tools, or artifact cleanup.
