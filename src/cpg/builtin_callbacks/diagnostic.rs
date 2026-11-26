use super::*;
use std::io::{self, Write};

const PREFIX: &str = "PRISM_D_BUILTIN_TRACE ";

pub(super) struct Decision<'a> {
    site: &'a CallSite,
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
    pub(super) fn new(site: &'a CallSite) -> Option<Self> {
        if !enabled() {
            return None;
        }
        Some(Self {
            site,
            receiver: None,
            receiver_path: None,
            receiver_use: None,
            representation: None,
            callback: None,
            declaration: None,
            implementation: None,
            owner: None,
            method: None,
        })
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
        #[cfg(test)]
        if capture::push(&line) {
            return;
        }
        let _ = io::stderr().lock().write_all(line.as_bytes());
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

pub(in crate::cpg::build) fn enabled() -> bool {
    #[cfg(test)]
    if capture::active() {
        return true;
    }
    std::env::var_os("PRISM_D_BUILTIN_TRACE").is_some_and(|value| value == "1")
}

#[cfg(test)]
pub(in crate::cpg) mod capture {
    use std::sync::Mutex;

    static LINES: Mutex<Option<Vec<String>>> = Mutex::new(None);
    static SERIAL: Mutex<()> = Mutex::new(());

    pub(super) fn active() -> bool {
        LINES.lock().unwrap().is_some()
    }

    pub(super) fn push(line: &str) -> bool {
        let mut sink = LINES.lock().unwrap();
        if let Some(lines) = &mut *sink {
            lines.push(line.to_owned());
            return true;
        }
        false
    }

    pub(in crate::cpg) fn run<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                LINES.lock().unwrap().take();
            }
        }
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        *LINES.lock().unwrap() = Some(Vec::new());
        let _reset = Reset;
        let result = f();
        let lines = LINES.lock().unwrap().take().unwrap();
        (result, lines)
    }
}
