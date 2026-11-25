import json
from pathlib import Path

from schema_impact import outbound_http, producer_go
from schema_impact.cli import main


GO_MINI = Path(__file__).with_name("fixtures") / "go_mini"


def read_jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def test_annotations_attach_only_to_go_declarations():
    bindings = producer_go.annotations(GO_MINI)
    assert [
        (b.element, b.file, b.line, b.function, b.receiver_type, b.via) for b in bindings
    ] == [
        ("op:GET /api/x", "handlers.go", 6, "Handler", "Controller", "swaggo_annotation"),
        ("op:GET /api/a", "handlers.go", 10, "List", None, "swaggo_annotation"),
        ("op:POST /api/b/{}", "handlers.go", 12, "List", None, "swaggo_annotation"),
        ("op:HEAD /outside", "handlers.go", 19, "Outside", None, "swaggo_annotation"),
    ]
    sources = {path.name: path.read_text() for path in GO_MINI.glob("*.go")}
    sources["notes.txt"] = sources["handlers.go"]
    assert producer_go.annotations_from_sources(sources) == bindings


def test_registrations_accept_literals_and_use_the_last_handler_identifier():
    assert [
        (r.method, r.path_suffix, r.handler_name, r.file, r.line)
        for r in producer_go.registrations(GO_MINI)
    ] == [
        ("GET", "/x", "Handler", "routes.go", 6),
        ("GET", "/x/{}", "h", "routes.go", 7),
        ("HEAD", "/outside", "Outside", "routes.go", 12),
    ]


def test_go_source_positions_and_literal_value_use_go_line_breaks(tmp_path):
    source = (
        "package example\r\n// separator \u2028 in a comment\r\n"
        "// @Router /x [get]\r\nfunc Example() { g.GET(`/\rx`, Example) }\r\n"
    )
    (tmp_path / "example.go").write_bytes(source.encode())
    assert producer_go.annotations(tmp_path)[0].line == 3
    assert producer_go.registrations(tmp_path) == [
        producer_go.Registration("GET", "/x", "Example", "example.go", 4),
    ]


def test_outbound_http_patterns_and_test_exclusions(tmp_path):
    assert outbound_http.count(GO_MINI) == {"http.Get": 1}
    (tmp_path / "worker.go").write_text("""
http.Get(url)
http.Post(url)
http.PostForm(url)
http.Head(url)
http.NewRequest(method, url)
http.NewRequestWithContext(ctx, method, url)
proxy.apiCLIENT.Do (req)
transport.Do(req)
myhttp.Post(url)
""")
    (tmp_path / "worker.py").write_text("""
requests.get(url)
requests.get(url)
requests.request(method, url)
httpx.patch(url)
httpx.Client()
httpx.AsyncClient()
urllib.request.urlopen(url)
aiohttp.ClientSession()
other_requests.get(url)
""")
    (tmp_path / "test_worker.py").write_text("requests.get(url)\n")
    (tmp_path / "worker_test.py").write_text("requests.get(url)\n")
    (tmp_path / "worker_test.go").write_text("http.Get(url)\n")
    assert outbound_http.count(tmp_path) == {
        "http.Get": 1, "http.Post": 1, "http.PostForm": 1, "http.Head": 1,
        "http.NewRequest": 1, "http.NewRequestWithContext": 1, "apiCLIENT.Do": 1,
        "requests.get": 2, "requests.request": 1, "httpx.patch": 1, "httpx.Client": 1,
        "httpx.AsyncClient": 1, "urllib.request.urlopen": 1, "aiohttp.ClientSession": 1,
    }


def test_producer_cli_writes_counts_and_source_evidence(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "handlers.go").write_text("""package example
// @Router /api/one [get]
func One() {}
// @Router /api/two [get]
// @Router /api/two [get]
func Two() {}
// @Router /api/multi [get]
func First() {}
// @Router /api/multi [get]
func Second() {}
// @Router /api/stale [post]
func Drift() {}
// @Router /api/never [get]
func Never() {}
// @Router /api/unlisted [get]
func Unlisted() {}
func routes() {
    g.GET("/one", Other)
    g.GET("/one", guard(), One)
    g.GET("/two", Two)
    g.GET("/multi", First)
    g.GET("/stale", Drift)
    g.GET("/same", Other)
    g.Any("/wild", Other)
    g.Handle(method, path, Other)
    g.Match(methods, path, Other)
    http.Get(url)
    g.GET(prefix + "/never", Never)
}
""")
    schema = repo / "schema.json"
    paths = {
        "/api/one": {"get": {}}, "/api/two": {"get": {}}, "/api/multi": {"get": {}},
        "/api/stale": {"post": {}}, "/api/never": {"get": {}}, "/api/absent": {"get": {}},
        "/first/same": {"get": {}}, "/second/same": {"get": {}},
    }
    schema.write_text(json.dumps({"swagger": "2.0", "paths": paths}, indent=2))
    out = tmp_path / "out"
    assert main(["producer", "--repo", str(repo), "--schema", "schema.json", "--out", str(out)]) == 0
    f1 = json.loads((out / "f1.json").read_text())
    assert f1["operations"] == {
        "total": 8, "exactly_one_function": 4, "no_function": 3, "several_functions": 1,
    }
    assert f1["annotations_without_operation"] == 1
    assert f1["registrations"] == 6
    assert f1["cross_check"] == {"population": 2, "agreements": 1, "disagreements": 1}
    assert f1["unextracted_registration"] == {"Any": 1, "Handle": 1, "Match": 1}
    assert len(read_jsonl(out / "producer.jsonl")) == 8
    assert json.loads((out / "outbound-http.json").read_text()) == {"http.Get": 1}

    rows = read_jsonl(out / "f1-residue.jsonl")
    assert f1["residue"] == rows
    assert len({row["row_id"] for row in rows}) == len(rows) == 6
    assert {row["element"] for row in rows if row["list"] == 2} == {
        "op:GET /api/never", "op:GET /api/absent", "op:POST /api/stale",
    }
    absent = next(row for row in rows if row.get("element") == "op:GET /api/absent")
    assert absent["file"] == "schema.json" and absent["function"] is None
    assert '\"get\": {' in absent["evidence"]["schema"][0]
    never = next(row for row in rows if row.get("element") == "op:GET /api/never")
    assert never["evidence"]["registration"] == []
    assert never["evidence"]["nonliteral_registration"] == ['    g.GET(prefix + "/never", Never)']
    assert never["nonliteral_registration_sites"][0]["file"] == "handlers.go"
    disagreement = next(row for row in rows if row["list"] == "cross_check")
    assert disagreement["file"] == "handlers.go" and disagreement["line"] == 18
    assert disagreement["function"] == "Other"
    assert disagreement["evidence"]["annotation"] == ["// @Router /api/one [get]"]
    assert disagreement["evidence"]["registration"] == ['    g.GET("/one", Other)']
    method = next(row for row in rows if row["list"] == 4)
    assert method["element"] == "op:POST /api/stale"
    assert method["evidence"]["registration"] == ['    g.GET("/stale", Drift)']
    assert method["evidence"]["annotation"] == ["// @Router /api/stale [post]"]
    unmatched = next(row for row in rows if row["list"] == 3)
    assert unmatched["registration"] == '    g.GET("/stale", Drift)'


def test_producer_cli_accepts_schema_with_merged_yaml_paths(tmp_path):
    schema = tmp_path / "schema.yaml"
    schema.write_text("""\
swagger: '2.0'
x-paths: &shared
  /merged:
    get: {}
paths:
  <<: *shared
""")
    out = tmp_path / "out"
    assert main(["producer", "--repo", str(tmp_path), "--schema", str(schema), "--out", str(out)]) == 0
    row, = read_jsonl(out / "f1-residue.jsonl")
    assert row["element"] == "op:GET /merged" and row["file"] == "schema.yaml"
