use super::*;
use mounts::{compose, Event, Interaction, MountEpoch, MountKey, Priority, Relation, Target};
use std::cell::Cell;

mod installation;
mod projection;

#[derive(Default)]
struct Synthetic {
    joins: Cell<usize>,
}

impl Relation<(&'static str, &'static str)> for Synthetic {
    fn interact(&self, earlier: &Event<Key>, later: &Event<Key>) -> Interaction {
        if later.key.1 == "mix" {
            return match earlier.key.0 {
                "a" => Interaction::Overwrite(MethodSet::known(["GET"])),
                "b" => Interaction::Reject(MethodSet::known(["GET"])),
                _ => Interaction::Disjoint,
            };
        }
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

    fn join(&self, mount: &Mount, path: &mut Exposure, _: &MethodSet) -> bool {
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

    fn mount_key(&self, mount: &Mount, joined: &Exposure, methods: &MethodSet) -> Option<MountKey> {
        (mount.site == site(8)
            && joined.status.value() == Some("/api/x")
            && methods == &MethodSet::known(["GET"]))
            .then_some(MountKey(1))
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
        rejection: None,
        target: Target::Route {
            record: i,
            path: exposure(None, full(value)),
            effective_methods: match key.0 {
                "GET" => MethodSet::known(["GET", "HEAD"]),
                "POST" => MethodSet::known(["POST"]),
                "child-x" | "child-y" => MethodSet::known(["GET"]),
                _ => MethodSet::ANY,
            },
        },
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
        rejection: None,
        target: Target::Mount {
            record: index,
            epoch: mode,
            key: None,
        },
    }
}

fn catalog(events: &[Event<Key>]) -> Vec<Route> {
    let mut routes: Vec<_> = events
        .iter()
        .filter_map(|event| {
            let Target::Route { record, path, .. } = &event.target else {
                return None;
            };
            let spelling = path.status.value().unwrap_or("config.path");
            let mut route = literal_route(spelling, spelling, 100 + record);
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
            Some((*record, route))
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

mod fold;
