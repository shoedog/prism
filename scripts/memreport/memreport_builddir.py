"""The build kind's cache directories: created exclusively, and emptied only through a descriptor of the
directory that was created.

This is the only place the driver deletes anything. A path can be made to name something else between any two
looks at it (the directory renamed away, another moved in), so nothing here deletes by path. The directory is
created relative to an open descriptor of its parent, opened at once, and that descriptor is kept until
cleanup: its contents are removed through it, wherever the directory has gone, and the name is then removed
with `rmdir`, which cannot remove anything that has content. What is reported afterwards is what was observed.
"""
import os
import sys
from pathlib import Path

DIRECTORY = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW


class Owned:
    """The directory `prepare_repetition` created: open descriptors of it and of its parent, and its identity."""

    def __init__(self, name, parent_fd, fd):
        self.name, self.parent_fd, self.fd = name, parent_fd, fd
        status = os.fstat(fd)
        self.identity = (status.st_dev, status.st_ino)

    def close(self):
        for attribute in ('fd', 'parent_fd'):
            descriptor = getattr(self, attribute)
            if descriptor is not None:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
                setattr(self, attribute, None)


def directory_identity(path):
    """(st_dev, st_ino) of `path` itself, not of what a symlink there points to."""
    status = os.lstat(path)
    return status.st_dev, status.st_ino


def inside(path, root):
    """Whether `path` resolves to somewhere strictly under `root` (symlinks followed)."""
    return Path(root).resolve() in Path(path).resolve().parents


def prepare_repetition(root, name, rep, after_mkdir=None):
    """Create <root>/<name>/<rep> exclusively. Returns (directory, owned, None), or (None, None, reason).

    Refused, before anything is created below a link, when `root` or <root>/<name> is a symlink; refused when the
    repetition directory already exists; refused when what is at the path right after its creation is not an
    empty directory (it was replaced); refused with the error text on any other filesystem error (a regular file
    in the way, a read-only directory, a full disk), so one bad state does not abort the whole run.
    `after_mkdir` is called between the creation and the opening, for tests that replace the directory there.
    """
    parent = root / name
    parent_fd = fd = None
    try:
        for path in (root, parent):
            if path.is_symlink():
                return None, None, ('%s is a symlink, so the cache directory could resolve outside the run '
                                    'directory' % path)
            path.mkdir(parents=path is root, exist_ok=True)
            if path.is_symlink() or not path.is_dir():
                return None, None, '%s is not a plain directory' % path
        parent_fd = os.open(parent, DIRECTORY)
        os.mkdir(str(rep), dir_fd=parent_fd)
        if after_mkdir:
            after_mkdir()
        fd = os.open(str(rep), DIRECTORY, dir_fd=parent_fd)
        if os.listdir(fd):
            # A directory with content is not the one created a moment ago. It is not this run's to delete.
            return None, None, 'the cache directory was replaced right after it was created; left alone'
        owned, parent_fd, fd = Owned(str(rep), parent_fd, fd), None, None
        return parent / str(rep), owned, None
    except FileExistsError:
        return None, None, 'the cache directory already existed'
    except OSError as error:
        return None, None, 'could not create the cache directory: %s' % error
    finally:
        for descriptor in (fd, parent_fd):
            if descriptor is not None:
                os.close(descriptor)


def refuse_removal(cache_dir, note):
    print('warning: did not remove %s: %s' % (cache_dir, note), file=sys.stderr, flush=True)
    return False, note


def empty_through(fd):
    """Delete everything inside the directory open at `fd`, by descriptor, never by path.

    Each sub-directory is opened relative to its parent's descriptor without following a link, emptied the
    same way and removed by `rmdir`; everything else is unlinked. (`shutil.rmtree` takes a `dir_fd` only from
    Python 3.11.)
    """
    with os.scandir(fd) as entries:
        listed = [(entry.name, entry.is_dir(follow_symlinks=False)) for entry in entries]
    for name, is_directory in listed:
        if not is_directory:
            os.unlink(name, dir_fd=fd)
            continue
        child = os.open(name, DIRECTORY, dir_fd=fd)
        try:
            empty_through(child)
        finally:
            os.close(child)
        os.rmdir(name, dir_fd=fd)


def remove_created(cache_dir, root, owned, at=None):
    """Delete the directory `prepare_repetition` created. Returns (removed, note).

    Its contents are deleted through the descriptor opened at creation, so they are the contents of that
    directory wherever it is now; a directory moved into its path is never touched. The name is then removed
    with `rmdir` relative to the parent's descriptor, and only if it still is that directory: `rmdir` cannot
    remove a directory with content, so even a replacement made at that instant loses nothing. `removed` is
    True only when the path is gone afterwards; otherwise `note` says why, and the path is named on stderr.
    Empty parents are removed after a success. `at` is called with 'before-empty' and 'before-rmdir', for tests
    that replace the directory at those points.
    """
    at = at or (lambda step: None)
    try:
        at('before-empty')
        try:
            empty_through(owned.fd)
        except OSError as error:
            return refuse_removal(cache_dir, 'could not be deleted: %s' % error)
        try:
            status = os.stat(owned.name, dir_fd=owned.parent_fd, follow_symlinks=False)
        except OSError as error:
            return refuse_removal(cache_dir, 'the directory this run created is no longer at this path (%s); it '
                                             'was emptied where it now is and the path was left alone' % error)
        if (status.st_dev, status.st_ino) != owned.identity:
            return refuse_removal(cache_dir, 'the path is no longer the directory this run created (replaced or '
                                             'moved during the measurement); left alone')
        at('before-rmdir')
        try:
            os.rmdir(owned.name, dir_fd=owned.parent_fd)
        except OSError as error:
            return refuse_removal(cache_dir, 'could not be deleted: %s' % error)
    finally:
        owned.close()
    if os.path.lexists(cache_dir):
        return refuse_removal(cache_dir, 'it still exists after the removal')
    for parent in (cache_dir.parent, root):
        try:
            parent.rmdir()    # only succeeds when empty
        except OSError:
            break
    return True, None
