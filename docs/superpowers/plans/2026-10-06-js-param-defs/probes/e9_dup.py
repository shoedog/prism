#!/usr/bin/env python3
"""E9 count: ADDED synthetic-owner byte rows whose wire identity (file/line/path/access of both ends,
labels ignored) already exists in the base wire rows (a callable nested in a named function owned twice).
Usage: e9_dup.py BASE_BYTES HEAD_CHANGED_ROWS"""
import json, sys
from collections import Counter
def wid(r):
    return json.dumps({e: {k: r[e][k] for k in ('file', 'line', 'path', 'access')} for e in ('from', 'to')}, sort_keys=True)
base = Counter(wid(json.loads(l)) for l in open(sys.argv[1]))
c = Counter()
for l in open(sys.argv[2]):
    ch = json.loads(l)
    if ch['class'] != 'ADDED': continue
    c['added'] += 1
    c['wire_duplicate_of_base' if base.get(wid(ch['row'])) else 'wire_new'] += 1
    c['param_def' if ch['row']['from'].get('parameter') else 'non_param_def'] += 1
print(json.dumps(dict(c), sort_keys=True))
