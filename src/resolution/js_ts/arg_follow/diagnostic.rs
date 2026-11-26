use super::{bytes, CallSite, FunctionId, ParamSlot, Parameter};
use std::io::{self, Write};

pub(super) enum Output {
    Stderr,
    #[cfg(test)]
    Capture(capture::Token),
}

pub(super) fn enabled() -> Option<Output> {
    #[cfg(test)]
    if let Some(capture) = capture::current() {
        return Some(Output::Capture(capture));
    }
    std::env::var_os("PRISM_E_ARG_FOLLOW_TRACE")
        .is_some_and(|value| value == "1")
        .then_some(Output::Stderr)
}

pub(super) struct Site<'a> {
    site: &'a CallSite,
    output: Option<&'a Output>,
}

impl<'a> Site<'a> {
    pub(super) fn new(site: &'a CallSite, output: Option<&'a Output>) -> Self {
        Self { site, output }
    }

    pub(super) fn report(
        &self,
        parameter: &Parameter<'_>,
        slot: &ParamSlot,
        caller: Option<&CallSite>,
        target: Option<(&FunctionId, (usize, usize))>,
        reason: &str,
    ) {
        let Some(output) = self.output else {
            return;
        };
        let record = serde_json::json!({
            "route": "js_arg_follow",
            "site": occurrence(self.site),
            "holder": slot.holder,
            "holder_span": bytes(parameter.owner),
            "declaration": slot.declaration,
            "position": slot.position,
            "path": slot.path,
            "body_read": parameter.body_read.map(bytes),
            "caller": caller.map(occurrence),
            "target": target.map(|(id, span)| serde_json::json!({"id": id, "span": span})),
            "disposition": if target.is_some() { "admitted" } else { "refused" },
            "reason": reason,
        });
        let line = format!("{record}\n");
        match output {
            Output::Stderr => {
                let _ = io::stderr().lock().write_all(line.as_bytes());
            }
            #[cfg(test)]
            Output::Capture(token) => token.push(line),
        }
    }
}

fn occurrence(site: &CallSite) -> serde_json::Value {
    serde_json::json!({
        "owner": site.caller,
        "span": [site.start_byte, site.end_byte],
        "callee": site.effective_source_callee_name(),
    })
}

#[cfg(test)]
pub(crate) mod capture {
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex, Weak};

    type Sink = Mutex<Vec<String>>;

    #[derive(Clone)]
    pub(crate) struct Token(Weak<Sink>);

    thread_local! {
        static CURRENT: RefCell<Option<Token>> = const { RefCell::new(None) };
    }

    impl Token {
        pub(super) fn push(&self, line: String) {
            if let Some(sink) = self.0.upgrade() {
                sink.lock().unwrap().push(line);
            }
        }
    }

    pub(crate) fn current() -> Option<Token> {
        CURRENT
            .with(|current| current.borrow().clone())
            .filter(|token| token.0.strong_count() > 0)
    }

    pub(crate) fn inherit<R>(capture: Option<Token>, f: impl FnOnce() -> R) -> R {
        struct Reset(Option<Token>);
        impl Drop for Reset {
            fn drop(&mut self) {
                CURRENT.with(|current| current.replace(self.0.take()));
            }
        }
        let _reset = Reset(CURRENT.with(|current| current.replace(capture)));
        f()
    }

    pub(crate) fn run<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
        let sink = Arc::<Sink>::default();
        let facts = inherit(Some(Token(Arc::downgrade(&sink))), f);
        let lines = std::mem::take(&mut *sink.lock().unwrap());
        (facts, lines)
    }
}
