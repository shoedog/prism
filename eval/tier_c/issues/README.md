<!-- eval/tier_c/issues/README.md -->
# Tier-C open-issue registry
`issues.toml` holds the frozen selection (spec §4). Each `[[issue]]` MUST satisfy the Goldilocks rubric:
multi-file (`files_touched_hint >= 2`), needs spec+plan (not one-shot), tractable (scope to first slice via
`scoped_slice`), pinned `sha` where the issue is still OPEN, buildable repo. Do not edit after a run starts.
