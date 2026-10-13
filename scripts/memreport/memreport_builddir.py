"""The build kind's cache directories: created exclusively, deleted only when still the one that was created.

This is the only place the driver deletes anything, so it deletes with proof: the directory's identity
(device and inode, not its name) is recorded when it is created and checked again, without following links,
immediately before it is removed; and what is reported afterwards is what was observed.
"""
import os
import shutil
import stat
import sys
from pathlib import Path


def directory_identity(path):
    """(st_dev, st_ino) of `path` itself, not of what a symlink there points to."""
    status = os.lstat(path)
    return status.st_dev, status.st_ino


def inside(path, root):
    """Whether `path` resolves to somewhere strictly under `root` (symlinks followed)."""
    return Path(root).resolve() in Path(path).resolve().parents


def prepare_repetition(root, name, rep):
    """Create <root>/<name>/<rep> exclusively. Returns (directory, identity, None), or (None, None, reason).

    Refused, before anything is created below a link, when `root` or <root>/<name> is a symlink; refused when the
    repetition directory already exists; refused with the error text on any other filesystem error (a regular
    file in the way, a read-only directory, a full disk), so one bad state does not abort the whole run.
    """
    parent = root / name
    try:
        for path in (root, parent):
            if path.is_symlink():
                return None, None, ('%s is a symlink, so the cache directory could resolve outside the run '
                                    'directory' % path)
            path.mkdir(parents=path is root, exist_ok=True)
            if path.is_symlink() or not path.is_dir():
                return None, None, '%s is not a plain directory' % path
        directory = parent / str(rep)
        directory.mkdir()
        return directory, directory_identity(directory), None
    except FileExistsError:
        return None, None, 'the cache directory already existed'
    except OSError as error:
        return None, None, 'could not create the cache directory: %s' % error


def refuse_removal(cache_dir, note):
    print('warning: did not remove %s: %s' % (cache_dir, note), file=sys.stderr, flush=True)
    return False, note


def remove_created(cache_dir, root, identity):
    """Delete the directory `prepare_repetition` created, if it is still that directory. Returns (removed, note).

    Nothing is deleted unless `cache_dir` is, right now, a real directory (not a link) with the recorded
    identity that resolves inside `root`. `removed` is True only when the path is gone afterwards; otherwise
    `note` says why, and the path is named on stderr. Empty parents are removed after a success.
    """
    try:
        status = os.lstat(cache_dir)
    except OSError as error:
        return refuse_removal(cache_dir, 'it can no longer be inspected (%s); the directory this run created '
                                         'is no longer at this path' % error)
    if stat.S_ISLNK(status.st_mode) or not stat.S_ISDIR(status.st_mode) or (status.st_dev, status.st_ino) != identity:
        return refuse_removal(cache_dir, 'the path is no longer the directory this run created (replaced or '
                                         'moved during the measurement); left alone')
    if not inside(cache_dir, root):
        return refuse_removal(cache_dir, 'it resolves outside %s; left alone' % root)
    try:
        shutil.rmtree(cache_dir)
    except OSError as error:
        return refuse_removal(cache_dir, 'could not be deleted: %s' % error)
    if os.path.lexists(cache_dir):
        return refuse_removal(cache_dir, 'it still exists after the removal')
    for parent in (cache_dir.parent, root):
        try:
            parent.rmdir()    # only succeeds when empty
        except OSError:
            break
    return True, None
