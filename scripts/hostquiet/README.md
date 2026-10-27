# Host-quiet marker

This cooperative marker serializes measurement steps on a shared machine.
Participating heavy work waits before starting while a live marker exists.
Already-running work is not paused, and the marker does not reserve CPU or RAM.

## Protocol

A measurement runner admits a step according to its own resource policy,
acquires the marker, then re-checks that the host is still quiet. It runs one
step and releases in a `finally` block or shell trap. If the host is busy after
acquisition, release and retry admission. Acquire separately for every step;
two measurements cannot hold the marker simultaneously. Heavy work calls
`wait_until_free` before planning, syncing trees or building. This is a
cooperative pre-start check, so work admitted just before acquisition may
still start; the measurement's post-acquisition quiet check is essential.

## Format and stale holders

`HOST_QUIET_DIR` selects the directory; the default is
`~/.local/state/host-quiet`, with `~` expanded. All participants must select the
same directory. Acquisition uses atomic `os.mkdir` of `<dir>/lock`. Its
`info.json` is written to a temporary file inside the lock directory and
published with `os.rename`:

```json
{"owner": "measurement", "step": "sample", "pid": 1234, "started_epoch": 1000.0, "expected_end_epoch": 2200.0}
```

A holder is stale if any of these conditions holds:

- Metadata is unreadable or missing and the lock directory's mtime is more
  than 60 seconds old. Incomplete or malformed metadata uses the same grace.
- `os.kill(pid, 0)` raises `ProcessLookupError`. `PermissionError` means alive.
- The current epoch is strictly greater than `expected_end_epoch + 1800`,
  even if the process is alive. Allow enough expected time for the step.

Otherwise the marker is live. Stale takeover renames the directory to
`lock.stale-<epoch>`, deletes that renamed directory, and tries `os.mkdir`
again. Losing acquisition returns immediately. Release removes a marker only
when its recorded pid equals the releasing holder's pid. Status and waiting
observe staleness without deleting anything.

## CLI and library

Run from the repository root with Python 3.9 or newer; no packages are needed:

```sh
python3 scripts/hostquiet/host_quiet.py acquire --owner measurement --step sample --expected-secs 1500
python3 scripts/hostquiet/host_quiet.py status
python3 scripts/hostquiet/host_quiet.py release
python3 scripts/hostquiet/host_quiet.py wait --timeout 1800 --poll 15
```

Acquire exits 0 on success or 1 for a live holder, printing holder JSON to
stderr on refusal. Acquire and release accept `--pid`; their default is the
parent process (`os.getppid()`), so the long-lived shell runner owns the
short-lived CLI's marker. Release always exits 0 and prints `released` or
`not released`. Status exits 0, printing `free` or holder JSON with a `stale`
boolean (only that boolean if metadata cannot be read).

Wait exits 0 as soon as no live marker exists and 3 on timeout. Timeout is
optional (default unlimited); poll defaults to 15 seconds and must be positive.
It logs owner, step and expected end in local time once per observed holder.

Import `host_quiet` from `scripts/hostquiet` for
`wait_until_free(log, timeout=None, poll=15.0, directory=None, clock=time.time,
sleep=time.sleep) -> bool`. It returns false on timeout; injectable clock and
sleep make tests deterministic. Library `acquire` and `release` default to the
current process pid, suitable for a long-lived Python runner.

```sh
python3 -m unittest discover -s scripts/hostquiet -p 'test_*.py'
```
