#!/usr/bin/env python3
"""Extract test functions from integration_test.rs into a new test file.

Usage:
  python3 scripts/extract_tests.py <output_file> <test_name_pattern>... [--apply]

Dry run (default): shows what would be extracted.
--apply: actually writes the files.

The output file will have `mod common; use common::*;` header.
Local helpers used ONLY by the extracted tests are moved too.
"""

import re
import sys

INPUT = "tests/integration_test.rs"


def find_test_functions(lines):
    """Find all #[test] fn test_* with their exact line ranges (0-indexed)."""
    tests = []
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('fn test_') and i > 0 and lines[i-1].strip() == '#[test]':
            name = re.match(r'fn\s+(test_\w+)', stripped).group(1)
            attr_line = i - 1
            brace_count = 0
            found_open = False
            end_idx = i
            for j in range(i, len(lines)):
                for ch in lines[j]:
                    if ch == '{':
                        brace_count += 1
                        found_open = True
                    elif ch == '}':
                        brace_count -= 1
                        if found_open and brace_count == 0:
                            end_idx = j
                            break
                if found_open and brace_count == 0:
                    break
            tests.append({'name': name, 'attr_line': attr_line, 'end_line': end_idx})
    return tests


def find_helpers(lines):
    """Find non-test fn defs, skipping those inside raw string literals."""
    helpers = []
    in_raw = False
    for i, line in enumerate(lines):
        if 'r#"' in line:
            in_raw = True
        if '"#' in line:
            in_raw = False
        if in_raw:
            continue
        stripped = line.strip()
        if (stripped.startswith('fn ') or stripped.startswith('pub fn ')) and not stripped.startswith('fn test_'):
            m = re.match(r'(?:pub\s+)?fn\s+(\w+)', stripped)
            if not m:
                continue
            name = m.group(1)
            brace_count = 0
            found_open = False
            end_idx = i
            for j in range(i, len(lines)):
                for ch in lines[j]:
                    if ch == '{':
                        brace_count += 1
                        found_open = True
                    elif ch == '}':
                        brace_count -= 1
                        if found_open and brace_count == 0:
                            end_idx = j
                            break
                if found_open and brace_count == 0:
                    break
            helpers.append({'name': name, 'start': i, 'end': end_idx})
    return helpers


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    apply = '--apply' in sys.argv
    args = [a for a in sys.argv[1:] if a != '--apply']
    output_file = args[0]
    patterns = args[1:]

    with open(INPUT) as f:
        lines = f.readlines()

    all_tests = find_test_functions(lines)
    all_helpers = find_helpers(lines)

    # Match tests
    matched = [t for t in all_tests if any(p in t['name'].lower() for p in patterns)]
    unmatched = [t for t in all_tests if t not in matched]

    if not matched:
        print(f"No tests matched patterns: {patterns}")
        sys.exit(1)

    # Find local helpers to move
    matched_text = ''.join(''.join(lines[t['attr_line']:t['end_line']+1]) for t in matched)
    unmatched_text = ''.join(''.join(lines[t['attr_line']:t['end_line']+1]) for t in unmatched)

    helpers_to_move = []
    for h in all_helpers:
        call = h['name'] + '('
        if call in matched_text and call not in unmatched_text:
            helpers_to_move.append(h)

    # Collect line ranges to remove
    remove_set = set()
    for t in matched:
        for l in range(t['attr_line'], t['end_line'] + 1):
            remove_set.add(l)
    for h in helpers_to_move:
        for l in range(h['start'], h['end'] + 1):
            remove_set.add(l)

    # Build extracted content
    extracted_parts = []
    # Helpers first
    for h in sorted(helpers_to_move, key=lambda x: x['start']):
        extracted_parts.append(''.join(lines[h['start']:h['end']+1]))
    # Then tests
    for t in sorted(matched, key=lambda x: x['attr_line']):
        extracted_parts.append(''.join(lines[t['attr_line']:t['end_line']+1]))

    output_content = "mod common;\nuse common::*;\n\n" + '\n\n'.join(extracted_parts) + '\n'

    # Build remaining integration_test.rs
    remaining = []
    prev_blank = False
    for i, line in enumerate(lines):
        if i in remove_set:
            continue
        is_blank = line.strip() == ''
        if is_blank and prev_blank:
            continue
        remaining.append(line)
        prev_blank = is_blank

    # Also remove orphaned section headers (// ====== with no test after)
    cleaned = []
    for i, line in enumerate(remaining):
        if line.strip().startswith('// ======') or line.strip().startswith('// ──'):
            # Check if there's a test within the next 3 non-blank lines
            has_test = False
            for j in range(i+1, min(i+5, len(remaining))):
                if remaining[j].strip().startswith('#[test]') or remaining[j].strip().startswith('fn test_'):
                    has_test = True
                    break
                if remaining[j].strip().startswith('// ======') or remaining[j].strip().startswith('// ──'):
                    break
                if remaining[j].strip().startswith('fn ') and not remaining[j].strip().startswith('fn test_'):
                    has_test = True  # helper function follows
                    break
            if not has_test:
                continue
        cleaned.append(line)

    print(f"\nExtraction: {output_file}")
    print(f"  Tests: {len(matched)}")
    print(f"  Local helpers: {len(helpers_to_move)} ({[h['name'] for h in helpers_to_move]})")
    print(f"  Lines removed: {len(remove_set)}")

    if apply:
        with open(output_file, 'w') as f:
            f.write(output_content)
        with open(INPUT, 'w') as f:
            f.writelines(cleaned)
        print(f"  Applied!")
    else:
        print(f"  (dry run — add --apply to write)")


if __name__ == '__main__':
    main()
