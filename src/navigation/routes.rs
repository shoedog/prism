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
    routes(
        &session.repo.files,
        session.index.call_graph(),
        &session.repo.manifest_snapshot,
        entry.map(RootSelector::parse).transpose()?,
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
