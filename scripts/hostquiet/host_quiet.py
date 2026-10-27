#!/usr/bin/env python3
"""Cooperative host-wide quiet marker; Python 3.9, standard library only."""

import argparse
from contextlib import contextmanager
import fcntl
import json
import math
import os
from pathlib import Path
import shutil
import sys
import tempfile
import threading
import time

GUARD_WAIT_SECS = 10.0
MAX_EXPECTED_SECS = 6 * 60 * 60
_guard_fds = set()
_guard_fds_lock = threading.Lock()


def _after_fork_child():
    try:
        for fd in _guard_fds:
            # LOCK_UN would also unlock the parent's shared open description.
            os.close(fd)
        _guard_fds.clear()
    finally:
        _guard_fds_lock.release()


os.register_at_fork(before=_guard_fds_lock.acquire,
                    after_in_parent=_guard_fds_lock.release,
                    after_in_child=_after_fork_child)


@contextmanager
def transition_guard(directory):
    """Serialize marker mutations; leave the guard inode in place forever."""
    deadline = time.monotonic() + GUARD_WAIT_SECS
    while True:
        # A fork must see either a registered fd or no fd. Failed attempts close
        # before dropping this lock, so retry sleeps never delay other forks.
        with _guard_fds_lock:
            fd = os.open(directory / 'transition.guard', os.O_CREAT | os.O_RDONLY, 0o666)
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                _guard_fds.add(fd)
                holder_pid = os.getpid()
            except (BlockingIOError, InterruptedError):
                os.close(fd)
                fd = None
            except BaseException:
                os.close(fd)
                raise
        if fd is not None:
            break
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            yield False
            return
        time.sleep(min(0.05, remaining))
    try:
        yield True
    finally:
        # A child may unwind an inherited context after its fork hook closed fd;
        # it must not close a descriptor subsequently reused in that child.
        if os.getpid() == holder_pid:
            with _guard_fds_lock:
                _guard_fds.remove(fd)
                try:
                    fcntl.flock(fd, fcntl.LOCK_UN)
                finally:
                    os.close(fd)


def marker_directory(directory=None):
    value = directory if directory is not None else os.environ.get(
        'HOST_QUIET_DIR', '~/.local/state/host-quiet')
    return Path(value).expanduser()


def read_info(lock):
    """Treat incomplete or malformed metadata like unreadable metadata."""
    try:
        info = json.loads((lock / 'info.json').read_text(encoding='utf-8'))
        if not isinstance(info, dict):
            return None
        if not all(isinstance(info.get(k), str) for k in ('owner', 'step')):
            return None
        if type(info.get('pid')) is not int or info['pid'] <= 0:
            return None
        for key in ('started_epoch', 'expected_end_epoch'):
            if type(info.get(key)) not in (int, float) or not math.isfinite(info[key]):
                return None
        return info
    except (OSError, ValueError, OverflowError):
        return None


def inspect_lock(directory=None, now=None):
    """Return (info, stale, stat), or None when the marker is free."""
    lock = marker_directory(directory) / 'lock'
    try:
        stat = lock.stat()
    except FileNotFoundError:
        return None
    info = read_info(lock)
    now = time.time() if now is None else now
    if info is None:
        return None, now - stat.st_mtime > 60, stat
    try:
        os.kill(info['pid'], 0)
    except ProcessLookupError:
        return info, True, stat
    except PermissionError:
        pass  # An existing process owned by another user is alive.
    except OverflowError:
        return None, now - stat.st_mtime > 60, stat
    return info, now > info['expected_end_epoch'] + 1800, stat


def acquire(owner, step, expected_secs, pid=None, directory=None):
    """Acquire or recover a marker under a bounded exclusive transition guard."""
    pid = os.getpid() if pid is None else pid
    if type(pid) is not int or pid <= 0:
        raise ValueError('pid must be positive')
    if not math.isfinite(expected_secs) or not 0 <= expected_secs <= MAX_EXPECTED_SECS:
        raise ValueError('expected_secs must be finite and between 0 and 21600 (6 hours)')
    directory = marker_directory(directory)
    directory.mkdir(parents=True, exist_ok=True)
    with transition_guard(directory) as held:
        return _acquire_locked(owner, step, expected_secs, pid, directory) if held else False


def _acquire_locked(owner, step, expected_secs, pid, directory):
    """The guard covers mkdir through publication, including failure cleanup."""
    lock = directory / 'lock'
    try:
        os.mkdir(lock)
    except FileExistsError:
        observed = inspect_lock(directory)
        if observed is not None:
            if not observed[1]:
                return False
            # Identity validation and replacement share the same guard.
            try:
                current = lock.stat()
                if (current.st_dev, current.st_ino) != (observed[2].st_dev, observed[2].st_ino):
                    return False
                stale = directory / ('lock.stale-' + str(time.time()))
                os.rename(lock, stale)
            except (FileNotFoundError, FileExistsError):
                return False
            shutil.rmtree(stale)
        try:
            os.mkdir(lock)
        except FileExistsError:
            return False
    now = time.time()
    info = dict(owner=owner, step=step, pid=pid, started_epoch=now,
                expected_end_epoch=now + expected_secs)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=lock,
                                         prefix='info-', delete=False) as handle:
            temporary = Path(handle.name)
            json.dump(info, handle)
        os.rename(temporary, lock / 'info.json')
    except BaseException:
        # Rename can have succeeded just before an exception was delivered.
        # Published metadata belongs to the holder and is never failure cleanup.
        if not (lock / 'info.json').exists():
            if temporary is not None:
                temporary.unlink(missing_ok=True)
            lock.rmdir()
        raise
    return True


def release(pid=None, directory=None):
    """Only the recorded holder may release; missing metadata is not ownership."""
    pid = os.getpid() if pid is None else pid
    directory = marker_directory(directory)
    lock = directory / 'lock'
    if not lock.exists():
        return False
    try:
        with transition_guard(directory) as held:
            if not held:
                return False
            info = read_info(lock)
            if info is None or info['pid'] != pid:
                return False
            shutil.rmtree(lock)
    except FileNotFoundError:
        return False
    return True


def wait_until_free(log, timeout=None, poll=15.0, directory=None,
                    clock=time.time, sleep=time.sleep):
    """Wait without taking a marker; log once per observed holder identity."""
    if not math.isfinite(poll) or poll <= 0:
        raise ValueError('poll must be finite and positive')
    if timeout is not None and (not math.isfinite(timeout) or timeout < 0):
        raise ValueError('timeout must be finite and nonnegative')
    started = clock()
    previous = None
    while True:
        now = clock()
        observed = inspect_lock(directory, now)
        if observed is None or observed[1]:
            return True
        info, _, stat = observed
        identity = (stat.st_dev, stat.st_ino, json.dumps(info, sort_keys=True))
        if identity != previous:
            if info is None:
                log('Waiting for host-quiet marker: owner unknown, step unknown, expected end unknown')
            else:
                end = time.strftime('%Y-%m-%d %H:%M:%S %Z', time.localtime(info['expected_end_epoch']))
                log(f'Waiting for host-quiet marker: owner={info["owner"]}, step={info["step"]}, expected end={end}')
            previous = identity
        if timeout is not None and now - started >= timeout:
            return False
        remaining = poll if timeout is None else min(poll, timeout - (now - started))
        sleep(remaining)


def finite_nonnegative(value):
    number = float(value)
    if not math.isfinite(number) or number < 0:
        raise argparse.ArgumentTypeError('must be finite and nonnegative')
    return number


def positive_pid(value):
    pid = int(value)
    if pid <= 0:
        raise argparse.ArgumentTypeError('pid must be positive')
    return pid


def expected_duration(value):
    number = finite_nonnegative(value)
    if number > MAX_EXPECTED_SECS:
        raise argparse.ArgumentTypeError('must be at most 21600 (6 hours)')
    return number


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    take = commands.add_parser('acquire')
    take.add_argument('--owner', required=True)
    take.add_argument('--step', required=True)
    take.add_argument('--expected-secs', required=True, type=expected_duration)
    take.add_argument('--pid', type=positive_pid, required=True)
    drop = commands.add_parser('release')
    drop.add_argument('--pid', type=positive_pid, required=True)
    commands.add_parser('status')
    wait = commands.add_parser('wait')
    wait.add_argument('--timeout', type=finite_nonnegative)
    wait.add_argument('--poll', type=finite_nonnegative, default=15.0)
    args = parser.parse_args(argv)
    if args.command == 'acquire':
        if acquire(args.owner, args.step, args.expected_secs, args.pid):
            return 0
        observed = inspect_lock()
        print(json.dumps(observed[0] if observed else None), file=sys.stderr)
        return 1
    if args.command == 'release':
        print('released' if release(args.pid) else 'not released')
        return 0
    if args.command == 'status':
        observed = inspect_lock()
        print('free' if observed is None else json.dumps(dict(observed[0] or {}, stale=observed[1])))
        return 0
    if args.poll <= 0:
        parser.error('--poll must be positive')
    return 0 if wait_until_free(lambda msg: print(msg, file=sys.stderr, flush=True),
                               timeout=args.timeout, poll=args.poll) else 3


if __name__ == '__main__':
    sys.exit(main())
