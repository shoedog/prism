import pytest

from tier_a.lsp_client import LspServerError
from tier_a.model import FunctionDef, Location
from tier_a.oracles import (LspOracle, OracleError, enrich_definitions,
                            map_document_symbols, map_incoming, map_outgoing,
                            uri_to_rel)


def test_uri_to_rel_posix():
    assert uri_to_rel("file:///repo/src/a%20b.rs", "/repo") == "src/a b.rs"
    assert uri_to_rel("file:///elsewhere/x.rs", "/repo") is None


def test_map_document_symbols_hierarchical_kinds_container_selection():
    syms = [{
        "name": "Engine", "kind": 5,
        "range": {"start": {"line": 9, "character": 0},
                  "end": {"line": 29, "character": 1}},
        "selectionRange": {"start": {"line": 9, "character": 6},
                           "end": {"line": 9, "character": 12}},
        "children": [{
            "name": "run", "kind": 6,
            "range": {"start": {"line": 11, "character": 4},
                      "end": {"line": 19, "character": 5}},
            "selectionRange": {"start": {"line": 11, "character": 8},
                               "end": {"line": 11, "character": 11}},
        }],
    }, {
        "name": "helper", "kind": 12,
        "range": {"start": {"line": 31, "character": 0},
                  "end": {"line": 33, "character": 1}},
        "selectionRange": {"start": {"line": 31, "character": 3},
                           "end": {"line": 31, "character": 9}},
    }]
    fds = map_document_symbols("src/x.py", syms)
    assert [f.name for f in fds] == ["run", "helper"]
    run = fds[0]
    assert (run.kind, run.container) == ("method", "Engine")
    assert run.location == Location("src/x.py", 12, 20)
    assert run.selection_line == 12
    assert run.selection_char == 8   # raw 0-based, fed back into prepareCallHierarchy
    assert fds[1].container is None


def test_map_document_symbols_flat_symbol_information_fallback():
    syms = [{
        "name": "build", "kind": 9, "containerName": "Engine",
        "location": {
            "uri": "file:///repo/src/x.py",
            "range": {"start": {"line": 20, "character": 4},
                      "end": {"line": 25, "character": 5}},
        },
    }]
    assert map_document_symbols("src/x.py", syms) == [
        FunctionDef("build", "constructor", "Engine",
                    Location("src/x.py", 21, 26), 21)
    ]


def test_map_incoming_one_edge_per_from_range():
    seed = FunctionDef("f", "function", None, Location("src/lib.rs", 5, 9), 5)
    items = [{
        "from": {
            "name": "caller_a", "uri": "file:///repo/src/m.rs",
            "range": {"start": {"line": 99, "character": 0},
                      "end": {"line": 120, "character": 1}},
            "selectionRange": {"start": {"line": 99, "character": 3},
                               "end": {"line": 99, "character": 11}},
        },
        "fromRanges": [
            {"start": {"line": 101, "character": 8},
             "end": {"line": 101, "character": 9}},
            {"start": {"line": 110, "character": 8},
             "end": {"line": 110, "character": 9}},
        ],
    }]
    edges = map_incoming(seed, items, root="/repo")
    assert len(edges) == 2
    assert edges[0].direction == "caller"
    assert edges[0].other_name == "caller_a"
    assert edges[0].other_def == Location("src/m.rs", 100, 121)
    assert {e.call_site.start_line for e in edges} == {102, 111}
    assert all(e.call_site.file == "src/m.rs" for e in edges)


def test_map_outgoing_call_sites_are_in_seed_file():
    seed = FunctionDef("f", "function", None, Location("src/lib.rs", 5, 30), 5)
    items = [{
        "to": {
            "name": "callee_x", "uri": "file:///repo/src/n.rs",
            "range": {"start": {"line": 3, "character": 0},
                      "end": {"line": 7, "character": 1}},
            "selectionRange": {"start": {"line": 3, "character": 3},
                               "end": {"line": 3, "character": 11}},
        },
        "fromRanges": [{"start": {"line": 12, "character": 4},
                        "end": {"line": 12, "character": 12}}],
    }]
    edges = map_outgoing(seed, items, root="/repo")
    assert len(edges) == 1
    assert edges[0].direction == "callee"
    assert edges[0].call_site == Location("src/lib.rs", 13, 13)
    assert edges[0].other_def == Location("src/n.rs", 4, 8)


def test_enrich_definitions_by_containment_smallest_span():
    inv = [
        FunctionDef("outer", "function", None, Location("src/p.rs", 1, 50), 1),
        FunctionDef("target", "method", "Edge", Location("src/p.rs", 10, 14), 10),
    ]
    raw = [{
        "uri": "file:///repo/src/p.rs",
        "range": {"start": {"line": 11, "character": 4},
                  "end": {"line": 11, "character": 10}},
    }]
    [dt] = enrich_definitions(raw, inv, root="/repo")
    assert (dt.name, dt.kind) == ("target", "method")


class FakeRawClient:
    def __init__(self, replies):
        self.replies = replies
        self.requests = []
        self.notifications = []
        self.server_info = {}
        self.stopped = False

    def request(self, method, params, timeout=None):
        self.requests.append((method, params, timeout))
        reply = self.replies[method]
        if isinstance(reply, Exception):
            raise reply
        return reply(params) if callable(reply) else reply

    def notify(self, method, params):
        self.notifications.append((method, params))

    def drain_notifications(self):
        return []

    def stop(self):
        self.stopped = True


def fake_oracle(replies):
    oracle = LspOracle(["fake-lsp"], "/repo", "python")
    oracle.client = FakeRawClient(replies)
    return oracle


def test_callers_raises_oracle_error_for_lsp_error():
    seed = FunctionDef("f", "function", None, Location("src/lib.py", 5, 9), 5)
    oracle = fake_oracle({
        "textDocument/prepareCallHierarchy": [{
            "name": "f", "uri": "file:///repo/src/lib.py",
            "range": {"start": {"line": 4, "character": 0},
                      "end": {"line": 8, "character": 0}},
            "selectionRange": {"start": {"line": 4, "character": 4},
                               "end": {"line": 4, "character": 5}},
        }],
        "callHierarchy/incomingCalls": LspServerError({
            "code": -32603,
            "message": "boom",
        }),
    })
    with pytest.raises(OracleError, match="callHierarchy/incomingCalls"):
        oracle.callers(seed)


def test_hierarchy_item_raises_oracle_error_for_empty_prepare_result():
    seed = FunctionDef("f", "function", None, Location("src/lib.py", 5, 9), 5)
    oracle = fake_oracle({"textDocument/prepareCallHierarchy": []})
    with pytest.raises(OracleError, match="no item"):
        oracle._hierarchy_item(seed)


def test_lsp_oracle_stop_delegates_to_client():
    oracle = fake_oracle({})
    oracle.stop()
    assert oracle.client.stopped


def test_gopls_receiver_names_normalized():
    # gopls: method name carries the receiver, container is None
    syms = [{
        "name": "(*AdminConfig).newAdminHandler", "kind": 6,
        "range": {"start": {"line": 4, "character": 0}, "end": {"line": 9, "character": 1}},
        "selectionRange": {"start": {"line": 4, "character": 22}, "end": {"line": 4, "character": 36}},
    }]
    [fd] = map_document_symbols("admin.go", syms)
    assert fd.name == "newAdminHandler"
    assert fd.container == "AdminConfig"
