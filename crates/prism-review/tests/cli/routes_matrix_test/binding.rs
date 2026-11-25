//! Fixture-only projection from static route lines to eligible terminal identities.
use super::{
    checked_path,
    listing::{items, router},
};
use prism::{ast::ParsedFile, languages::Language};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

fn method_in(set: &Value, method: &str) -> bool {
    set == "any" || items(&set["known"]).iter().any(|known| known == method)
}

fn host_matches(
    path: &Value,
    host: Option<&str>,
    dns_only: bool,
    framework: &Value,
    method: &str,
) -> bool {
    if path["host_kind"] == "unconstrained" {
        return true;
    }
    let Some(host) = host.filter(|_| !dns_only && path["host_kind"] == "literal") else {
        return false;
    };
    let no_port = || {
        host.rsplit_once(':')
            .filter(|(name, port)| {
                (!name.contains(':') || name.ends_with(']'))
                    && port.bytes().all(|byte| byte.is_ascii_digit())
            })
            .map_or(host, |(name, _)| name)
    };
    items(&path["hosts"]).iter().any(|expected| {
        expected.as_str().is_some_and(|expected| {
            if framework == "net/http" {
                expected == if method == "CONNECT" { host } else { no_port() }
            } else {
                expected.eq_ignore_ascii_case(no_port())
            }
        })
    })
}

fn endpoint(pin: &Value, route_pins: &Value, dir: &Path) -> Result<Value, String> {
    let Some(name) = pin["endpoint"].as_str() else {
        return Ok(Value::Null);
    };
    let declarations: BTreeSet<_> = items(route_pins)
        .iter()
        .flat_map(|route| items(&route["callbacks"]))
        .filter_map(|callback| {
            let id = if callback["delegate"]["name"] == name {
                &callback["delegate"]
            } else {
                callback
            };
            (id["name"] == name).then_some((id["file"].as_str()?, id["start_line"].as_u64()?))
        })
        .collect();
    let declarations: Vec<_> = declarations.into_iter().collect();
    let [(file, line)] = declarations.as_slice() else {
        return Err(format!("{name}: no unique callback pin"));
    };
    let path = checked_path(dir, &json!(file), false)?;
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let language = Language::from_path(file).ok_or_else(|| format!("{file}: no parser"))?;
    let parsed = ParsedFile::parse(file, &source, language).map_err(|error| error.to_string())?;
    let mut functions = parsed.functions().iter().filter(|function| {
        function.name.as_deref() == Some(name) && function.start_line as u64 == *line
    });
    let function = functions
        .next()
        .filter(|_| functions.next().is_none())
        .ok_or_else(|| format!("{file}:{line}: expected one endpoint declaration {name}"))?;
    Ok(
        json!({"file":file,"name":name,"start_line":function.start_line,"end_line":function.end_line,
        "start_byte":function.start_byte,"end_byte":function.end_byte}),
    )
}

pub(super) fn check_bindings(
    row: &Value,
    dir: &Path,
    root: Option<&Value>,
    routes: &[Value],
    errors: &mut Vec<String>,
) {
    let roots: BTreeSet<_> = routes
        .iter()
        .flat_map(|route| items(&route["paths"]))
        .filter_map(|path| {
            (path["status"] == "full")
                .then(|| router(&path["root"]))
                .flatten()
        })
        .collect();
    let inferred = if roots.len() == 1 {
        json!(roots.first())
    } else {
        Value::Null
    };
    for (index, pin) in items(&row["expect_bindings"]).iter().enumerate() {
        let root = pin.get("root").or(root).unwrap_or(&inferred);
        let method = pin["method"].as_str().unwrap_or_default();
        let mut terminals = Vec::new();
        for route in routes {
            if root.is_null()
                || route["protocol"] != "http"
                || !(method_in(&route["methods"], method)
                    || method_in(&route["default_methods"], method))
            {
                continue;
            }
            for path in items(&route["paths"]) {
                if path["status"] != "full"
                    || path["root"] != *root
                    || path["value"] != pin["value"]
                    || path["shadowed"] == true
                    || path.get("setup_guard").is_some()
                    || path.get("mount_refusal").is_some()
                    || !path["version"].is_null()
                    || !method_in(&path["method_guard"], method)
                    || !host_matches(
                        path,
                        pin["host"].as_str(),
                        pin["dns_only"] == true,
                        &route["framework"],
                        method,
                    )
                {
                    continue;
                }
                let terminal = items(&route["callbacks"])
                    .iter()
                    .rev()
                    .find(|callback| callback["role"] != "pre");
                let callable = terminal
                    .filter(|_| !method_in(&path["binding_blocked"], method))
                    .map_or(Value::Null, |callback| {
                        if !callback["delegate"].is_null() {
                            return callback["delegate"].clone();
                        }
                        if callback["status"] == "unresolved" {
                            return Value::Null;
                        }
                        let keys = [
                            "file",
                            "name",
                            "start_line",
                            "end_line",
                            "start_byte",
                            "end_byte",
                        ];
                        Value::Object(
                            keys.into_iter()
                                .map(|key| (key.to_owned(), callback[key].clone()))
                                .collect(),
                        )
                    });
                terminals.push(callable);
            }
        }
        let actual = match terminals.as_slice() {
            [terminal] => terminal,
            _ => &Value::Null,
        };
        match endpoint(pin, &row["expect"], dir) {
            Ok(want) if want == *actual => {}
            Ok(want) => errors.push(format!(
                "expect_bindings[{index}]: expected {want}, got {actual}"
            )),
            Err(error) => errors.push(format!("expect_bindings[{index}]: {error}")),
        }
    }
}

#[test]
fn projection_checks_root_native_host_and_blocked_methods() {
    let dir = tempfile::tempdir().unwrap();
    let source = "function terminal() {}";
    std::fs::write(dir.path().join("app.js"), source).unwrap();
    let mut probe =
        json!({"method":"HEAD","value":"/ready","host":"EXAMPLE.test:8080","endpoint":"terminal"});
    let mut row = json!({
        "expect":[{"callbacks":[{"file":"app.js","name":"terminal","start_line":1}]}],
        "expect_bindings":[probe]
    });
    let root = json!({"file":"app.js","binding":"app","construction_byte":0,"invocations":[]});
    let mut route = json!({
        "protocol":"http","framework":"net/http","methods":{"known":["HEAD"]},
        "callbacks":[{"status":"resolved","file":"app.js","name":"terminal",
            "start_line":1,"end_line":1,"start_byte":0,"end_byte":source.len()}],
        "paths":[{"status":"full","root":root,"value":"/ready","host_kind":"literal",
            "hosts":["EXAMPLE.test"],"method_guard":"any","binding_blocked":{"known":["GET"]}}]
    });
    let compare = |row: &Value, route: &Value, root: &Value| {
        let mut errors = Vec::new();
        check_bindings(
            row,
            dir.path(),
            Some(root),
            std::slice::from_ref(route),
            &mut errors,
        );
        errors
    };
    assert!(compare(&row, &route, &root).is_empty());
    let mut wrong_root = root.clone();
    wrong_root["construction_byte"] = json!(4);
    assert_eq!(compare(&row, &route, &wrong_root).len(), 1);
    probe["host"] = json!("example.test:8080");
    row["expect_bindings"][0] = probe;
    assert_eq!(compare(&row, &route, &root).len(), 1);
    row["expect_bindings"][0]["host"] = json!("EXAMPLE.test:8080");
    route["paths"][0]["binding_blocked"] = json!("any");
    assert_eq!(compare(&row, &route, &root).len(), 1);
    row["expect_bindings"][0]["endpoint"] = Value::Null;
    assert!(compare(&row, &route, &root).is_empty());
}
