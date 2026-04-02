# How To: Update the Coverage Matrix

The coverage matrix (`coverage/matrix.json`) is the source of truth for which languages support which features and which algorithms have been tested with which languages. It must be updated whenever the analysis capabilities change.

## When to update

| Trigger | What to change | Example |
|---------|---------------|---------|
| **New language added** | Add language to every applicable feature in `language_features`, add column to every algorithm in `algorithm_coverage` | Adding Shell/Bash support |
| **New algorithm added** | Add row to `algorithm_coverage` with status per language | Adding a new `dependency_slice` algorithm |
| **Language feature gap fixed** | Change `status: "gap"` → `status: "handled"`, add `tests` field | Fixing `for_of_destructuring` for JS/TS |
| **New language feature identified** | Add new entry to appropriate category in `language_features` | Discovering Python `match` statement needs handling |
| **Algorithm tests added for a language** | Update status in `algorithm_coverage` (`none` → `basic` → `full`) | Adding Go tests for `gradient_slice` |
| **Feature test added** | Add test pattern to the feature's `tests` array | Adding `test_python_with_as_*` for `with_as_binding` |

## Step by step

### 1. Edit `coverage/matrix.json`

The file has two main sections:

**`language_features`** — organized by category (variable_binding, field_access, control_flow, call_patterns, other_patterns). Each feature has:

```json
"object_destructuring": {
  "languages": ["javascript", "typescript"],
  "status": "handled",
  "tests": ["test_destructuring_object_*", "test_destructuring_renamed_*"]
}
```

- `languages`: which languages have this feature (not which languages Prism supports — only languages where the pattern exists)
- `status`: `"handled"` or `"gap"`
- `tests`: glob patterns matching test function names that verify the feature works. The validation test scans all test files (integration, CLI, and unit) for matches.
- `gap_id`: (gaps only) sequential number matching `docs/language-analysis-gaps.md`
- `tracking`: (gaps only) path to the tracking document
- `notes`: (optional) explanation of limitations or partial support

**`algorithm_coverage`** — one entry per algorithm with status per language:

```json
"taint": {
  "python": "full",
  "javascript": "full",
  "typescript": "basic",
  "go": "full",
  "java": "none",
  ...
}
```

Status levels:
- `"full"`: 3+ tests covering the algorithm with this language, including edge cases
- `"basic"`: 1-2 tests, typically just verifying it runs without error
- `"none"`: no tests for this algorithm-language combination

### 2. Run the badge generator

```bash
python3 scripts/generate_coverage_badges.py
```

This does three things:
- Generates `coverage/report.json`, `coverage/badges.md`, `coverage/table.md` (gitignored, not committed)
- Updates README.md badges and algorithm table between marker comments
- Prints a coverage summary to stdout

### 3. Verify the validation test passes

```bash
cargo test test_coverage_matrix -- --nocapture
```

This runs two checks:
- **`test_coverage_matrix_validation`**: every feature with `status: "handled"` and a `tests` array must have at least one matching test in the codebase. Fails if coverage drops below 80%.
- **`test_coverage_matrix_algorithm_completeness`**: every algorithm must be in the matrix, each with ≥2 languages covered.

If the validation test fails with "unverified claims," either:
- The test pattern doesn't match any test name — fix the glob pattern in `tests`
- The test doesn't exist yet — add it before claiming `status: "handled"`

### 4. Commit the changes

Commit both `coverage/matrix.json` and `README.md` (updated by the script). The generated files under `coverage/` are gitignored.

```bash
git add coverage/matrix.json README.md
git commit -m "Update coverage matrix: <what changed>"
```

### 5. CI verification

The `language-coverage` CI job will:
- Run the badge generator script
- Fail if README badges are out of sync with `matrix.json`
- Publish the coverage table to the GitHub Actions job summary

## Common scenarios

### Adding a new language (e.g., Shell/Bash)

1. Add `"shell"` to every feature in `language_features` that the language supports. Not every feature applies — Shell doesn't have destructuring, classes, or field access. Only add it where the pattern exists:

```json
"simple_assignment": {
  "languages": ["python", ..., "shell"],
  "status": "handled"
}
```

2. Add a column for every algorithm in `algorithm_coverage`:

```json
"taint": {
  ...,
  "shell": "none"
}
```

3. As tests are added, update `"none"` → `"basic"` → `"full"`.

4. Update the constants in `scripts/generate_coverage_badges.py`: `LANGUAGES`, `LANG_LABELS`, `LANG_LOGOS`, `LANG_SHORT`.

### Fixing a gap

1. Change the feature's status from `"gap"` to `"handled"`
2. Add the `tests` field with patterns matching the new tests
3. Remove `gap_id` and `tracking` fields
4. Update `docs/language-analysis-gaps.md` to mark the gap as fixed

```json
// Before:
"with_as_binding": {
  "languages": ["python"],
  "status": "gap",
  "gap_id": 4,
  "tracking": "docs/language-analysis-gaps.md"
}

// After:
"with_as_binding": {
  "languages": ["python"],
  "status": "handled",
  "tests": ["test_python_with_as_*"]
}
```

### Adding a new feature to track

1. Pick the right category (`variable_binding`, `field_access`, `control_flow`, `call_patterns`, or `other_patterns`)
2. Add the feature with all languages that have it
3. If it's already handled, include test patterns. If not, mark as a gap:

```json
"match_statement": {
  "languages": ["python"],
  "status": "gap",
  "gap_id": 10,
  "tracking": "docs/language-analysis-gaps.md",
  "notes": "Python 3.10+ structural pattern matching"
}
```

## File locations

| File | Purpose | Committed? |
|------|---------|------------|
| `coverage/matrix.json` | Source of truth | Yes |
| `coverage/report.json` | Generated report | No (gitignored) |
| `coverage/badges.md` | Generated badge markdown | No (gitignored) |
| `coverage/table.md` | Generated algorithm table | No (gitignored) |
| `scripts/generate_coverage_badges.py` | Generator script | Yes |
| `README.md` | Updated between marker comments | Yes |
| `docs/cross-language-coverage.md` | Design doc | Yes |
| `docs/language-analysis-gaps.md` | Gap tracking | Yes |
