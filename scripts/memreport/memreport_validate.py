"""Which reports may be used as evidence (imported by memreport.py).

`well_formed` is the one structural check: after it passes, every field the driver reads has a type the
driver can use, so `derive`, `build_entry`, `assemble_summary` and `render` cannot raise on the report
and no non-finite number reaches a statistic. `validate` adds the rules of each run kind.
"""
from memreport_changed import validate_changed

REPORT_VERSION = 3
PRIME = 'prime'  # the discarded run that fills the cache; its report only has to parse
BUILD = 'build'  # a first build into an empty cache directory: the opposite of the cached kinds
FOOTPRINT = 'footprint'
U64_MAX = 2 ** 64 - 1  # the binary's counters are u64; a larger integer cannot have come from it

# Per kind: the (stage, field) pairs `derive` reads and cannot do without.
REQUIRED = {
    'warm': (('after_load_repo', 'peak_bytes'), ('after_index', 'peak_bytes'), ('after_index', FOOTPRINT)),
    'cold': (('after_call_edges', 'peak_bytes'),),
    'refresh': (('after_refresh', 'peak_bytes'),),
    'changed-refresh': tuple((name, 'peak_bytes') for name in
                             ('start', 'after_load_repo', 'after_index', 'after_call_edges',
                              'after_settle', 'after_refresh')),
    BUILD: (('after_call_edges', 'peak_bytes'),),
    'workload': (('after_settle', FOOTPRINT),),
    PRIME: (),
}
CACHE_REASONS = ('the CPG cache was not hit, so this was a cold build',
                 'the call-edge index was computed, not loaded from its sidecar')
# A `build` run is supposed to miss both caches; the report has to say so (a null is not a miss).
BUILD_REASONS = ('the CPG cache was hit, so this was not a first build',
                 'the call-edge index was loaded, so this was not a first build',
                 'the report does not say the CPG cache was missed',
                 'the report does not say the call-edge index was computed')
# A `build` run is also supposed to leave both cache files behind. The binary catches a failed save, prints it
# and exits 0 with both flags false, so only the files (and, at run time, the text) show that nothing was written.
BUILD_CACHE_REASONS = ('the run wrote no CPG cache file, so this was not a build that writes the cache',
                       'the run wrote no call-edge sidecar, so this was not a build that writes the cache',
                       'the result records no cache file sizes, so it is not shown to be a build that writes '
                       'the cache')
BUILD_SAVE_MARKERS = (
    ('failed to save CPG cache', 'the CPG cache could not be saved, so this was not a build that writes the cache'),
    ('nav edge cache: failed to save sidecar',
     'the call-edge sidecar could not be saved, so this was not a build that writes the cache'),
)
# A run of these kinds is supposed to load both caches from a primed directory.
CACHED_KINDS = ('warm', 'workload', 'refresh', 'changed-refresh')

STAGE_NUMBERS = ('elapsed_ms', 'rss_bytes', 'phys_footprint_bytes', 'compressed_bytes', 'peak_bytes',
                 'heap_in_use_bytes', 'tree_sitter_heap_bytes')
ATTRIBUTION_NUMBERS = ('files', 'source_bytes', 'tree_visible_nodes', 'cpg_nodes', 'cpg_edges',
                       'cpg_node_inline_bytes', 'node_strings', 'node_string_bytes',
                       'node_string_heap_bytes_estimate', 'node_unique_strings', 'node_unique_string_bytes',
                       'node_unique_string_heap_bytes_estimate')
WORKLOAD_NUMBERS = ('requests', 'responses', 'errors', 'response_bytes')
BUILD_STRINGS = ('package_version', 'git_sha', 'build_identity', 'grammar_fingerprint', 'allocator')


def is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def is_count(value):
    """A byte or event count: an int (not a bool, not a float) from 0 to the u64 maximum."""
    return type(value) is int and 0 <= value <= U64_MAX


def footprint(entry):
    """Physical footprint where the platform reports it, resident size otherwise."""
    value = entry.get('phys_footprint_bytes')
    return value if value is not None else entry.get('rss_bytes')


def find_stage(report, name):
    """The named stage, or None when the report has no such stage."""
    stages = report.get('stages')
    if not isinstance(stages, list):
        return None
    for entry in stages:
        if isinstance(entry, dict) and entry.get('name') == name:
            return entry
    return None


def bad_counts(section, label, keys):
    """The reason the first of `keys` in `section` is not None or a count, else None."""
    for key in keys:
        value = section.get(key)
        if value is not None and not is_count(value):
            return '%s.%s is %r, not a non-negative integer' % (label, key, value)
    return None


def well_formed(report):
    """None when every field the driver reads has a usable type, else the reason it does not."""
    if not isinstance(report, dict):
        return 'report is not an object'
    if type(report.get('prism_mem_report')) is not int or report['prism_mem_report'] != REPORT_VERSION:
        return 'report version is %r, expected %d' % (report.get('prism_mem_report'), REPORT_VERSION)
    stages = report.get('stages')
    if not isinstance(stages, list):
        return 'stages is not a list'
    for position, entry in enumerate(stages):
        if not isinstance(entry, dict) or not isinstance(entry.get('name'), str):
            return 'stages[%d] is not an object with a string name' % position
        reason = bad_counts(entry, 'stage %s' % entry['name'], STAGE_NUMBERS)
        if reason:
            return reason
    reason = bad_counts({'ru_maxrss_bytes': report.get('ru_maxrss_bytes')}, 'report', ('ru_maxrss_bytes',))
    if reason:
        return reason
    attribution = report.get('attribution')
    if not isinstance(attribution, dict):
        return 'report has no attribution section'
    reason = bad_counts(attribution, 'attribution', ATTRIBUTION_NUMBERS)
    if reason:
        return reason
    ratio = attribution.get('node_string_unique_ratio')
    # The range test comes first: it is False for NaN and infinity and does not convert a huge integer.
    if ratio is not None and not (is_number(ratio) and 0 <= ratio <= 1):
        return 'attribution.node_string_unique_ratio is %r, not a number from 0 to 1' % (ratio,)
    workload = report.get('workload')
    if workload is not None:
        if not isinstance(workload, dict):
            return 'workload is not an object'
        reason = bad_counts(workload, 'workload', WORKLOAD_NUMBERS)
        if reason:
            return reason
    if report.get('refresh') is not None and not isinstance(report['refresh'], dict):
        return 'refresh is not an object'
    return well_formed_headers(report)


def well_formed_headers(report):
    """The build, env, cache and threads sections."""
    build = report.get('build')
    if not isinstance(build, dict):
        return 'report has no build section'
    for key in BUILD_STRINGS:
        if key in build and not isinstance(build[key], str):
            return 'build.%s is %r, not a string' % (key, build[key])
    if not isinstance(build.get('build_identity'), str):
        return 'build.build_identity is missing'
    if 'binary_input_dirty' in build and not isinstance(build['binary_input_dirty'], bool):
        return 'build.binary_input_dirty is %r, not a boolean' % (build['binary_input_dirty'],)
    if 'cpg_cache_version' in build and not is_count(build['cpg_cache_version']):
        return 'build.cpg_cache_version is %r, not a non-negative integer' % (build['cpg_cache_version'],)
    env = report.get('env')
    if not isinstance(env, dict) or not all(isinstance(k, str) and isinstance(v, str) for k, v in env.items()):
        return 'env is not an object of strings'
    cache = report.get('cache')
    if not isinstance(cache, dict):
        return 'report has no cache section'
    for key in ('cpg_hit', 'call_edges_loaded'):
        if key not in cache or not (cache[key] is None or isinstance(cache[key], bool)):
            return 'cache.%s is missing or not true, false or null' % key
    threads = report.get('threads')
    if type(threads) is not int or threads < 1 or threads > U64_MAX:
        return 'threads is %r, not a positive integer' % (threads,)
    return None


def build_cache_reason(cache_files):
    """None when the stored sizes show a build wrote both cache files (integers above 0), else why not."""
    if not isinstance(cache_files, dict):
        return BUILD_CACHE_REASONS[2]
    for position, key in enumerate(('cpg_cache_bytes', 'call_edge_sidecar_bytes')):
        size = cache_files.get(key)
        if type(size) is not int or size <= 0:
            return BUILD_CACHE_REASONS[position]
    return None


def validate(kind, report, expected=None):
    """None when the report may be used as evidence for `kind`, else the reason it may not.

    `expected` (workload runs): {'requests': int, 'sha256': hex} of the workload this run was told to use.
    """
    if kind not in REQUIRED:
        return 'unknown kind %r' % kind
    reason = well_formed(report)
    if reason:
        return reason
    if kind in CACHED_KINDS:
        # Positive evidence recorded by the process itself: a run that merely printed no complaint
        # (a sidecar that is absent or undecodable prints nothing) is not a cache-loaded run.
        if report['cache']['cpg_hit'] is not True:
            return CACHE_REASONS[0]
        if report['cache']['call_edges_loaded'] is not True:
            return CACHE_REASONS[1]
    if kind == BUILD:
        for position, key in enumerate(('cpg_hit', 'call_edges_loaded')):
            if report['cache'][key] is not False:
                return BUILD_REASONS[position if report['cache'][key] is True else position + 2]
    for stage_name, field in REQUIRED[kind]:
        entry = find_stage(report, stage_name)
        if entry is None:
            return 'stage %s is missing' % stage_name
        if field == FOOTPRINT:
            if not is_number(footprint(entry)):
                return '%s.phys_footprint_bytes and %s.rss_bytes are unavailable' % (stage_name, stage_name)
        elif not is_number(entry.get(field)):
            return '%s.%s is unavailable' % (stage_name, field)
    if kind in ('refresh', 'changed-refresh'):
        refresh = report.get('refresh')
        if not isinstance(refresh, dict):
            return 'report has no refresh section'
        if refresh.get('status') != 'refreshed':
            return 'refresh status is %r, not \'refreshed\' (the tree changed during the refresh)' % (
                refresh.get('status'),)
    if kind == 'changed-refresh':
        return validate_changed(report, expected)
    if kind == 'workload':
        return validate_workload(report, expected)
    return None


def validate_workload(report, expected):
    workload = report.get('workload')
    if not isinstance(workload, dict):
        return 'report has no workload section'
    requests, responses, errors = (workload.get(key) for key in ('requests', 'responses', 'errors'))
    if not all(is_number(value) for value in (requests, responses, errors)):
        return 'workload requests/responses/errors are unavailable'
    if errors != 0:
        return 'workload had %d errors' % errors
    if requests == 0:
        return 'workload ran no requests'
    if responses != requests:
        return 'workload answered %d of %d requests' % (responses, requests)
    if expected is not None:
        if requests != expected['requests']:
            return 'workload ran %d requests, the manifest has %d' % (requests, expected['requests'])
        if workload.get('sha256') != expected['sha256']:
            return 'workload sha256 %s differs from the manifest %s' % (workload.get('sha256'), expected['sha256'])
    return None
