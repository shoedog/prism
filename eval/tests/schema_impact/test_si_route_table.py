import json
from pathlib import Path

from schema_impact.cli import main
from schema_impact.producer_route_table import HandlerSite, RouteSpec, extract, routes


MINI = Path(__file__).with_name("fixtures") / "route_table_mini"
SPEC = RouteSpec(
    callee="register",
    method_key="verb",
    path_key="pattern",
    handler_key="target",
    validation_key="check",
    mount="/garden/",
)


def test_public_route_table_fixture():
    rows = routes(MINI, SPEC)

    assert isinstance(rows, list)
    assert [(row.method, row.path) for row in rows] == [
        ("GET", "/garden/blooms/{}"),
        ("POST", "/garden/blooms"),
        ("DELETE", "/garden/blooms/{}"),
    ]
    assert [(row.file, row.line) for row in rows] == [
        ("blooms.js", 8), ("blooms.js", 9), ("blooms.js", 10),
    ]
    assert [row.handler_text for row in rows] == [
        "pickBloom", "chooseBloom", "garden.missing",
    ]
    assert [row.handler_site for row in rows] == [
        HandlerSite("blooms.js", 3, "pickBloom"),
        HandlerSite("press.js", 1, "pressBloom"),
        None,
    ]
    assert [row.has_validation for row in rows] == [False, True, False]
    assert not any(row.optional_param for row in rows)


def test_literal_array_arguments_and_normalization(tmp_path):
    (tmp_path / "entries.js").write_text(r"""
const prefix = "/dynamic";
const verbs = ["get"];
register("unused", [
  { "verb": ["head", /* method */ "*", "get",], pattern: "//fern/{tip?}.json/" },
  { verb: "*", pattern: "/" },
  { verb: "get", pattern: "/Fern/\u0074ip/%2f/" },
  { verb: ["get", prefix], pattern: "/mixed" },
  { verb: ["get", , "post"], pattern: "/sparse" },
  { verb: "get", pattern: prefix },
  { verb: verbs, pattern: "/nonliteral-method" },
  { verb: "get", pattern: `/template` },
  [{ verb: "get", pattern: "/nested-array" }],
]);
server.other([{ verb: "get", pattern: "/different-callee" }]);
server.register({ routes: [{ verb: "get", pattern: "/nested-argument" }] });
""", encoding="utf-8")

    result = extract(tmp_path, SPEC)
    assert [(row.method, row.path, row.optional_param) for row in result.routes] == [
        ("HEAD", "/garden/fern/{}.json", True),
        ("*", "/garden/fern/{}.json", True),
        ("GET", "/garden/fern/{}.json", True),
        ("*", "/garden", False),
        ("GET", "/garden/Fern/tip/%2f", False),
    ]
    assert result.unextracted == [{"method": "*", "count": 2}]
    assert result.unparsed == []


def test_handler_and_validation_nesting_have_independent_order(tmp_path):
    (tmp_path / "nesting.js").write_text("""
function top() {}
const later = () => {};
const group = { open() {} };
server.register([
  {verb: "get", pattern: "/top", target: top, options: {target: later, check: null}},
  {verb: "get", pattern: "/option", options: {target: group.open}, config: {target: top}},
  {verb: "get", pattern: "/config", config: {target: top, check: {}}},
  {verb: "get", pattern: "/none", options: getOptions()},
]);
""".lstrip(), encoding="utf-8")
    spec = RouteSpec(
        callee="register", method_key="verb", path_key="pattern",
        handler_key="target", validation_key="check",
        handler_under=("options", "", "config"), validation_under=("options",),
    )

    rows = routes(tmp_path, spec)
    assert [(row.handler_text, row.handler_site, row.has_validation) for row in rows] == [
        ("later", HandlerSite("nesting.js", 2, "later"), True),
        ("group.open", HandlerSite("nesting.js", 3, "open"), False),
        ("top", HandlerSite("nesting.js", 1, "top"), False),
        (None, None, False),
    ]


def test_named_import_resolution_checks_exports_and_repo_boundary(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    (tmp_path / "outside.js").write_text("export function leak() {}", encoding="utf-8")
    (repo / "alias.js").symlink_to(tmp_path / "outside.js")
    (repo / "esm.mjs").write_text("""
function hidden() {}
function shown() {}
export {shown as published};
export const group = { work: () => {} };
""".lstrip(), encoding="utf-8")
    (repo / "cjs").mkdir()
    (repo / "cjs" / "index.cjs").write_text("""
function keep() {}
module.exports = { pick: keep, other() {} };
""".lstrip(), encoding="utf-8")
    (repo / "route.js").write_text("""
import {published as named, hidden, group} from "./esm.mjs";
import {leak} from "../outside.js";
import {leak as viaLink} from "./alias.js";
const {pick: chosen, other} = require("./cjs");
register([
  {verb: "get", pattern: "/named", target: named},
  {verb: "get", pattern: "/member", target: group.work},
  {verb: "get", pattern: "/chosen", target: chosen},
  {verb: "get", pattern: "/other", target: other},
  {verb: "get", pattern: "/hidden", target: hidden},
  {verb: "get", pattern: "/outside", target: leak},
  {verb: "get", pattern: "/symlink", target: viaLink},
  {verb: "get", pattern: "/unrelated", target: stranger.named},
]);
""", encoding="utf-8")

    assert [row.handler_site for row in routes(repo, SPEC)] == [
        HandlerSite("esm.mjs", 2, "shown"),
        HandlerSite("esm.mjs", 4, "work"),
        HandlerSite("cjs/index.cjs", 1, "keep"),
        HandlerSite("cjs/index.cjs", 2, "other"),
        None, None, None, None,
    ]


def test_handler_sites_respect_lexical_shadowing(tmp_path):
    (tmp_path / "scopes.js").write_text("""
function pick() {}
function install(pick) {
  function inner() {}
  register([
    {verb: "get", pattern: "/inner", target: inner},
    {verb: "get", pattern: "/parameter", target: pick},
    {verb: "get", pattern: "/inline", target: () => {}},
  ]);
}
{
  const pick = obtainHandler();
  register([{verb: "get", pattern: "/block", target: pick}]);
}
register([
  {verb: "get", pattern: "/outer", target: pick},
  {verb: "get", pattern: "/unknown", target: inner},
]);
""".lstrip(), encoding="utf-8")

    assert [row.handler_site for row in routes(tmp_path, SPEC)] == [
        HandlerSite("scopes.js", 3, "inner"),
        None, None, None,
        HandlerSite("scopes.js", 1, "pick"),
        None,
    ]


def test_class_member_resolves_only_static_methods(tmp_path):
    (tmp_path / "classes.js").write_text("""
class Basket {
  static gather() {}
  place() {}
  static get unavailable() {}
}
register([
  {verb: "get", pattern: "/static", target: Basket.gather},
  {verb: "get", pattern: "/instance", target: Basket.place},
  {verb: "get", pattern: "/getter", target: Basket.unavailable},
]);
""".lstrip(), encoding="utf-8")

    assert [row.handler_site for row in routes(tmp_path, SPEC)] == [
        HandlerSite("classes.js", 2, "gather"), None, None,
    ]


def test_routes_cli_writes_rows_and_diagnostics(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "broken.js").write_text("""
register([{verb: "get", pattern: "/ignore-entire-file"}]);
function broken(
""", encoding="utf-8")
    (repo / "ok.cjs").write_text("""
const send = function () {};
register([{verb: ["get", "*"], pattern: "/{name?}", target: send, check: {}}]);
""".lstrip(), encoding="utf-8")
    spec = tmp_path / "spec.json"
    spec.write_text(json.dumps({
        "callee": "register", "method_key": "verb", "path_key": "pattern",
        "handler_key": "target", "validation_key": "check", "mount": "/api",
        "handler_under": [""],
    }), encoding="utf-8")
    out = tmp_path / "out"

    assert main(["routes", "--repo", str(repo), "--spec", str(spec), "--out", str(out)]) == 0
    rows = [json.loads(line) for line in (out / "routes.jsonl").read_text().splitlines()]
    assert rows == [
        {
            "method": method, "path": "/api/{}", "handler_text": "send",
            "handler_site": {"file": "ok.cjs", "line": 1, "name": "send"},
            "has_validation": True, "file": "ok.cjs", "line": 2, "optional_param": True,
        }
        for method in ("GET", "*")
    ]
    assert json.loads((out / "routes-summary.json").read_text()) == {
        "routes": 2, "with_handler_site": 2, "with_validation": 2,
        "unextracted": 1, "unparsed": 1,
        "unextracted_details": [{"method": "*", "count": 1}],
        "unparsed_files": ["broken.js"],
    }
