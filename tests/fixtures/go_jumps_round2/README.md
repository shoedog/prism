# Historical slice-2 round-2 contract

The original `head4797.json` endpoint/control receipt is retained. #645 updates the explicit label
changes for switch and else-if entries that are now reachable: missingCase82→76/77/86, elseIf116→120
and the nested-condition b108→113. The jump target, truly unreachable definition and cycle controls
remain. The immediate-base receipt in `../go_branches` independently covers all endpoints and labels.
