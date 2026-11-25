use super::*;
use mounts::{compose, Event, Interaction, MountEpoch, Priority, Relation, Target};
use std::cell::Cell;

mod installation;
mod projection;

#[derive(Default)]
struct Synthetic {
    joins: Cell<usize>,
}

impl Relation<(&'static str, &'static str)> for Synthetic {
    fn interact(&self, earlier: &Event<Key>, later: &Event<Key>) -> Interaction {
        if earlier.key.0 != later.key.0 {
            return Interaction::Disjoint;
        }
        let methods = MethodSet::known(["GET"]);
        match later.key.1 {
            "overwrite" => Interaction::Overwrite(methods),
            "reject" => Interaction::Reject(methods),
            "skip" => Interaction::Skip,
            "unknown" | "append" => Interaction::Unknown {
                methods,
                keep_earlier: later.key.1 == "append",
            },
            "contest" => Interaction::Priority {
                methods,
                priority: Priority::Unknown,
            },
            "first" => Interaction::Priority {
                methods,
                priority: Priority::Earlier { entire: true },
            },
            "partial" => Interaction::Priority {
                methods,
                priority: Priority::Later { entire: false },
            },
            _ => Interaction::CoInstall,
        }
    }

    fn join(&self, mount: &Mount, path: &mut Exposure) -> bool {
        self.joins.set(self.joins.get() + 1);
        let ExposureStatus::Full { value } = &mut path.status else {
            return true;
        };
        if mount.end_guard == Some(true) && !value.is_empty() {
            return false;
        }
        if let MountPrefix::Literal { value: prefix } = &mount.prefix {
            value.insert_str(0, prefix);
        }
        true
    }

    fn path_tokens(&self, value: &str) -> Option<Vec<PathToken>> {
        Some(vec![PathToken::Literal(value.into())])
    }
}

type Key = (&'static str, &'static str);

fn leaf(i: usize, dispatcher: usize, value: &str, key: Key) -> Event<Key> {
    Event {
        dispatcher: router(dispatcher),
        setup_root: None,
        site: site(100 + i),
        call: None,
        key,
        target: Target::Route(i, exposure(None, full(value))),
    }
}

fn edge(parent: usize, child: usize, start: usize, prefix: MountPrefix) -> Mount {
    Mount {
        parent: router_receiver(parent),
        child: MountChild {
            receiver: router_receiver(child),
            realm: None,
        },
        prefix,
        ..mount(start, 0, 0)
    }
}

fn prefix(value: &str) -> MountPrefix {
    MountPrefix::Literal {
        value: value.into(),
    }
}

fn install(index: usize, mount: &Mount, mode: MountEpoch) -> Event<Key> {
    let Receiver::Router { identity, .. } = &mount.parent else {
        unreachable!()
    };
    Event {
        dispatcher: identity.clone(),
        setup_root: None,
        site: mount.site.clone(),
        call: None,
        key: ("mount", "co-install"),
        target: Target::Mount(index, mode),
    }
}

fn catalog(events: &[Event<Key>]) -> Vec<Route> {
    let mut routes: Vec<_> = events
        .iter()
        .filter_map(|event| {
            let Target::Route(i, path) = &event.target else {
                return None;
            };
            let spelling = path.status.value().unwrap_or("config.path");
            let mut route = literal_route(spelling, spelling, 100 + i);
            route.receiver = Receiver::Router {
                identity: event.dispatcher.clone(),
                proof: None,
            };
            route.callbacks.push(Callback {
                position: 0,
                status: CallbackStatus::Unresolved {
                    text: "handler".into(),
                    reason: Reason::CallbackUnproven,
                },
                delegate: None,
                role: None,
            });
            Some((*i, route))
        })
        .collect();
    routes.sort_by_key(|(i, _)| *i);
    routes.into_iter().map(|(_, route)| route).collect()
}

fn origin(table: &RouteTable, index: usize) -> &Route {
    table
        .routes()
        .iter()
        .find(|route| route.site == site(100 + index))
        .unwrap()
}

fn literals(table: &RouteTable, index: usize) -> Vec<&str> {
    origin(table, index)
        .paths
        .iter()
        .filter_map(|p| p.status.value())
        .collect()
}
