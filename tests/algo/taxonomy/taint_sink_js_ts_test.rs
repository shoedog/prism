#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn run_taint_js_ts_single(
    source: &str,
    path: &str,
    language: Language,
    diff_lines: BTreeSet<usize>,
) -> prism::slice::SliceResult {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink_on(result: &prism::slice::SliceResult, line: usize) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == line)
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

#[test]
fn test_tsx_nest_body_reaches_dangerously_set_inner_html() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    return <div dangerouslySetInnerHTML={{ __html: html }} />;
  }
}
"#;
    let result = run_taint_js_ts_single(source, "page.tsx", Language::Tsx, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "NestJS @Body DTO field should reach TSX dangerouslySetInnerHTML.__html"
    );
}

#[test]
fn test_tsx_text_interpolation_is_not_xss_sink() {
    let source = r#"import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    return <div>{html}</div>;
  }
}
"#;
    let result = run_taint_js_ts_single(source, "page.tsx", Language::Tsx, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "React text interpolation is escaped by default and should not be a Phase 3 XSS sink"
    );
}

#[test]
fn test_express_request_query_reaches_sequelize_query() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Express req.query assignment should reach SQL raw query sink"
    );
}

#[test]
fn test_express_multiline_query_arg_reaches_sequelize_query() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  return sequelize.query(
    `SELECT * FROM users WHERE name = '${term}'`
  );
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "multi-line structured sink args should match tainted identifiers on their own lines"
    );
}

#[test]
fn test_express_request_param_multi_hop_alias_reaches_sql() {
    let source = r#"import express from "express";

const app = express();

app.get("/item/:id", function(req, res) {
  const a = req.params.id;
  const b = a;
  return sequelize.query(`SELECT * FROM items WHERE id = '${b}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express request-data aliases should keep propagating through standard JS DFG"
    );
}

#[test]
fn test_express_request_query_reaches_mongoose_where() {
    let source = r#"import express from "express";
import mongoose from "mongoose";

const app = express();
const User = mongoose.model("User");

app.get("/search", function(req, res) {
  const predicate = req.query.predicate;
  return User.$where(predicate);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "Mongoose $where should fire when Express query data reaches the JS predicate"
    );
}

#[test]
fn test_express_request_query_reaches_prisma_raw_unsafe() {
    let source = r#"import express from "express";
import { PrismaClient } from "@prisma/client";

const app = express();
const prisma = new PrismaClient();

app.get("/search", async function(req, res) {
  const term = req.query.term;
  return prisma.$queryRawUnsafe(`SELECT * FROM users WHERE name = '${term}'`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 9),
        "Prisma $queryRawUnsafe should fire for tainted raw SQL"
    );
}

#[test]
fn test_express_mixed_template_sql_with_bind_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.get("/search", function(req, res) {
  const term = req.query.term;
  const id = 1;
  return sequelize.query(`SELECT * FROM users WHERE name = '${term}' AND id = $1`, { bind: [id] });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpolated template SQL must not be suppressed just because another placeholder is bound"
    );
}

#[test]
fn test_express_json_parse_does_not_fire_deserialization() {
    let source = r#"import express from "express";

const app = express();

app.post("/json", (req, res) => {
  const payload = req.body.payload;
  const parsed = JSON.parse(payload);
  res.json(parsed);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 7),
        "JSON.parse is not a CWE-502 sink"
    );
}

#[test]
fn test_express_yaml_unsafe_schema_name_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "UNSAFE_SCHEMA must not be treated as SAFE_SCHEMA by substring matching"
    );
}

#[test]
fn test_express_destructured_yaml_load_still_fires() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "destructured js-yaml load imports should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_destructured_yaml_load_safe_schema_suppresses() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bare js-yaml load imports should honor imported safe schema constants"
    );
}

#[test]
fn test_express_same_line_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => { const payload = req.body.payload; const JSON_SCHEMA = req.body.schema; return load(payload, { schema: JSON_SCHEMA }); });
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "same-line local schema bindings must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_block_var_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  if (req.body.useSchema) {
    var JSON_SCHEMA = req.body.schema;
  }
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "block-contained var schema bindings are function-scoped and must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_later_var_shadowed_yaml_schema_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const parsed = load(payload, { schema: JSON_SCHEMA });
  var JSON_SCHEMA = req.body.schema;
  return parsed;
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "later var schema declarations are hoisted and must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_uninitialized_yaml_schema_shadow_still_fires() {
    let source = r#"import express from "express";
import { load, JSON_SCHEMA } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  let JSON_SCHEMA;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "uninitialized local declarations must shadow imported safe schema constants"
    );
}

#[test]
fn test_express_nested_imported_yaml_schema_does_not_suppress_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const JSON_SCHEMA = {};
const app = express();

function helper() {
  const { JSON_SCHEMA } = require("js-yaml");
  return JSON_SCHEMA;
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload, { schema: JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper js-yaml schema imports must not make outer JSON_SCHEMA references trusted"
    );
}

#[test]
fn test_express_commonjs_aliased_yaml_load_still_fires() {
    let source = r#"import express from "express";
const { load: yamlLoad } = require("js-yaml");

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yamlLoad(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "CommonJS destructuring aliases from js-yaml should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_late_top_level_commonjs_yaml_load_still_fires() {
    let source = r#"import express from "express";

const app = express();

app.post("/yaml", (req, res) => {
  return load(req.body.payload);
});

const { load } = require("js-yaml");
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level CommonJS bindings declared after a route callback are visible when the handler runs"
    );
}

#[test]
fn test_express_commonjs_require_member_yaml_load_still_fires() {
    let source = r#"import express from "express";
const yamlLoad = require("js-yaml").load;

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yamlLoad(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "CommonJS require(\"js-yaml\").load aliases should be registered as CWE-502 sinks"
    );
}

#[test]
fn test_express_yaml_dump_import_does_not_fire() {
    let source = r#"import express from "express";
import { dump } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return dump(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "js-yaml dump serializes and must not be treated as the bare load sink"
    );
}

#[test]
fn test_express_shadowed_yaml_load_import_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const load = (value) => value;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "local handler bindings should shadow imported js-yaml load"
    );
}

#[test]
fn test_express_nested_block_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  {
    const load = (value) => value;
    load("safe");
  }
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "block-local load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_function_shadowed_yaml_load_import_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function load(value) {
    return value;
  }
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "local function declarations should shadow imported js-yaml load"
    );
}

#[test]
fn test_express_nested_function_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function helper() {
    const load = (value) => value;
    return load("safe");
  }
  helper();
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested-function load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_nested_function_var_yaml_load_shadow_still_fires_outer_call() {
    let source = r#"import express from "express";
import { load } from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  function helper() {
    var load = (value) => value;
    return load("safe");
  }
  helper();
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested-function var load bindings must not shadow outer imported js-yaml load calls"
    );
}

#[test]
fn test_express_nested_imported_yaml_load_does_not_fire_outer_call() {
    let source = r#"import express from "express";

const app = express();

function helper() {
  const { load } = require("js-yaml");
  return load("safe");
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "nested helper js-yaml load imports must not make outer bare load calls sinks"
    );
}

#[test]
fn test_express_unrelated_bare_load_does_not_fire() {
    let source = r#"import express from "express";
import { load } from "./local-loader";

const app = express();

app.post("/load", (req, res) => {
  const payload = req.body.payload;
  return load(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "bare load should only be a deserialization sink when it resolves to js-yaml"
    );
}

#[test]
fn test_express_yaml_unrelated_safe_schema_token_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA, label: "SAFE_SCHEMA" });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe-schema tokens outside the schema option must not suppress unsafe yaml.load"
    );
}

#[test]
fn test_express_yaml_exact_safe_schema_suppresses() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "exact js-yaml safe schema constants should still suppress unsafe-load findings"
    );
}

#[test]
fn test_express_nested_module_alias_yaml_schema_does_not_suppress_outer_call() {
    let source = r#"import express from "express";

const yaml = { load: (value) => value, JSON_SCHEMA: {} };
const app = express();

function helper() {
  const yaml = require("js-yaml");
  return yaml.JSON_SCHEMA;
}

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper js-yaml module aliases must not make outer yaml.JSON_SCHEMA references trusted"
    );
}

#[test]
fn test_express_yaml_duplicate_schema_override_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA, schema: yaml.UNSAFE_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "duplicate schema properties can override safe schema constants and must fail closed"
    );
}

#[test]
fn test_express_yaml_spread_schema_override_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const opts = { schema: yaml.UNSAFE_SCHEMA };
  return yaml.load(payload, { schema: yaml.JSON_SCHEMA, ...opts });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "spread schema options can override safe schema constants and must fail closed"
    );
}

#[test]
fn test_express_yaml_ignored_third_safe_schema_arg_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  return yaml.load(payload, { schema: yaml.UNSAFE_SCHEMA }, yaml.JSON_SCHEMA);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe-looking ignored args after the options argument must not suppress unsafe yaml.load"
    );
}

#[test]
fn test_express_yaml_request_controlled_schema_holder_still_fires() {
    let source = r#"import express from "express";
import yaml from "js-yaml";

const app = express();

app.post("/yaml", (req, res) => {
  const payload = req.body.payload;
  const schemaHolder = { JSON_SCHEMA: req.body.schema };
  return yaml.load(payload, { schema: schemaHolder.JSON_SCHEMA });
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "safe schema suppression must not trust arbitrary objects ending in JSON_SCHEMA"
    );
}

#[test]
fn test_express_body_reaches_new_function_rce_bucket() {
    let source = r#"import express from "express";

const app = express();

app.post("/compile", (req, res) => {
  const code = req.body.code;
  const fn = new Function(code);
  return fn();
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "new Function is intentionally bucketed with CWE-502/RCE sinks for Phase 3"
    );
}

#[test]
fn test_express_node_serialize_unserialize_fires() {
    let source = r#"import express from "express";
import serialize from "node-serialize";

const app = express();

app.post("/payload", (req, res) => {
  const payload = req.body.payload;
  return serialize.unserialize(payload);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "node-serialize unserialize should fire for Express request body data"
    );
}

#[test]
fn test_koa_request_body_reaches_fetch_ssrf() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 7),
        "Koa ctx.request.body URL should reach fetch SSRF sink"
    );
}

#[test]
fn test_express_query_reaches_axios_get_ssrf() {
    let source = r#"import express from "express";
import axios from "axios";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return axios.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express req.query URL should reach axios.get SSRF sink"
    );
}

#[test]
fn test_js_ts_ssrf_unrelated_url_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL("https://example.com");
  if (!allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allowlist check for an unrelated URL must not suppress fetch(target)"
    );
}

#[test]
fn test_js_ts_ssrf_inverted_url_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allow-on-pass guard whose branch returns must not suppress the later fetch"
    );
}

#[test]
fn test_js_ts_ssrf_denylist_guard_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const blockedHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!blockedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "denylist-shaped URL guards must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_disallowed_name_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const disallowedHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!disallowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "negative receiver names containing allow must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_unsafe_name_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const unsafeHosts = new Set(["169.254.169.254"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!unsafeHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "negative receiver names containing safe must not be treated as allowlist proof"
    );
}

#[test]
fn test_js_ts_ssrf_request_derived_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const allowedHosts = new Set(ctx.query.allowedHosts);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "request-derived allowlist collections must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_mutated_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = new Set(["example.com"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  allowedHosts.add(ctx.query.allowedHost);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "allowlist collections mutated with non-literal values must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_alias_mutated_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = new Set(["example.com"]);

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const hosts = allowedHosts;
  hosts.add(ctx.query.allowedHost);
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "allowlist collections mutated through aliases must not suppress SSRF sinks"
    );
}

#[test]
fn test_js_ts_ssrf_object_assign_array_allowlist_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  Object.assign(allowedHosts, [ctx.query.allowedHost]);
  const parsed = new URL(target);
  if (!allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Object.assign array mutations can replace literal allowlist entries and must fail closed"
    );
}

#[test]
fn test_js_ts_ssrf_nested_allowlist_literal_does_not_suppress_outer_guard() {
    let source = r#"import Koa from "koa";

const app = new Koa();

function helper() {
  const allowedHosts = new Set(["example.com"]);
  return allowedHosts;
}

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper allowlists must not prove trust for an outer SSRF guard"
    );
}

#[test]
fn test_js_ts_ssrf_late_top_level_allowlist_mutation_does_not_suppress() {
    let source = r#"import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!allowedHosts.has(parsed.hostname)) {
    return;
  }
  return fetch(target);
});

const allowedHosts = new Set(["example.com"]);
allowedHosts.add(process.env.ALLOWED_HOST);
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level allowlist mutations after handler registration run before requests and must fail closed"
    );
}

#[test]
fn test_express_query_reaches_superagent_get_ssrf() {
    let source = r#"import express from "express";
import superagent from "superagent";

const app = express();

app.get("/proxy", (req, res) => {
  const target = req.query.target_url;
  return superagent.get(target);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "superagent.get should be covered by the Phase 3 SSRF sink list"
    );
}

#[test]
fn test_unrelated_get_method_does_not_fire_ssrf() {
    let source = r#"import express from "express";

const app = express();
const cache = new Map();

app.get("/cache", (req, res) => {
  const key = req.query.key;
  return cache.get(key);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "non-HTTP .get methods should not be treated as SSRF sinks"
    );
}

#[test]
fn test_fastify_request_body_reaches_child_process_exec() {
    let source = r#"import fastify from "fastify";
import { exec } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return exec(`psql -c ${arg}`);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Fastify request.body should reach child_process.exec command sink"
    );
}

#[test]
fn test_exec_file_literal_binary_does_not_flat_leak() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("psql", ["-c", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink_on(&result, 8),
        "literal-binary execFile should suppress the broad flat execFile fallback"
    );
}

#[test]
fn test_exec_file_literal_binary_shell_option_identifier_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: true };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "execFile with identifier-bound shell:true options must not be suppressed as literal-binary safe"
    );
}

#[test]
fn test_exec_file_literal_binary_uninspectable_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const useShell = request.body.useShell;
  const opts = { shell: useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "execFile with non-literal shell option state must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_false_or_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const useShell = request.body.useShell;
  const opts = { shell: false || useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "shell option expressions that start with false are still uninspectable and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_uninspectable_options_node_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = buildOptions(request.body.useShell);
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "identifier options bound to non-object expressions must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_spread_shell_override_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const override = { shell: request.body.useShell };
  const opts = { shell: false, ...override };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "options spreads can override shell:false at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_computed_shell_override_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const key = "shell";
  const opts = { shell: false, [key]: request.body.useShell };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "computed option keys can override shell:false at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_getter_shell_option_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { get shell() { return request.body.useShell; } };
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "accessor option properties can enable shell at runtime and must fail closed"
    );
}

#[test]
fn test_exec_file_literal_binary_alias_shell_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: false };
  const alias = opts;
  alias.shell = request.body.useShell;
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "alias mutation can enable shell after an inspectably safe options binding"
    );
}

#[test]
fn test_exec_file_literal_binary_object_assign_shell_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const opts = { shell: false };
  Object.assign(opts, { shell: request.body.useShell });
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "Object.assign can enable shell after an inspectably safe options binding"
    );
}

#[test]
fn test_exec_file_nested_safe_options_do_not_suppress_outer_call() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

function helper() {
  const opts = { shell: false };
  return opts;
}

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("psql", ["-c", arg], opts);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper options bindings must not make outer execFile options inspectably safe"
    );
}

#[test]
fn test_exec_file_late_top_level_options_mutation_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.get("/run", async (request, reply) => {
  const arg = request.query.arg;
  return execFile("git", ["status", arg], opts);
});

const opts = { shell: false };
Object.assign(opts, { shell: process.env.SHELL });
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "top-level options mutations after handler registration run before requests and must fail closed"
    );
}

#[test]
fn test_exec_file_shell_wrapper_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("sh", ["-c", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "literal shell-wrapper execFile must not be suppressed as a safe literal-binary form"
    );
}

#[test]
fn test_exec_file_node_args_variable_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  const args = ["-e", arg];
  return execFile("node", args);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpreter execFile argv passed through a variable must fail closed"
    );
}

#[test]
fn test_exec_file_node_long_eval_flag_still_fires() {
    let source = r#"import fastify from "fastify";
import { execFile } from "child_process";

const app = fastify();

app.post("/run", async (request, reply) => {
  const arg = request.body.shellArg;
  return execFile("node", ["--eval", arg]);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "interpreter execFile long eval flags must not be suppressed as literal-binary safe forms"
    );
}

#[test]
fn test_express_send_file_path_traversal_fires() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  return res.sendFile(path.join("/uploads", filename));
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "Express req.params filename should reach res.sendFile path traversal sink"
    );
}

#[test]
fn test_express_same_line_handler_reference_reaches_send_file() {
    let source = r#"import express from "express";

const app = express();

app.get("/download/:name", (req, res) => res.sendFile(req.params.name));
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([5]));
    assert!(
        has_taint_sink_on(&result, 5),
        "same-line JS/TS handler param references should remain visible to sink matching"
    );
}

#[test]
fn test_express_request_param_alias_chain_reaches_send_file() {
    let source = r#"import express from "express";

const app = express();

app.get("/download/:name", (req, res) => {
  const file = req.params.name;
  const candidate = file;
  const finalPath = candidate;
  return res.sendFile(finalPath);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "JS/TS target-seed alias synthesis should follow assignment chains beyond one hop"
    );
}

#[test]
fn test_js_ts_path_inverted_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "an allow-prefix branch that returns must not suppress the later sendFile"
    );
}

#[test]
fn test_js_ts_path_unrelated_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const other = path.resolve(uploadsDir, "static.txt");
  if (!other.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "a prefix check for an unrelated path variable must not suppress sendFile(candidate)"
    );
}

#[test]
fn test_js_ts_path_attacker_controlled_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(filename);
  if (!resolved.startsWith(filename)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "path prefix guards must prove the prefix is trusted, not attacker-controlled"
    );
}

#[test]
fn test_js_ts_path_non_boundary_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "startsWith(base) without a path boundary must not suppress path traversal sinks"
    );
}

#[test]
fn test_js_ts_path_boundary_prefix_guard_suppresses() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads/";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        !has_taint_sink(&result),
        "startsWith(base) may suppress only when the trusted base preserves a path boundary"
    );
}

#[test]
fn test_js_ts_path_nested_prefix_literal_does_not_suppress_outer_guard() {
    let source = r#"import express from "express";
import path from "path";

const app = express();

function helper() {
  const uploadsDir = "/uploads/";
  return uploadsDir;
}

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "nested helper path-prefix constants must not prove containment for an outer guard"
    );
}

#[test]
fn test_js_ts_path_root_prefix_guard_does_not_suppress() {
    let source = r#"import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.js", Language::JavaScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink(&result),
        "root prefixes prove no containment and must not suppress path traversal sinks"
    );
}

#[test]
fn test_express_query_reaches_fs_promises_read_file() {
    let source = r#"import express from "express";
import { readFile } from "fs/promises";

const app = express();

app.get("/file", async (req, res) => {
  const filename = req.query.file;
  return readFile(filename);
});
"#;
    let result =
        run_taint_js_ts_single(source, "app.ts", Language::TypeScript, BTreeSet::from([1]));
    assert!(
        has_taint_sink_on(&result, 8),
        "destructured fs/promises readFile should fire as a modern Node path traversal sink"
    );
}
