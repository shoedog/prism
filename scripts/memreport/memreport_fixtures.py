"""Canned reports and a fake prism-mcp for the memreport tests (shared by the test modules)."""
import hashlib
import json
import stat
from pathlib import Path

MIB = 1024 * 1024

ATTRIBUTION = {
    'files': 10, 'source_bytes': 4 * MIB, 'tree_visible_nodes': 1000, 'cpg_nodes': 500, 'cpg_edges': 900,
    'cpg_node_inline_bytes': 20 * MIB, 'node_strings': 300, 'node_string_bytes': 3000,
    'node_string_heap_bytes_estimate': 100 * MIB, 'node_unique_strings': 100, 'node_unique_string_bytes': 1000,
    'node_unique_string_heap_bytes_estimate': 40 * MIB, 'node_string_unique_ratio': 0.4,
}


def stage(name, footprint, peak, rss=None, heap=None, tree=None):
    return {'name': name, 'elapsed_ms': 0, 'rss_bytes': rss if rss is not None else footprint,
            'phys_footprint_bytes': footprint, 'compressed_bytes': 0, 'peak_bytes': peak,
            'heap_in_use_bytes': heap, 'tree_sitter_heap_bytes': tree}


def report(stages, attribution=None, workload=None, refresh=None, version=2, env=None, build=None):
    return {'prism_mem_report': version, 'build': build or {'build_identity': 'b', 'binary_input_dirty': False},
            'platform': {'os': 'macos'}, 'config': {}, 'env': env if env is not None else {},
            'stages': stages, 'ru_maxrss_bytes': 1, 'attribution': attribution or {},
            'workload': workload, 'refresh': refresh}


def workload_section(sha256='f' * 64, requests=8, responses=None, errors=0):
    return {'requests': requests, 'responses': requests if responses is None else responses,
            'errors': errors, 'response_bytes': 10, 'sha256': sha256}


def canned(kind, sha256='f' * 64, requests=8, heap=900 * MIB, tree=300 * MIB):
    """A report that validates for `kind`."""
    if kind == 'warm':
        return report([stage('after_load_repo', 100 * MIB, 120 * MIB, heap=50 * MIB),
                       stage('after_index', 400 * MIB, 700 * MIB, heap=350 * MIB)])
    if kind == 'cold':
        return report([stage('after_call_edges', 500 * MIB, 800 * MIB)])
    if kind == 'refresh':
        return report([stage('after_refresh', 500 * MIB, 900 * MIB)], refresh={'status': 'refreshed'})
    if kind == 'workload':
        return report([stage('after_settle', 1000 * MIB, 1200 * MIB, heap=heap, tree=tree)],
                      attribution=dict(ATTRIBUTION), workload=workload_section(sha256, requests))
    raise ValueError(kind)


def emit(report_dict, before=''):
    """Fake-binary body that prints `before` lines, then the report line, on stderr."""
    return ('%sprint(json.dumps(json.loads(%r)), file=sys.stderr)\n'
            % (before, json.dumps(report_dict)))


def noise(text):
    return 'print(%r, file=sys.stderr)\n' % text


def fake_binary(directory, body, name='fake-prism-mcp'):
    """A stand-in for prism-mcp: a Python script that runs `body`."""
    path = Path(directory) / name
    path.write_text('#!/usr/bin/env python3\nimport json, sys, time\n' + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def digest(data):
    return hashlib.sha256(data).hexdigest()
