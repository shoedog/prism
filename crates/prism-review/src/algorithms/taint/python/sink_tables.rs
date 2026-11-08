//! Python structured sink tables.

use prism::frameworks::{SanitizerCategory, SinkPattern};

pub const PY_CWE79_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "mark_safe",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Markup",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "markupsafe.Markup",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "format_html",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "render_template_string",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
];

pub const PY_CWE89_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "execute",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "executemany",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "raw",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const PY_CWE918_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "requests.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib.request.urlopen",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib.request.Request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib3.PoolManager.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
];

pub const PY_CWE502_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "pickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "pickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cPickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cPickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cloudpickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cloudpickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "yaml.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "jsonpickle.decode",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "marshal.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "marshal.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dill.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dill.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];
