//! Navigation entry point and renderers for the route listing.
use super::NavigationSession;
use crate::framework_entries::routes::{routes, EntryError, RootSelector, RouteTable};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum RouteFormat {
    Text,
    Json,
}

pub fn list(session: &NavigationSession, entry: Option<&str>) -> Result<RouteTable, EntryError> {
    list_selected(session, entry.map(RootSelector::parse).transpose()?)
}

/// Parse the selector before opening an index for a CLI query.
pub fn list_with_session<E>(
    entry: Option<&str>,
    open: impl FnOnce() -> Result<NavigationSession, E>,
) -> Result<Result<RouteTable, EntryError>, E> {
    let entry = match entry.map(RootSelector::parse).transpose() {
        Ok(entry) => entry,
        Err(error) => return Ok(Err(error)),
    };
    Ok(list_selected(&open()?, entry))
}

fn list_selected(
    session: &NavigationSession,
    entry: Option<RootSelector>,
) -> Result<RouteTable, EntryError> {
    routes(
        &session.repo.files,
        session.index.call_graph(),
        &session.repo.manifest_snapshot,
        entry,
    )
}

impl RouteFormat {
    pub fn render(self, table: Result<RouteTable, EntryError>, mounts: bool) -> String {
        let mut output = String::new();
        match table {
            Ok(table) => {
                for route in table.routes() {
                    line(&mut output, self, "route", route, false);
                }
                if mounts {
                    for mount in table.mounts() {
                        line(&mut output, self, "mount", mount, true);
                    }
                }
                line(&mut output, self, "summary", table.summary(), true);
            }
            Err(error) => line(&mut output, self, "error", &error, true),
        }
        output
    }
}

fn line(
    out: &mut String,
    format: RouteFormat,
    kind: &str,
    payload: &impl Serialize,
    wrapped: bool,
) {
    match format {
        RouteFormat::Json if wrapped => {
            let record = std::collections::BTreeMap::from([(kind, payload)]);
            writeln!(
                out,
                "{}",
                serde_json::to_string(&record).expect("serializable route record")
            )
            .unwrap();
        }
        RouteFormat::Json => {
            writeln!(
                out,
                "{}",
                serde_json::to_string(payload).expect("serializable route record")
            )
            .unwrap();
        }
        RouteFormat::Text => {
            let Value::Object(fields) =
                serde_json::to_value(payload).expect("serializable route record")
            else {
                unreachable!("route records are objects");
            };
            out.push_str(kind);
            for (field, value) in fields {
                write!(out, " {field}={value}").unwrap();
            }
            out.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{nav_session, NavOptions};
    use crate::framework_entries::routes::{Framework, Reason, Summary};
    use serde_json::json;

    #[test]
    fn renders_routes_mounts_and_summary_in_both_formats() {
        let site = json!({"file":"app.js","start_byte":10,"end_byte":22,"line":3});
        let route = json!({
            "framework":"object-table","protocol":"http","site":site,
            "receiver":{"status":"unproven","reason":"table_consumer_unproven"},
            "candidate":true,"paths":[],"callbacks":[]
        });
        let mount = json!({
            "parent":{"status":"router","file":"app.js","binding":"app",
                      "construction_byte":2,"invocations":[]},
            "child":{"status":"unproven","reason":"mount_child_unproven"},
            "site":site,"prefix":{"status":"absent"},"alternative_ordinal":0,"child_ordinal":0
        });
        let diagnostic = json!({"site":site,"reason":"unsupported_registration"});
        let table = RouteTable::new(
            vec![serde_json::from_value(route.clone()).unwrap()],
            vec![serde_json::from_value(mount.clone()).unwrap()],
            vec![serde_json::from_value(diagnostic.clone()).unwrap()],
            vec![],
            [Framework::ObjectTable],
        );
        let summary = json!({"summary":{
            "enabled_adapters":["object-table"],"registrations":1,"proven":0,"unproven":1,
            "candidates":1,"mounts":1,"diagnostics":[diagnostic],"failed_roots":[]
        }});
        let json_with_mounts = [route.clone(), json!({"mount":mount}), summary.clone()];
        let json_without_mounts = [route, summary];
        let render_json = |mounts| {
            let output = RouteFormat::Json.render(Ok(table.clone()), mounts);
            assert!(output.ends_with('\n'));
            output
                .lines()
                .map(|line| serde_json::from_str::<Value>(line).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(render_json(true), json_with_mounts);
        assert_eq!(render_json(false), json_without_mounts);

        let route_text = concat!(
            "route callbacks=[] candidate=true framework=\"object-table\" paths=[] protocol=\"http\"",
            " receiver={\"reason\":\"table_consumer_unproven\",\"status\":\"unproven\"}",
            " site={\"end_byte\":22,\"file\":\"app.js\",\"line\":3,\"start_byte\":10}\n",
        );
        let mount_text = concat!(
            "mount alternative_ordinal=0 child={\"reason\":\"mount_child_unproven\",\"status\":\"unproven\"}",
            " child_ordinal=0 parent={\"binding\":\"app\",\"construction_byte\":2,\"file\":\"app.js\",\"invocations\":[],\"status\":\"router\"}",
            " prefix={\"status\":\"absent\"} site={\"end_byte\":22,\"file\":\"app.js\",\"line\":3,\"start_byte\":10}\n",
        );
        let summary_text = concat!(
            "summary candidates=1 diagnostics=[{\"reason\":\"unsupported_registration\",",
            "\"site\":{\"end_byte\":22,\"file\":\"app.js\",\"line\":3,\"start_byte\":10}}]",
            " enabled_adapters=[\"object-table\"] failed_roots=[] mounts=1 proven=0 registrations=1 unproven=1\n",
        );
        assert_eq!(
            RouteFormat::Text.render(Ok(table.clone()), true),
            [route_text, mount_text, summary_text].concat()
        );
        assert_eq!(
            RouteFormat::Text.render(Ok(table), false),
            [route_text, summary_text].concat()
        );
    }

    #[test]
    fn malformed_entry_skips_opening_and_renders_only_the_error() {
        let entry = "app:factory(\"args\")";
        let refused = list_with_session(Some(entry), || -> Result<_, ()> {
            panic!("an invalid selector must not open a navigation session")
        })
        .unwrap()
        .unwrap_err();
        assert_eq!(
            refused,
            EntryError {
                reason: Reason::EntryUnproven,
                entry: entry.into()
            }
        );
        assert_eq!(
            RouteFormat::Json.render(Err(refused.clone()), true),
            "{\"error\":{\"reason\":\"entry_unproven\",\"entry\":\"app:factory(\\\"args\\\")\"}}\n"
        );
        assert_eq!(
            RouteFormat::Text.render(Err(refused), false),
            "error entry=\"app:factory(\\\"args\\\")\" reason=\"entry_unproven\"\n"
        );
    }

    #[test]
    fn session_open_failure_is_distinct_from_an_unproven_root() {
        let entry = "app:app";
        assert_eq!(
            list_with_session(Some(entry), || Err("cannot open repo")),
            Err("cannot open repo")
        );

        let repo = tempfile::tempdir().unwrap();
        let options = NavOptions {
            no_cache: true,
            ..NavOptions::default()
        };
        let open = || nav_session(repo.path(), &options);
        let refusal = list_with_session(Some(entry), open).unwrap().unwrap_err();
        assert_eq!(
            refusal,
            EntryError {
                reason: Reason::EntryUnproven,
                entry: entry.into()
            }
        );

        let table = list_with_session(None, open).unwrap().unwrap();
        assert!(table.routes().is_empty());
        assert!(table.mounts().is_empty());
        assert_eq!(
            table.summary(),
            &Summary {
                enabled_adapters: Default::default(),
                registrations: 0,
                proven: 0,
                unproven: 0,
                candidates: 0,
                mounts: 0,
                diagnostics: vec![],
                failed_roots: vec![],
            }
        );
    }
}
