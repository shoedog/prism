use prism::ast::ParsedFile;
use prism::frameworks;
use prism::languages::Language;

fn parse_js(source: &str) -> ParsedFile {
    ParsedFile::parse("app.js", source, Language::JavaScript).unwrap()
}

fn parse_ts(source: &str) -> ParsedFile {
    ParsedFile::parse("app.ts", source, Language::TypeScript).unwrap()
}

#[test]
fn test_detect_express_import() {
    let parsed = parse_js(
        r#"import express from "express";
const app = express();
"#,
    );
    assert_eq!(
        frameworks::detect_for(&parsed).map(|s| s.name),
        Some("express")
    );
}

#[test]
fn test_detect_fastify_require() {
    let parsed = parse_js(
        r#"const fastify = require("fastify");
const app = fastify();
"#,
    );
    assert_eq!(
        frameworks::detect_for(&parsed).map(|s| s.name),
        Some("fastify")
    );
}

#[test]
fn test_detect_koa_import() {
    let parsed = parse_js(
        r#"import Koa from "koa";
const app = new Koa();
"#,
    );
    assert_eq!(frameworks::detect_for(&parsed).map(|s| s.name), Some("koa"));
}

#[test]
fn test_detect_nestjs_import_takes_precedence() {
    let parsed = parse_ts(
        r#"import { Controller } from "@nestjs/common";
import express from "express";
"#,
    );
    assert_eq!(
        frameworks::detect_for(&parsed).map(|s| s.name),
        Some("nestjs")
    );
}

#[test]
fn test_no_js_framework_without_import() {
    let parsed = parse_js(
        r#"function handler(req, res) {
  res.send(req.query.q);
}
"#,
    );
    assert_eq!(frameworks::detect_for(&parsed).map(|s| s.name), None);
}
