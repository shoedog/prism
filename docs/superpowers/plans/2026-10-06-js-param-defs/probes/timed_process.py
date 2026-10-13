"""Time and reap our own analyzer child, retaining its exit status and peak RSS."""
import os
import subprocess
import sys
import time


def timed_process(command, stdout, stderr, timeout):
    started = time.monotonic()
    process = subprocess.Popen([str(c) for c in command], stdout=stdout, stderr=stderr)
    timed_out = False
    while True:
        pid, status, usage = os.wait4(process.pid, os.WNOHANG)
        if pid:
            process.returncode = os.waitstatus_to_exitcode(status)
            break
        if time.monotonic() - started >= timeout:
            process.kill()
            _, status, usage = os.wait4(process.pid, 0)
            process.returncode = os.waitstatus_to_exitcode(status)
            timed_out = True
            break
        time.sleep(0.02)
    rss_bytes = usage.ru_maxrss * (1 if sys.platform == 'darwin' else 1024)
    return {'exit': 124 if timed_out else process.returncode,
            'seconds': round(time.monotonic() - started, 3), 'max_rss_bytes': rss_bytes}
