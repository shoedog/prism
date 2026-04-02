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
import sys
from collections import defaultdict
from urllib.parse import quote

MATRIX_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "matrix.json")
REPORT_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "report.json")
BADGES_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "badges.md")
TABLE_PATH = os.path.join(os.path.dirname(__file__), "..", "coverage", "table.md")

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


def generate_badges(feature_cov):
    """Generate shields.io badge markdown."""
    lines = []
    for lang in LANGUAGES:
        label = LANG_LABELS[lang]
        logo = LANG_LOGOS[lang]
        pct = feature_cov[lang]["percentage"]
        color = badge_color(pct)
        encoded_label = quote(label)
        lines.append(
            f"![{label}](https://img.shields.io/badge/{encoded_label}-{pct}%25-{color}?logo={logo}&logoColor=white)"
        )

    # Overall
    total_handled = sum(v["handled"] for v in feature_cov.values())
    total_all = sum(v["total"] for v in feature_cov.values())
    overall_pct = int(100 * total_handled / total_all) if total_all > 0 else 0
    overall_color = badge_color(overall_pct)
    lines.append("")
    lines.append(
        f"![Language Coverage](https://img.shields.io/badge/language_coverage-{len(LANGUAGES)}_languages_%7C_{overall_pct}%25-{overall_color})"
    )
    return "\n".join(lines)


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


def main():
    matrix = load_matrix()
    feature_cov = compute_feature_coverage(matrix)
    algo_cov = compute_algorithm_coverage(matrix)

    # Generate report
    report = generate_report(feature_cov, algo_cov, matrix)
    with open(REPORT_PATH, "w") as f:
        json.dump(report, f, indent=2)
    print(f"Report written to {REPORT_PATH}")

    # Generate badges
    badges = generate_badges(feature_cov)
    with open(BADGES_PATH, "w") as f:
        f.write(badges + "\n")
    print(f"Badges written to {BADGES_PATH}")

    # Generate table
    table = generate_algorithm_table(matrix)
    with open(TABLE_PATH, "w") as f:
        f.write(table + "\n")
    print(f"Table written to {TABLE_PATH}")

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
