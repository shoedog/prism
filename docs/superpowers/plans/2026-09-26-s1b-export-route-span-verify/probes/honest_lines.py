"""P8 honest-line measurement of a commit: added non-blank, non-comment lines in .rs files, split src vs tests.
tests = file under tests/, *_tests.rs / *_test.rs / tests.rs basename, or line inside a `#[cfg(test)]` item
(brace-matched in the post-image). Docs (.md), fixtures (.toml/.ts/...) reported separately.
Usage: python3 honest_lines.py <repo> <commit>"""
import subprocess, sys, re, collections
repo, c = sys.argv[1], sys.argv[2]
def git(*a): return subprocess.run(['git', '-C', repo, *a], capture_output=True, text=True).stdout
files = [l.split('\t') for l in git('show', '--numstat', '--format=', c).splitlines() if l.strip()]
out = collections.Counter()
per_file = collections.Counter()
for add, rem, path in files:
    if not path.endswith('.rs'):
        out['nonrs:' + path.rsplit('.', 1)[-1]] += int(add) if add.isdigit() else 0
        continue
    post = git('show', f'{c}:{path}').splitlines()
    test_lines = set()
    i = 0
    while i < len(post):
        if post[i].strip().startswith('#[cfg(test)]'):
            j = i + 1; depth = 0; started = False
            while j < len(post):
                depth += post[j].count('{') - post[j].count('}')
                if '{' in post[j]: started = True
                if started and depth <= 0: break
                if not started and post[j].rstrip().endswith(';'): break
                j += 1
            test_lines.update(range(i, j + 1)); i = j + 1
        else: i += 1
    is_test_file = path.startswith('tests/') or re.search(r'(_tests?|/tests)\.rs$', path)
    diff = git('show', '--format=', '-U0', c, '--', path).splitlines()
    ln = 0
    for d in diff:
        m = re.match(r'@@ -\d+(?:,\d+)? \+(\d+)', d)
        if m: ln = int(m.group(1)) - 1; continue
        if d.startswith('+++') or d.startswith('---'): continue
        if d.startswith('+'):
            ln += 1
            t = d[1:].strip()
            if not t or t.startswith('//'): continue
            b = 'tests' if is_test_file or (ln - 1) in test_lines else 'src'
            out[b] += 1
            per_file[(b, path)] += 1
print(c, dict(out))
if '--per-file' in sys.argv:
    for (b, path), n in sorted(per_file.items()):
        print('  %-5s %4d %s' % (b, n, path))
