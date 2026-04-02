#!/usr/bin/env python3
"""Generate coverage badges and tables from coverage/matrix.json.

Outputs:
  coverage/report.json    — machine-readable coverage summary
  coverage/badges.md      — shields.io badge markdown for README
  coverage/table.md       — algorithm × language table for README

Usage:
  python3 scripts/generate_coverage_badges.py
"""

import json
import os
import re
import sys
from collections import defaultdict
from urllib.parse import quote

MATRIX_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "matrix.json")
REPORT_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "report.json")
BADGES_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "badges.md")
TABLE_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "table.md")
README_PATH = os.path.join(os.path.dirname(__file__), "..", "README.md")

LANGUAGES = ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust", "lua"]
LANG_LABELS = {
    "python": "Python", "javascript": "JavaScript", "typescript": "TypeScript",
    "go": "Go", "java": "Java", "c": "C", "cpp": "C++", "rust": "Rust", "lua": "Lua",
}
LANG_LOGOS = {
    "python": "python", "javascript": "javascript", "typescript": "typescript",
    "go": "go", "java": "openjdk", "c": "c", "cpp": "cplusplus",
    "rust": "rust", "lua": "lua",
}
LANG_SHORT = {
    "python": "Py", "javascript": "JS", "typescript": "TS", "go": "Go",
    "java": "Ja", "c": "C", "cpp": "C++", "rust": "Rs", "lua": "Lua",
}


def badge_color(pct):
    if pct >= 95:
        return "brightgreen"
    elif pct >= 85:
        return "green"
    elif pct >= 70:
        return "yellow"
    elif pct >= 50:
        return "orange"
    else:
        return "red"


def load_matrix():
    with open(MATRIX_PATH) as f:
        return json.load(f)


def compute_feature_coverage(matrix):
    """Compute per-language feature coverage from the language_features section."""
    lang_total = defaultdict(int)
    lang_handled = defaultdict(int)
    lang_gaps = defaultdict(list)

    for category, features in matrix["language_features"].items():
        for feature_name, spec in features.items():
            for lang in spec["languages"]:
                lang_total[lang] += 1
                if spec["status"] == "handled":
                    lang_handled[lang] += 1
                else:
                    lang_gaps[lang].append(feature_name)

    result = {}
    for lang in LANGUAGES:
        total = lang_total[lang]
        handled = lang_handled[lang]
        pct = int(100 * handled / total) if total > 0 else 0
        result[lang] = {
            "handled": handled,
            "total": total,
            "percentage": pct,
            "gaps": lang_gaps[lang],
        }
    return result


def compute_algorithm_coverage(matrix):
    """Compute algorithm × language coverage stats."""
    algo_data = matrix["algorithm_coverage"]
    lang_algo_full = defaultdict(int)
    lang_algo_basic = defaultdict(int)
    lang_algo_none = defaultdict(int)

    for algo, langs in algo_data.items():
        for lang in LANGUAGES:
            status = langs.get(lang, "none")
            if status == "full":
                lang_algo_full[lang] += 1
            elif status == "basic":
                lang_algo_basic[lang] += 1
            else:
                lang_algo_none[lang] += 1

    total_algos = len(algo_data)
    result = {}
    for lang in LANGUAGES:
        full = lang_algo_full[lang]
        basic = lang_algo_basic[lang]
        covered = full + basic
        pct = int(100 * covered / total_algos) if total_algos > 0 else 0
        result[lang] = {
            "full": full,
            "basic": basic,
            "none": lang_algo_none[lang],
            "total": total_algos,
            "covered": covered,
            "percentage": pct,
        }
    return result


def generate_badges(feature_cov, algo_cov):
    """Generate shields.io badge markdown with two rows and nav links."""
    lines = []

    # Row 1: Feature coverage badges → link to feature table
    lines.append("**Language feature coverage** · [details](#language-feature-coverage)")
    lines.append("")
    for lang in LANGUAGES:
        label = LANG_LABELS[lang]
        logo = LANG_LOGOS[lang]
        pct = feature_cov[lang]["percentage"]
        color = badge_color(pct)
        encoded_label = quote(label)
        lines.append(
            f"![{label}](https://img.shields.io/badge/{encoded_label}-{pct}%25-{color}?logo={logo}&logoColor=white)"
        )

    lines.append("")

    # Row 2: Algorithm test coverage badges → link to algorithm table
    lines.append("**Algorithm test coverage** · [details](#algorithm--language)")
    lines.append("")
    for lang in LANGUAGES:
        label = LANG_LABELS[lang]
        logo = LANG_LOGOS[lang]
        pct = algo_cov[lang]["percentage"]
        color = badge_color(pct)
        encoded_label = quote(label)
        lines.append(
            f"![{label} algo](https://img.shields.io/badge/{encoded_label}-{pct}%25-{color}?logo={logo}&logoColor=white)"
        )

    return "\n".join(lines)


def generate_feature_table(feature_cov, matrix):
    """Generate language feature coverage table: one row per language."""
    rows = [
        "| Language | Features | Coverage | Gaps |",
        "|----------|----------|----------|------|",
    ]
    for lang in LANGUAGES:
        cov = feature_cov[lang]
        label = LANG_LABELS[lang]
        pct = cov["percentage"]
        gaps = ", ".join(f"`{g}`" for g in cov["gaps"]) if cov["gaps"] else "—"
        rows.append(f"| {label} | {cov['handled']}/{cov['total']} | {pct}% | {gaps} |")
    return "\n".join(rows)


def generate_algorithm_table(matrix):
    """Generate algorithm × language coverage table."""
    algo_data = matrix["algorithm_coverage"]
    header_langs = " | ".join(LANG_SHORT[l] for l in LANGUAGES)
    header = f"| Algorithm | {header_langs} |"
    separator = "|" + "|".join(["---"] * (len(LANGUAGES) + 1)) + "|"

    rows = [header, separator]
    for algo in sorted(algo_data.keys()):
        cells = []
        for lang in LANGUAGES:
            status = algo_data[algo].get(lang, "none")
            if status == "full":
                cells.append(" ✅ ")
            elif status == "basic":
                cells.append(" 🟡 ")
            else:
                cells.append(" ❌ ")
        algo_display = algo.replace("_", " ").title().replace(" ", "")
        # Keep readable names
        algo_display = algo.replace("_s", " S").replace("_d", " D").replace("_f", " F").replace("_o", " O")
        algo_display = algo
        row = f"| {algo} | {'|'.join(cells)}|"
        rows.append(row)

    rows.append("")
    rows.append("✅ full (3+ tests) · 🟡 basic (1-2 tests) · ❌ none")
    return "\n".join(rows)


def generate_report(feature_cov, algo_cov, matrix):
    """Generate machine-readable report."""
    total_gaps = []
    for lang in LANGUAGES:
        for gap in feature_cov[lang]["gaps"]:
            total_gaps.append({"language": lang, "feature": gap})

    return {
        "feature_coverage": feature_cov,
        "algorithm_coverage": algo_cov,
        "summary": {
            "languages": len(LANGUAGES),
            "total_features": sum(v["total"] for v in feature_cov.values()),
            "handled_features": sum(v["handled"] for v in feature_cov.values()),
            "overall_percentage": int(
                100
                * sum(v["handled"] for v in feature_cov.values())
                / sum(v["total"] for v in feature_cov.values())
            ),
            "total_gaps": len(total_gaps),
            "gaps": total_gaps,
        },
    }


def update_readme(badges, algo_table, feature_table):
    """Update README.md between marker comments.

    Replaces content between:
      <!-- COVERAGE_BADGES_START --> ... <!-- COVERAGE_BADGES_END -->
      <!-- COVERAGE_TABLE_START --> ... <!-- COVERAGE_TABLE_END -->
      <!-- COVERAGE_FEATURE_TABLE_START --> ... <!-- COVERAGE_FEATURE_TABLE_END -->
    """
    if not os.path.exists(README_PATH):
        print(f"README not found at {README_PATH}, skipping update")
        return False

    with open(README_PATH) as f:
        content = f.read()

    updated = False

    replacements = [
        ("COVERAGE_BADGES", badges),
        ("COVERAGE_TABLE", algo_table),
        ("COVERAGE_FEATURE_TABLE", feature_table),
    ]

    for tag, new_content in replacements:
        pattern = rf"(<!-- {tag}_START -->).*?(<!-- {tag}_END -->)"
        replacement = f"<!-- {tag}_START -->\n{new_content}\n<!-- {tag}_END -->"
        new, n = re.subn(pattern, replacement, content, flags=re.DOTALL)
        if n > 0:
            content = new
            updated = True

    if updated:
        with open(README_PATH, "w") as f:
            f.write(content)
        print(f"README updated at {README_PATH}")
    else:
        print("No marker comments found in README — skipping update")

    return updated


def main():
    matrix = load_matrix()
    feature_cov = compute_feature_coverage(matrix)
    algo_cov = compute_algorithm_coverage(matrix)

    # Generate report
    report = generate_report(feature_cov, algo_cov, matrix)
    with open(REPORT_PATH, "w") as f:
        json.dump(report, f, indent=2)
    print(f"Report written to {REPORT_PATH}")

    # Generate badges (both rows)
    badges = generate_badges(feature_cov, algo_cov)
    with open(BADGES_PATH, "w") as f:
        f.write(badges + "\n")
    print(f"Badges written to {BADGES_PATH}")

    # Generate tables
    algo_table = generate_algorithm_table(matrix)
    feature_table = generate_feature_table(feature_cov, matrix)
    with open(TABLE_PATH, "w") as f:
        f.write(algo_table + "\n")
    print(f"Table written to {TABLE_PATH}")

    # Update README
    update_readme(badges, algo_table, feature_table)

    # Print summary
    print(f"\nLanguage Feature Coverage:")
    for lang in LANGUAGES:
        cov = feature_cov[lang]
        gaps = f"  gaps: {', '.join(cov['gaps'])}" if cov["gaps"] else ""
        print(f"  {LANG_LABELS[lang]:12s}: {cov['handled']}/{cov['total']} ({cov['percentage']}%){gaps}")

    print(f"\nOverall: {report['summary']['overall_percentage']}% ({report['summary']['handled_features']}/{report['summary']['total_features']} features)")
    print(f"Gaps remaining: {report['summary']['total_gaps']}")

    return 0 if report["summary"]["overall_percentage"] >= 85 else 1


if __name__ == "__main__":
    sys.exit(main())
