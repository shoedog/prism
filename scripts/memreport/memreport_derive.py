"""The per-run quantities the readout is built from (imported by memreport.py)."""
from memreport_validate import find_stage, footprint, is_number


def num(mapping, key):
    """mapping[key] when it is a number, else None (absent, null or malformed)."""
    value = mapping.get(key) if isinstance(mapping, dict) else None
    return value if is_number(value) else None


def difference(high, low):
    return None if high is None or low is None else high - low


def index_ms(report):
    """Time from the end of `load_repo` to the end of the index, ms; None when either time is unavailable.

    For a cache-loaded run this is the index load, for a first build the index build.
    """
    return difference(num(find_stage(report, 'after_index'), 'elapsed_ms'),
                      num(find_stage(report, 'after_load_repo'), 'elapsed_ms'))


def derive(kind, report):
    """The per-run quantities the readout is built from (bytes). A value that is unavailable is None.

    Never raises for a report that `validate(kind, report)` accepted.
    """
    if kind == 'warm':
        loaded, built = find_stage(report, 'after_load_repo'), find_stage(report, 'after_index')
        overshoot = built['peak_bytes'] - max(loaded['peak_bytes'], footprint(built))
        return {'peak': built['peak_bytes'], 'm2_observed_overshoot': max(0, overshoot),
                'index_heap': difference(num(built, 'heap_in_use_bytes'), num(loaded, 'heap_in_use_bytes')),
                'load_ms': index_ms(report)}
    if kind == 'build':
        return {'build_peak': find_stage(report, 'after_call_edges')['peak_bytes'], 'build_ms': index_ms(report)}
    if kind == 'cold':
        return {'cold_peak': find_stage(report, 'after_call_edges')['peak_bytes']}
    if kind == 'refresh':
        return {'refresh_peak': find_stage(report, 'after_refresh')['peak_bytes']}
    if kind == 'workload':
        attribution, settled = report['attribution'], find_stage(report, 'after_settle')
        return {
            'steady': footprint(settled),
            'steady_heap': num(settled, 'heap_in_use_bytes'),
            'm1': num(settled, 'tree_sitter_heap_bytes'),
            'm3': difference(num(attribution, 'node_string_heap_bytes_estimate'),
                             num(attribution, 'node_unique_string_heap_bytes_estimate')),
        }
    raise ValueError('unknown kind %r' % kind)
