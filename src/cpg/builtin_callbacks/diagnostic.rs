use super::*;
use std::io::{self, Write};

const PREFIX: &str = "PRISM_D_BUILTIN_TRACE ";

#[derive(Clone, Copy)]
pub(in crate::cpg::build) enum Output {
    Stderr,
    #[cfg(test)]
    Capture(u64),
}

pub(super) struct Decision<'a> {
    site: &'a CallSite,
    output: Output,
    pub(super) receiver: Option<(usize, usize)>,
    pub(super) receiver_path: Option<String>,
    pub(super) receiver_use: Option<serde_json::Value>,
    pub(super) representation: Option<&'static str>,
    pub(super) callback: Option<(usize, usize)>,
    pub(super) declaration: Option<(usize, usize)>,
    pub(super) implementation: Option<(usize, usize)>,
    pub(super) owner: Option<FunctionId>,
    pub(super) method: Option<String>,
}

impl<'a> Decision<'a> {
    pub(super) fn new(site: &'a CallSite, output: Output) -> Self {
        Self {
            site,
            output,
            receiver: None,
            receiver_path: None,
            receiver_use: None,
            representation: None,
            callback: None,
            declaration: None,
            implementation: None,
            owner: None,
            method: None,
        }
    }

    pub(super) fn report(
        &self,
        disposition: &str,
        reason: &str,
        role: Option<(&str, usize)>,
        formal: Option<&CpgNode>,
    ) {
        let record = serde_json::json!({
            "route": "builtin_callback",
            "caller_file": self.site.caller.file,
            "caller": self.site.caller.name,
            "caller_lines": [self.site.caller.start_line, self.site.caller.end_line],
            "call": [self.site.start_byte, self.site.end_byte],
            "callee": self.site.callee_name,
            "method": self.method,
            "receiver": self.receiver,
            "receiver_path": self.receiver_path,
            "receiver_use": self.receiver_use,
            "receiver_representation": self.representation,
            "callback": self.callback,
            "declaration": self.declaration,
            "implementation": self.implementation,
            "target_file": self.owner.as_ref().map(|id| &id.file),
            "target_owner": self.owner.as_ref().map(|id| &id.name),
            "role": role.map(|(name, _)| name),
            "slot": role.map(|(_, slot)| slot),
            "formal": formal.and_then(endpoint),
            "disposition": disposition,
            "reason": reason,
        });
        let line = format!("{PREFIX}{record}\n");
        match self.output {
            Output::Stderr => {
                let _ = io::stderr().lock().write_all(line.as_bytes());
            }
            #[cfg(test)]
            Output::Capture(id) => capture::push(id, line),
        }
    }
}

pub(super) fn endpoint(node: &CpgNode) -> Option<serde_json::Value> {
    let CpgNode::Variable {
        file,
        function,
        function_start_line,
        path,
        access,
        start_byte,
        end_byte,
        ..
    } = node
    else {
        return None;
    };
    Some(serde_json::json!({
        "file": file, "owner": function, "owner_line": function_start_line,
        "path": path, "access": access, "span": [start_byte, end_byte],
    }))
}

pub(in crate::cpg::build) fn enabled() -> Option<Output> {
    #[cfg(test)]
    if let Some(id) = capture::active_session() {
        return Some(Output::Capture(id));
    }
    std::env::var_os("PRISM_D_BUILTIN_TRACE")
        .is_some_and(|value| value == "1")
        .then_some(Output::Stderr)
}

#[cfg(test)]
pub(in crate::cpg) mod capture {
    use std::sync::Mutex;

    struct Session {
        id: u64,
        lines: Vec<String>,
    }

    static ACTIVE: Mutex<Option<Session>> = Mutex::new(None);
    static SERIAL: Mutex<u64> = Mutex::new(0);

    pub(super) fn active_session() -> Option<u64> {
        ACTIVE.lock().unwrap().as_ref().map(|session| session.id)
    }

    pub(super) fn push(id: u64, line: String) {
        if let Some(session) = ACTIVE
            .lock()
            .unwrap()
            .as_mut()
            .filter(|session| session.id == id)
        {
            session.lines.push(line);
        }
    }

    pub(in crate::cpg) fn run<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                ACTIVE.lock().unwrap().take();
            }
        }
        let mut generation = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        *generation += 1;
        *ACTIVE.lock().unwrap() = Some(Session {
            id: *generation,
            lines: Vec::new(),
        });
        let _reset = Reset;
        let result = f();
        let lines = ACTIVE.lock().unwrap().take().unwrap().lines;
        (result, lines)
    }
}

#[cfg(test)]
#[test]
fn param_binding_builtin_diagnostic_cannot_report_to_a_later_capture() {
    use crate::cpg::param_binding_builtin_tests::DIRECT;
    use crate::languages::Language;

    const MARKER: &str = "expired_capture_decision";
    let file = "capture-session.js";
    let parsed = ParsedFile::parse(file, DIRECT, Language::JavaScript).unwrap();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.to_owned(), parsed)]));
    let call = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "forEach")
        .unwrap();
    let (decision, first) = capture::run(|| {
        let decision = Decision::new(call, enabled().unwrap());
        decision.report("refused", "active_capture_decision", None, None);
        decision
    });
    assert!(first
        .iter()
        .any(|line| line.contains("active_capture_decision")));
    decision.report("refused", "expired_capture_stderr_probe", None, None);
    let (_, second) = capture::run(|| decision.report("refused", MARKER, None, None));
    assert!(
        !second.iter().any(|line| line.contains(MARKER)),
        "{second:?}"
    );
}
