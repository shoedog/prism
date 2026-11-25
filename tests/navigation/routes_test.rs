use prism::api::{nav_session, NavOptions};
use prism::framework_entries::routes::*;
use prism::navigation::routes::RouteFormat;
use serde_json::{json, Value};

#[test]
fn listing_sorts_origins_and_paths_and_counts_hidden_mounts() {
    let root = json!({"file":"app.js","binding":"app","construction_byte":4,"invocations":[]});
    let site = json!({"file":"app.js","line":3,"start_byte":10,"end_byte":24});
    let exposure = json!({
        "root":root,"status":"full","value":"/a","path_tokens":[["literal","/a"]],
        "normalized":"/a","host_kind":"unconstrained","hosts":[],"method_guard":"any","version":null
    });
    let mut suffix = exposure.clone();
    suffix["status"] = json!("suffix");
    suffix["reason"] = json!("variable_prefix");
    let router = json!({"status":"router","file":"app.js","binding":"app","construction_byte":4,"invocations":[]});
    let route = json!({
        "framework":"hapi","site":site,"receiver":router,"protocol":"http",
        "methods":{"known":["get","GET","get"]},"path":{"raw":"'/a'","literal":"/a"},
        "paths":[suffix,exposure],"callbacks":[],
        "origin_plugin":{"file":"plugins.js","object_byte":20,"name":"reports"}
    });
    let mut candidate = route.clone();
    candidate["origin_plugin"]["object_byte"] = json!(10);
    candidate["receiver"] = json!({"status":"unproven","reason":"receiver_unproven"});
    candidate["paths"] = json!([]);
    candidate["path"] = json!({"raw":"'/0'","literal":"/0"});
    let mount = json!({
        "parent":router,"child":router,"site":site,"prefix":{"status":"absent"},
        "alternative_ordinal":0,"child_ordinal":0
    });
    let failure = json!({"root":root,"site":site,"reason":"dispatch_conflict"});
    let table = RouteTable::new(
        vec![
            serde_json::from_value(route).unwrap(),
            serde_json::from_value(candidate).unwrap(),
        ],
        vec![serde_json::from_value(mount).unwrap()],
        vec![],
        vec![serde_json::from_value(failure).unwrap()],
        [Framework::Hapi, Framework::Hapi],
    );
    let output = RouteFormat::Json.render(Ok(table.clone()), true);
    let lines: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0]["origin_plugin"]["object_byte"], 10);
    assert_eq!(lines[0]["candidate"], true);
    assert_eq!(lines[1]["methods"], json!({"known":["GET","get"]}));
    assert_eq!(lines[1]["paths"][0]["status"], "full");
    assert_eq!(lines[1]["paths"][1]["status"], "suffix");
    assert_eq!(lines[1]["paths"][0]["binding_blocked"], "any");
    assert_eq!(lines[2]["mount"]["child_ordinal"], 0);
    assert_eq!(lines[3]["summary"]["enabled_adapters"], json!(["hapi"]));
    for (key, value) in [
        ("registrations", 2),
        ("proven", 1),
        ("unproven", 1),
        ("candidates", 1),
        ("mounts", 1),
    ] {
        assert_eq!(lines[3]["summary"][key], value, "{key}");
    }
    let hidden = RouteFormat::Json.render(Ok(table.clone()), false);
    assert_eq!(hidden.lines().count(), 3);
    assert_eq!(hidden.lines().last(), output.lines().last());
    let text = RouteFormat::Text.render(Ok(table), true);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].contains("receiver_unproven"));
    assert!(lines[1].contains("variable_prefix"));
    assert!(lines[2].starts_with("mount "));
    assert!(lines[3].starts_with("summary "));
}

#[test]
fn session_infers_empty_and_refuses_explicit_entry() {
    let repo = tempfile::tempdir().unwrap();
    let mut options = NavOptions::default();
    options.no_cache = true;
    let session = nav_session(repo.path(), &options).unwrap();
    let query = |entry| prism::navigation::routes::list(&session, entry);
    let table = query(None).unwrap();
    assert!(table.routes().is_empty());
    assert!(table.summary().enabled_adapters.is_empty());
    let refusal = query(Some("app:app")).unwrap_err();
    assert_eq!(refusal.reason, Reason::EntryUnproven);
    assert_eq!(
        RouteFormat::Text.render(Err(refusal), false),
        "error entry=\"app:app\" reason=\"entry_unproven\"\n"
    );
}

#[test]
fn empty_exposures_sort_by_literal_then_raw() {
    let row = |raw: &str, literal: &str, start| {
        serde_json::from_value::<Route>(json!({
            "framework":"express","protocol":"http",
            "site":{"file":"app.js","line":1,"start_byte":start,"end_byte":start + 1},
            "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
            "methods":null,"path":{"raw":raw,"literal":literal},
            "paths":[],"callbacks":[],"candidate":false
        })).unwrap()
    };
    let later = row("\"/z\"", "/z", 0);
    let tied = row("`/a`", "/a", 1);
    let first = row("'/a'", "/a", 2);
    let table = RouteTable::new(vec![later, tied, first], vec![], vec![], vec![], []);
    assert_eq!(
        table
            .routes()
            .iter()
            .map(|row| row.site.start_byte)
            .collect::<Vec<_>>(),
        [2, 1, 0]
    );
}

#[test]
fn unproven_receivers_cannot_retain_exposures() {
    let route = serde_json::from_value::<Route>(json!({
        "framework":"express","protocol":"http","site":{"file":"app.js","line":1,"start_byte":0,"end_byte":2},
        "receiver":{"status":"unproven","reason":"receiver_unproven"},
        "methods":null,"path":{"raw":"x","literal":null},
        "paths":[{"root":null,"status":"dynamic","reason":"non_literal_path",
                  "path_tokens":null,"normalized":null,"host_kind":"unconstrained","hosts":[],
                  "method_guard":"any","version":null}],
        "callbacks":[]
    })).unwrap();
    let table = RouteTable::new(vec![route], vec![], vec![], vec![], []);
    assert!(table.routes()[0].paths.is_empty());
    assert_eq!(table.routes()[0].candidate, Some(true));
    assert_eq!(table.summary().candidates, 1);
}

#[test]
fn candidate_false_is_omitted() {
    let route: Route = serde_json::from_value(json!({
        "framework":"express","protocol":"http","site":{"file":"app.js","line":1,"start_byte":0,"end_byte":2},
        "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
        "methods":null,"path":{"raw":"x","literal":null},"paths":[],"callbacks":[],"candidate":false
    })).unwrap();
    assert!(serde_json::to_value(route)
        .unwrap()
        .get("candidate")
        .is_none());
}
