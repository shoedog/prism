import json
from pathlib import Path

import pytest

from schema_impact import config, consumer_ts
from schema_impact.cli import main


FIXTURE = Path(__file__).parent / "fixtures" / "ts_mini"
CONFIG = FIXTURE / "config.toml"


@pytest.mark.parametrize(
    ("expression", "expected"),
    [
        ("'/a/file'", ("/a/file", "literal")),
        ("`/a/item/${id}`", ("/a/item/{}", "template")),
        ("`/a/similar?title=${title}`", ("/a/similar", "literal")),
        ("`/a/item-${id}`", (None, "dynamic")),
        ("`${base}/x`", (None, "dynamic")),
        ("`${apiUrl}?username=${u}`", ("/a/info", "literal")),
        ("`${qaUrl}${u}`", ("/a/top", "literal")),
        ("'/a/' + id", ("/a/{}", "template")),
        ("'/a/' + (offset + 1) + '/last'", ("/a/{}/last", "template")),
        ("'/a/' + id + suffix", (None, "dynamic")),
        ("id ? `/a/${id}` : null", ("/a/{}", "template")),
        ("id ? false : ['/a/file', params]", ("/a/file", "literal")),
        ("id ? '' : '/a/file'", ("/a/file", "literal")),
        ("id ? undefined : '/a/file'", ("/a/file", "literal")),
        ("c ? '/a' : '/b'", (None, "conditional")),
        ("[apiUrl, { params: { action } }]", ("/a/info", "literal")),
        ("[makeKey(), params]", (None, "dynamic")),
        ("[, '/later']", (None, "dynamic")),
        ("[`/a/${item.id}`, params]", ("/a/{}", "template")),
        ("('/a//plain/')", ("/a/plain", "literal")),
        ("'/a/%2f/#literal'", ("/a/%2f/#literal", "literal")),
    ],
)
def test_url_classes(expression, expected):
    assert consumer_ts.pattern(expression, {"apiUrl": "/a/info", "qaUrl": "/a/top?username="}) == expected


@pytest.fixture
def sites():
    extracted = {}
    for site in consumer_ts.calls(FIXTURE, config.load(CONFIG), "src"):
        source = (FIXTURE / site.file).read_text().splitlines()[site.line - 1]
        marker = source.split("// ", 1)[-1]
        assert marker not in extracted, (marker, site)
        extracted[marker] = site
    return extracted


def test_calls_fixture(sites):
    expected = {
        "CONST_URL": ("GET", "/a/list"), "RELATIVE": ("GET", "/a/items"),
        "INSTANCE_PUT": ("PUT", "/a/items/{}"), "OLD_ALIAS": ("GET", "/a/old"),
        "POSITIONAL_METHOD": ("PATCH", "/a/other"), "FIXED_METHOD": ("POST", "/chosen"),
        "OPTIONS_METHOD": ("DELETE", "/chosen"), "FETCH": ("GET", "/global"),
        "FETCH_POST": ("POST", "/global"), "AXIOS_VERB": ("PATCH", "/chosen"),
        "AXIOS_CONFIG": ("POST", "/chosen"), "AXIOS_DEFAULT": ("GET", "/chosen"),
        "AXIOS_CHAIN": ("DELETE", "/chosen"), "WINDOW_FETCH": ("GET", "/global"),
        "LOCAL_CONST": ("GET", "/a/local"), "KEPT_AFTER_BLOCK": ("GET", "/a/retained"),
        "LATER_MODULE_CONST": ("GET", "/a/later"),
        "NAMESPACE": ("GET", "/a/namespace"), "AXIOS_NAMED": ("GET", "/wire"),
        "CALLBACK": ("GET", "/a/item"), "HOOK": ("GET", "/a/item"),
        "POST_FETCHER_HOOK": ("POST", "/a/item"), "INLINE_PUT_HOOK": ("PUT", "/a/item"),
        "HOOK_WITHOUT_FETCHER_SPEC": ("GET", "/legacy/item"),
    }
    unbound = {
        "UNKNOWN_METHOD": "unknown_client_method", "UNKNOWN_DEEP_METHOD": "unknown_client_method",
        "DOTTED_PROPERTY": "unknown_client_method", "OUTER_FUNCTION_CONST": "dynamic_url",
        "OPTIONS_OBJECT_URL": "unconfigured_client_form", "CALLBACK_URL": "unconfigured_client_form",
        "UNCONFIGURED": "unconfigured_client_form", "UNCONFIGURED_EXPORT": "unconfigured_client_form",
        "POSITIONAL_DYNAMIC": "dynamic_method", "OPTIONS_DYNAMIC": "dynamic_method",
        "FETCH_DYNAMIC_INIT": "dynamic_method", "FETCH_MISSING_METHOD": "dynamic_method",
        "AXIOS_DYNAMIC": "dynamic_method", "SHADOWED_CONST": "dynamic_url",
        "REASSIGNED_CONST": "dynamic_url", "NON_STRING_KEY": "non_string_key",
        "ARRAY_NON_STRING_KEY": "non_string_key", "CONDITIONAL_URL": "conditional_url",
        "DYNAMIC_FETCHER_METHOD": "dynamic_method", "AXIOS_FACTORY": "unconfigured_client_form",
        "AXIOS_INLINE_FACTORY": "unconfigured_client_form",
    }
    assert sites.keys() == expected.keys() | unbound.keys()
    for marker, values in expected.items():
        assert (sites[marker].method, sites[marker].pattern) == values
        assert sites[marker].reason is None
    for marker, reason in unbound.items():
        assert sites[marker].reason == reason
    assert {site.url_class for site in sites.values()} == {"literal", "template", "dynamic", "conditional"}
    assert sites["CONST_URL"].response_type == sites["HOOK"].response_type == "Type.Item"
    assert sites["RELATIVE"].response_type is None
    assert sites["CALLBACK"].enclosing_function is None
    assert sites["HOOK"].enclosing_function == "Widget"
    assert sites["LOCAL_CONST"].enclosing_function == "local"
    assert sites["CALLBACK"].file == "src/Widget.tsx"


def test_config_loads_nested_methods_and_hooks():
    loaded = config.load(CONFIG)
    assert loaded.aliases["@/"] == "src/"
    assert isinstance(loaded.clients, list)
    request = loaded.clients[0]
    assert (request.file, request.export, request.response_root) == ("src/request.ts", "default", "data")
    assert request.methods["instance.put"] == "PUT"
    assert request.methods["via"] == {"method_arg": 1}
    hook = loaded.clients[-1]
    assert (hook.hook, hook.url_arg, hook.fetcher_arg) == ("useSWR", 0, 1)


def test_consumer_cli_writes_accounted_evidence(tmp_path):
    schema = tmp_path / "schema.json"
    schema.write_text(json.dumps({
        "swagger": "2.0",
        "paths": {
            "/a/items/{id}": {"put": {"summary": "Replace an item"}},
            "/a/item": {"get": {"summary": "Read one item"}, "post": {"summary": "Create an item"}},
            "/a/list": {"get": {"summary": "List items"}},
            "/global": {"get": {"summary": "Global reader"}},
        },
    }))
    out = tmp_path / "evidence"
    assert main(["consumer", "--repo", str(FIXTURE), "--root", "src", "--schema",
                 str(schema), "--config", str(CONFIG), "--out", str(out)]) == 0
    counts = json.loads((out / "f2.json").read_text())
    rows = [json.loads(row) for row in (out / "consumer.jsonl").read_text().splitlines()]
    assert len(rows) == counts["denominator"]
    assert sum(counts["url_class"].values()) == len(rows)
    assert counts["unique"] + counts["several"] + counts["none"] == len(rows)
    assert sum(counts["none_reasons"].values()) == counts["none"]
    assert counts["all_unique_string_equal"] is False  # a template equality still uses a schema parameter
    sample = [json.loads(row) for row in (out / "f2-sample.jsonl").read_text().splitlines()]
    assert len(sample) == counts["unique"]
    assert sample == sorted(sample, key=lambda row: (row["file"], row["line"]))
    entry = next(row for row in sample if row["operation_id"] == "op:GET /a/list")
    assert entry["source_line"] == "fromBarrel.get<Type.Item>(list); // CONST_URL"
    assert entry["schema_summary"] == "List items"
    assert "List items" in (out / "f2-sample.md").read_text()
