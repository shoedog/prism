"""Validation for the opt-in, pinned private-mirror edit scenario."""
import hashlib
import re
from pathlib import PurePosixPath


def verified_original(root, edit):
    """Refuse before any child or write; only bytes read here may be used for recovery."""
    target = root / edit['path']
    try:
        if target.is_symlink() or target.resolve() != target or not target.is_file():
            raise OSError('target missing, replaced or not a regular mirror file')
        original = target.read_bytes()
    except OSError as error:
        raise SystemExit('changed-refresh target unavailable: %s' % error) from error
    if original != edit['before'].encode():
        raise SystemExit('changed-refresh wrong starting content')
    return original


def edit_reason(edit):
    keys = ('repo_root', 'commit', 'path', 'before', 'after', 'before_sha256', 'after_sha256')
    if not isinstance(edit, dict) or set(edit) != set(keys) or any(not isinstance(edit[k], str) for k in keys):
        return 'changed edit is missing exact content or identity'
    if not edit['repo_root'].startswith('/') or not re.fullmatch(r'[0-9a-f]{40}', edit['commit']):
        return 'changed edit identity unavailable'
    path = PurePosixPath(edit['path'])
    if not path.parts or path.is_absolute() or '..' in path.parts or '.git' in path.parts:
        return 'changed edit path must be relative and inside the mirror'
    if not edit['before'] or not edit['after'] or edit['before'] == edit['after']:
        return 'changed edit is empty or a no-op'
    for key in ('before', 'after'):
        try:
            digest = hashlib.sha256(edit[key].encode()).hexdigest()
        except UnicodeError:
            return 'changed edit is not UTF-8'
        if digest != edit[key + '_sha256']:
            return 'changed edit %s hash mismatch' % key
    return None


def validate_changed(report, expected):
    section = report.get('changed_refresh')
    if not isinstance(section, dict):
        return 'changed-refresh record missing'
    edit = section.get('edit')
    reason = edit_reason(edit)
    if reason:
        return reason
    if expected is not None and edit != expected:
        return 'changed edit differs from preregistered manifest'
    for observed, declared in (('before_sha256', 'before_sha256'), ('after_sha256', 'after_sha256'),
                               ('restored_sha256', 'before_sha256')):
        if section.get(observed) != edit[declared]:
            return 'changed-refresh %s hash mismatch' % observed
    refresh = report.get('refresh', {})
    if any(type(refresh.get(k)) is not int or refresh[k] < 0 for k in ('generation', 'indexed_files', 'tracked_paths',
                                                    'stale_index_total_before_refresh')):
        return 'changed-refresh full result is incomplete'
    # S0 is FullOnly; later strategy experiments must revise this acceptance contract deliberately.
    if ('fallback_reason' not in refresh or refresh.get('strategy') != 'full' or refresh.get('generation') != 1
            or refresh.get('fallback_reason') is not None or refresh.get('stale_before_refresh') is not True
            or refresh.get('stale_index_total_before_refresh') != 1
            or refresh.get('stale_index_paths_before_refresh') != [edit['path']]):
        return 'changed-refresh result does not report the declared change'
    return None
