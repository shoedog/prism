use crate::support::{fixture, rows, Row};
use prism_boundary::element::route::{
    project_route, Dialect, RouteDeclaration, StarPolicy, Visibility, Withheld,
};
use prism_boundary::element::{self, CaptureWidth};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn projections() -> BTreeMap<String, Vec<Value>> {
    let sources: BTreeMap<_, _> = rows().map(|row| (row.id, row)).collect();
    let mut rows = BTreeMap::new();
    for line in include_str!("../projections/routes-go.jsonl")
        .lines()
        .chain(include_str!("../projections/routes-js.jsonl").lines())
    {
        let entry: Value = serde_json::from_str(line).unwrap();
        let id = entry["row"].as_str().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(fixture(&sources[id], None))),
            entry["source_sha256"].as_str().unwrap(),
            "{id}"
        );

        assert!(rows
            .insert(
                entry["row"].as_str().unwrap().into(),
                entry["declarations"].as_array().unwrap().clone()
            )
            .is_none());
    }
    rows
}

pub fn projection(row: &Row, data: &Value) -> RouteDeclaration {
    let text = String::from_utf8(fixture(row, None)).unwrap();
    let line = data["line"].as_u64().unwrap() as usize;
    let policy_lines: Vec<_> = data["policy_lines"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|value| value.as_u64().unwrap() as usize)
        .collect();
    for line in [line].into_iter().chain(policy_lines.iter().copied()) {
        assert!(
            !text.lines().nth(line - 1).unwrap().trim().is_empty(),
            "{}:{line}",
            row.fixture
        );
    }
    let strings = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().into())
            .collect()
    };
    let flag = |key: &str| data[key].as_bool().unwrap_or(false);
    let string = |key: &str| data[key].as_str().map(str::to_owned);
    RouteDeclaration {
        dialect: match data["dialect"].as_str().unwrap() {
            "Gin" => Dialect::Gin,
            "Express4" => Dialect::Express4,
            "Express5" => Dialect::Express5,
            "Hapi" => Dialect::Hapi,
            "Brace" => Dialect::Brace,
            "Literal" => Dialect::Literal,
            value => panic!("{value}"),
        },
        version: string("version"),
        methods: strings(&data["methods"]).into_iter().map(Some).collect(),
        paths: strings(&data["paths"]),
        prefixes: data["prefixes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(strings)
            .collect(),
        mount: string("mount"),
        uppercase: flag("uppercase"),
        ascii_fold: flag("ascii_fold"),
        strip_slash: flag("strip_slash"),
        raw_path: flag("raw_path"),
        empty_tail: flag("empty_tail"),
        bare_parent: flag("bare_parent"),
        implicit_methods: flag("implicit_methods"),
        selector: string("selector"),
        api: "test-api".into(),
        authority: string("authority").unwrap(),
        file: row.fixture.into(),
        line,
        policy_lines,
        star: if data["star"] == "Open" {
            StarPolicy::Open
        } else if let Some(closed) = data["star"].get("Closed") {
            StarPolicy::Closed(strings(closed))
        } else {
            StarPolicy::Unchecked
        },
        withheld: None,
        visibility: match data["visibility"].as_str() {
            Some("Local") => Visibility::Local,
            Some("Detached") => Visibility::Detached,
            Some("Internal") => Visibility::Internal,
            None => Visibility::Public,
            Some(value) => panic!("{value}"),
        },
    }
}

#[test]
fn route_matrix_census_and_scoped_losses() {
    let mut projections = projections();
    let expectations: Value =
        serde_json::from_str(include_str!("../fixtures/ids_matrix/expectations.json")).unwrap();
    let mut count = 0;
    for row in rows().filter(|r| {
        !r.id.starts_with("control-") && (r.fixture.ends_with(".go") || r.fixture.ends_with(".js"))
    }) {
        count += 1;
        let cases = projections.remove(row.id).unwrap();
        let mut public = BTreeSet::new();
        let mut documented = BTreeSet::new();
        let has_registration = cases.iter().any(|case| case["authority"] == "registration");
        let mut local = BTreeSet::new();
        let mut annotations = Vec::new();
        let mut registrations = Vec::new();
        let mut selectors = BTreeSet::new();
        let mut limits = BTreeSet::new();
        for case in cases {
            let declaration = projection(&row, &case);
            let result = project_route(&declaration);
            assert_eq!(result.declaration, declaration, "{}", row.id);
            limits.extend(result.limits.iter().map(|limit| limit.code()));
            for route in &result.routes {
                let id = route.id.to_string();
                if declaration.authority == "annotation" {
                    annotations.push(route.id.clone());
                } else {
                    registrations.push(route.id.clone());
                }
                if let Some(selector) = &declaration.selector {
                    selectors.insert(selector.clone());
                }
                if route.public {
                    if declaration.authority == "annotation" {
                        documented.insert(id);
                    } else {
                        public.insert(id);
                    }
                } else {
                    local.insert(id);
                }
                for (path, expected) in expectations[row.id]["path_probes"]
                    .as_object()
                    .into_iter()
                    .flatten()
                {
                    assert_eq!(
                        route.path.accepts(path),
                        expected.as_bool().unwrap(),
                        "{} {path}",
                        row.id
                    );
                }
                if expectations[row.id]["bare_parent_requires_separate_policy"] == true {
                    assert!(!route.path.accepts("/mugs"), "{}", row.id);
                }
                if let Some(expected) = expectations[row.id]["optional_separator"].as_bool() {
                    assert_eq!(
                        route.path.captures().next().unwrap().optional_separator,
                        expected,
                        "{}",
                        row.id
                    );
                }
                for alias in expectations[row.id]["aliases"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    assert!(route.path.accepts(alias.as_str().unwrap()), "{}", row.id);
                }
                if row.id == "s1-express5-root-inclusive-wildcard" {
                    assert!(route.path.accepts("/"));
                }
            }
        }
        if !has_registration {
            public = documented;
        }
        let companion = &expectations[row.id];
        if let Some(expected) = companion["drift"].as_str() {
            let drift = element::route::annotation_drift(&registrations[0], &annotations[0]);
            assert_eq!(drift.unwrap().code(), expected);
            assert_ne!(registrations, annotations, "{}", row.id);
            assert_eq!(
                annotations[0].to_string(),
                companion["annotation"].as_str().unwrap()
            );
            assert!(!public.contains(&annotations[0].to_string()), "{drift:?}");
        }
        assert_eq!(
            public.iter().map(String::as_str).collect::<Vec<_>>(),
            row.operations,
            "{}",
            row.id
        );
        if let Some(reason) = row.reason {
            assert!(limits.contains(reason), "{} {limits:?}", row.id);
        }
        if let Some(ids) = companion["local_ids"].as_array() {
            assert_eq!(
                local.iter().map(String::as_str).collect::<Vec<_>>(),
                ids.iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>(),
                "{}",
                row.id
            );
        }
        if row.reason == Some("dispatch_discriminator") {
            assert_eq!(selectors.len(), 2, "{}", row.id);
            assert_eq!(registrations.len(), 2, "{}", row.id);
        }
    }
    assert_eq!(count, 52);
    assert!(projections.is_empty(), "{:?}", projections.keys());
}

#[test]
fn route_withheld_policies_and_capture_widths() {
    let inputs = projections();
    let expectations: Value =
        serde_json::from_str(include_str!("../fixtures/ids_matrix/expectations.json")).unwrap();
    let mut count = 0;
    for row in rows().filter(|row| {
        row.id.starts_with("control-")
            && row.id.ends_with("-withheld")
            && row.fixture.ends_with(".js")
            || row.id == "control-gin-catchall-version-withheld"
    }) {
        let companion = &expectations[row.id];
        let baseline = companion["baseline"].as_str().unwrap_or_else(|| {
            row.id
                .strip_prefix("control-")
                .unwrap()
                .strip_suffix("-withheld")
                .unwrap()
        });
        let mut public = BTreeSet::new();
        for (position, input) in inputs[baseline].iter().enumerate() {
            let mut route = projection(&row, input);
            if baseline == "s1-hapi-internal-route" && position == 1 {
                public.extend(
                    project_route(&route)
                        .routes
                        .iter()
                        .map(|r| r.id.to_string()),
                );
                continue;
            }
            let code = match companion["withheld"].as_str() {
                Some("version") => {
                    route.version = None;
                    if companion["scope"] == "tail_width" {
                        "tail_width_unchecked"
                    } else {
                        "optional_separator_unchecked"
                    }
                }
                Some("join_policy" | "plugin_selector") => {
                    route.withheld = Some(Withheld::Mount);
                    "mount_unchecked"
                }
                Some("method_key" | "method_normalization") => {
                    route.withheld = Some(Withheld::Method);
                    "method_unchecked"
                }
                Some("path_dialect") => {
                    route.withheld = Some(Withheld::Path);
                    "path_unchecked"
                }
                Some("router_options") => {
                    route.withheld = Some(Withheld::RouterOptions);
                    "router_policy_unchecked"
                }
                Some("entry_options") => {
                    route.withheld = Some(Withheld::EntryOptions);
                    "entry_options_unchecked"
                }
                None if companion["reason"] == "star_policy_unchecked" => {
                    route.star = StarPolicy::Unchecked;
                    if route.dialect == Dialect::Hapi {
                        route.version = None;
                    }
                    "star_policy_unchecked"
                }
                unexpected => panic!("unexpected withheld policy {unexpected:?} in {}", row.id),
            };
            let result = project_route(&route);
            assert!(
                result.limits.iter().any(|limit| limit.code() == code),
                "{} {code}",
                row.id
            );
            assert_eq!(result.declaration, route);
            if route.withheld == Some(Withheld::Mount) {
                assert!(result
                    .routes
                    .iter()
                    .all(|route| route.id.path().starts_with("/mugs") || route.id.path() == "/"));
                assert!(!result.routes.is_empty());
            }
            public.extend(
                result
                    .routes
                    .iter()
                    .filter(|r| r.public)
                    .map(|r| r.id.to_string()),
            );
        }
        assert_eq!(
            public.iter().map(String::as_str).collect::<Vec<_>>(),
            row.operations,
            "{}",
            row.id
        );
        count += 1;
    }
    assert_eq!(count, 21);
    let row = rows()
        .find(|row| row.id == "t1-express-array-paths")
        .unwrap();
    let mut route = projection(&row, &inputs[row.id][0]);
    route.uppercase = false;
    route.methods = vec![Some("ProPFIND".into()), None, Some("ProPFIND".into())];
    route.paths = vec!["/mugs/*trail".into(), "/mugs/%7Bpart%7D".into()];
    route.dialect = element::route::Dialect::Express5;
    route.version = Some("5.1.0".into());
    let result = project_route(&route);
    assert_eq!(
        result
            .routes
            .iter()
            .map(|r| r.id.to_string())
            .collect::<Vec<_>>(),
        ["op:ProPFIND /mugs/%7Bpart%7D", "op:ProPFIND /mugs/{*}"]
    );
    assert!(result.limits.iter().any(|l| l.code() == "method_unchecked"));
    let tail = &result.routes[1].path;
    assert_eq!(
        tail.captures().next().unwrap().width,
        CaptureWidth::OneOrMore
    );
    assert!(!tail.accepts("/mugs/"));
    assert!(tail.accepts("/mugs/a/b"));
    assert!(!tail.accepts("/mugs/a//b"));
    route.visibility = Visibility::Internal;
    assert!(project_route(&route).routes.iter().all(|r| !r.public));
}
