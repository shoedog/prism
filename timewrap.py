#!/usr/bin/env python3
"""usage: timewrap.py TIME_FILE CMD [ARGS...]
Runs CMD (stdout/stderr inherited), writes wall seconds and peak RSS of the child tree to TIME_FILE and exits with
CMD's own exit code. No /usr/bin/time needed. ru_maxrss is KiB on Linux, bytes on macOS (normalised to KiB)."""
import resource, subprocess, sys, time

out, cmd = sys.argv[1], sys.argv[2:]
t0 = time.monotonic()
code = subprocess.call(cmd)
wall = time.monotonic() - t0
rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
if sys.platform == "darwin":
    rss //= 1024
with open(out, "w") as f:
    f.write(f"Elapsed (wall clock) seconds: {wall:.2f}\n")
    f.write(f"Maximum resident set size (kbytes): {rss}\n")
    f.write(f"Exit status: {code}\n")
sys.exit(code)
