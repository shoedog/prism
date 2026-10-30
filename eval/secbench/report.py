"""Source-bound R1 decision tables, preserving baseline and unresolved strata."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
from .run import OUTCOMES, WEIGHTS, workstream, canonical, digest


def load(path):
    return [json.loads(line) for line in path.read_bytes().splitlines()]


def table(headers, rows):
    return '\n'.join(['| ' + ' | '.join(headers) + ' |',
                      '| ' + ' | '.join('---' for _ in headers) + ' |',
                      *['| ' + ' | '.join(map(str, row)) + ' |' for row in rows]])


def build(rows, baseline, conversions):
    old = {(r['class'], r['entry']): r for r in baseline}
    coverage = defaultdict(lambda: Counter())
    for row in rows:
        previous = old[(row['class'], row['entry'])]
        reason = previous.get('gt_reason') or ('acquisition_excluded' if previous['outcome'] == 'acquisition_excluded' else 'eligible_GT')
        count = coverage[reason];count['entries'] += 1
        count['before_eligible'] += int(previous['outcome'] in OUTCOMES)
        eligible = row['outcome'] in OUTCOMES
        count['after_eligible'] += int(eligible)
        if eligible and previous['outcome'] == 'gt_unavailable':
            count['newly_eligible'] += 1
            count[row['source'].get('resolution_mode', 'unknown')] += 1
    categories = Counter(r.get('first_break', {}).get('category', 'unresolved') for r in rows if r['outcome'] in OUTCOMES and r['outcome'] != 'traced')
    for category in ('B-destructure','B-rest-spread','B-default','B-arguments','B-plain-argument'):
        categories.setdefault(category, 0)
    errors = Counter(r['error_mechanism']['mechanism'] for r in rows if r['outcome'] == 'prism_error')
    opportunities = {}
    by_key = {(r['class'], r['entry']): r for r in rows}
    for ws in (2,3,4,5,6):
        nonerrors = [r for r in rows if r['outcome'] in OUTCOMES and r['outcome'] not in ('traced','prism_error') and workstream(r['first_break']['category']) == ws]
        allerrors = [r for r in rows if r['outcome'] == 'prism_error' and workstream(r['first_break']['category']) == ws]
        singles = [r for r in conversions if r.get('single_workstream') == ws]
        joint = [r for r in conversions if r['conversion'] and len(r['workstreams']) > 1 and ws in r['workstreams']]
        stack = [r for r in conversions if len(r['workstreams']) > 1 and ws in r['workstreams']]
        def dedup(items):
            return len({by_key[(r['class'],r['entry'])].get('source',{}).get('normalized_handler_sha256') or r['class']+'/'+r['entry'] for r in items})
        opportunities[str(ws)] = {'nonerror_first_break': len(nonerrors), 'including_errors': len(nonerrors)+len(allerrors),
                                  'weighted_including_errors': sum(WEIGHTS[r['class']] for r in nonerrors+allerrors),
                                  'single_conversions': len(singles), 'single_weighted': sum(WEIGHTS[r['class']] for r in singles),
                                  'single_deduplicated': dedup(singles), 'joint_conversions': len(joint),
                                  'joint_weighted': sum(WEIGHTS[r['class']] for r in joint), 'joint_deduplicated': dedup(joint),
                                  'multi_barrier_entries': len(stack)}
    http = [r for r in rows if r['outcome'] in OUTCOMES and r['source'].get('normalized_handler_sha256')]
    return {'outcomes': dict(Counter(r['outcome'] for r in rows)), 'categories': dict(categories), 'error_mechanisms': dict(errors),
            'coverage': {k: dict(v) for k,v in sorted(coverage.items())}, 'workstreams': opportunities,
            'http_raw': len(http), 'http_deduplicated': len({r['source']['normalized_handler_sha256'] for r in http}),
            'severity_weights': WEIGHTS, 'conversion_statuses': dict(Counter(r['status'] for r in conversions))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path, required=True)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--conversions', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    rows, baseline, conversions = load(args.run/'entries.jsonl'), load(args.baseline/'entries.jsonl'), load(args.conversions/'entries.jsonl')
    if len(rows) != 600 or len({(r['class'],r['entry']) for r in rows}) != 600:
        raise ValueError('600 distinct rows required')
    if len(conversions) != sum(r['outcome'] in OUTCOMES and r['outcome'] != 'traced' for r in rows):
        raise ValueError('conversion ledger must retain every non-traced eligible entry')
    result = build(rows, baseline, conversions)
    outcome_rows = []
    for cls in sorted({r['class'] for r in rows}):
        counts = Counter(r['outcome'] for r in rows if r['class']==cls)
        outcome_rows.append([cls,sum(counts.values()),*[counts[o] for o in OUTCOMES],counts['gt_unavailable'],counts['acquisition_excluded']])
    lines = ['# MEAS-A repair R1 decision evidence','',
             'The original b7512809 packet and original observations are historical baselines. This version uses the same authenticated main binary and corpus bytes; product source is unchanged.', '',
             table(['Class','Total','Traced','Function only','Partial','Not reached','Prism error','GT unavailable','Acquisition excluded'],outcome_rows),'',
             '## Mechanism-uniform attribution','',
             'Outcome and mechanism are separate. Every Prism error has a deterministic error_mechanism. Syntax-only downstream screens remain hypotheses; no source Def means the break is at source binding. Ordinary arguments are outside the owner’s destructuring axis. Defaults receive no failure credit without a distinguishing control.', '',
             table(['Category','Workstream','Eligible nontraces'],[[k,workstream(k) or 'unassigned',v] for k,v in sorted(result['categories'].items())]),'',
             table(['Prism error mechanism','Entries'],sorted(result['error_mechanisms'].items())), '',
             table(['Workstream','Nonerror first breaks','Including errors sensitivity','Weighted with errors'],[[w,v['nonerror_first_break'],v['including_errors'],v['weighted_including_errors']] for w,v in result['workstreams'].items()]),'',
             '## Demonstrated conversions','',
             'Each attempted rewrite has an unmodified scratch control, authenticated observations, exact remapped formal/terminal identities and a retained patch. Single-workstream counts are distinct from joint conversions. Joint columns overlap across workstreams and must not be summed. A barrier stack without traced output receives no conversion credit.', '',
             table(['Workstream','Single raw','Single dedup','Single weighted','Joint raw','Joint dedup','Joint weighted','Entries with multiple barriers'],[[w,v['single_conversions'],v['single_deduplicated'],v['single_weighted'],v['joint_conversions'],v['joint_deduplicated'],v['joint_weighted'],v['multi_barrier_entries']] for w,v in result['workstreams'].items()]),'',
             'Weights: command/code injection 3, prototype pollution/path traversal 2, ReDoS 1; these are scenarios, not CVSS. Raw counts are the equal-weighted scores.', '',
             f"HTTP sources: {result['http_raw']} raw, {result['http_deduplicated']} normalized-handler clusters. Dedup removes comments and whitespace and normalizes identifier/string/number tokens; it is a near-clone sensitivity, not proof of semantic equivalence.", '',
             'The comparison should favor a combined ws6 registration + ws3 member checkpoint for HTTP recovery. Single conversions and severity scores govern standalone priority; raw clone-heavy HTTP first-break counts do not prove ws6 alone converts that population. B-rest/destructure, B-arguments and plain argument transfer stay separate. Admission remains an explicit ws4 stratum.', '',
             '## Ground-truth coverage by original reason','',
             table(['Original reason','Rows','Before eligible','After eligible','New eligible','HTTP gain','Checker gain'],[[k,v['entries'],v.get('before_eligible',0),v.get('after_eligible',0),v.get('newly_eligible',0),v.get('http_registration',0),v.get('typescript_checker',0)] for k,v in result['coverage'].items()]),'',
             'HTTP GT seeds the registered request formal directly. It measures the within-package value path and bypasses runtime ingress/native source recognition. The independent TypeScript 5.9.3 checker admits only one in-package callable signature, with stable lexical bindings and conservative mutation/ambiguity refusals. Both are separate binding modes.', '',
             '## Rewrites and limits','',
             '- Member-only: append an effect-free bare read after directive prologues, preserving the consumed property path.',
             '- Callback registration/capture: bind the identical function/arrow through a pure identity call in the same lexical scope before its containing statement; preserve anonymous name, argument position, body, this and closure. Earlier side-effecting arguments and conditional or reflective scopes are refused.',
             '- Rest and legacy arguments: add a named arrow checkpoint around the unchanged body, passing the original engine-created array/arguments object. Preserve outer signature/arity, lexical this/arguments and return value; eval/super/generators/async forms are refused. These simulate normalized input binding and explicitly change the source checkpoint, rather than claiming native caller-to-formal coverage.',
             '- Admission: copy dist/build bytes into indexed siblings; change only AST-identified local imports and the package entry locator, with exact endpoint remapping. Observable runtime paths and non-import path strings are refused before mutation.', '',
             'The four-distinct-barrier cap is declared in conversions-final/binding.json. Unresolved hypotheses and refused rewrites stay in the denominator. No corpus package, test input, server, callback or dependency was executed.', '',
             '## Independent labels and verification limits','',
             'blind-sample.jsonl is frozen and label-free, with 40 stratified entries. The separate reference file must be hidden from the second labeller. Cohen’s kappa is pending controller-supplied independent labels; use python3 -m eval.secbench.blind --reference repair-r1/blind-reference.jsonl --labels LABELS.jsonl. This enriched sample cannot estimate population prevalence.', '',
             'No runtime behavior, native source/sink recognition, field-selective source soundness, external dependency flow or shipped workstream implementation is established. Refused rewrite scopes were not verified. Query timeouts remain errors. See VERIFICATION.md for suite totals, exclusions, replay and same-environment RED controls.', '']
    args.out.mkdir(parents=True,exist_ok=True)
    (args.out/'decision.json').write_bytes(canonical(result))
    (args.out/'REPORT.md').write_text('\n'.join(lines))
    (args.out/'report-binding.json').write_bytes(canonical({'entries_sha256': digest((args.run/'entries.jsonl').read_bytes()),
        'baseline_sha256': digest((args.baseline/'entries.jsonl').read_bytes()),'conversions_sha256': digest((args.conversions/'entries.jsonl').read_bytes()),
        'reporter_sha256': digest(Path(__file__).read_bytes())}))


if __name__ == '__main__':
    main()
