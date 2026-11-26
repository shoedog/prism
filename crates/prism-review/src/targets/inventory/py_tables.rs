use super::Family;

const VERBS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "request",
];

pub(super) fn family(path: &str) -> Option<Family> {
    let (owner, member) = path.rsplit_once('.')?;
    let http = match owner {
        "requests" => VERBS.contains(&member),
        "httpx" => VERBS.contains(&member) || member == "stream",
        "urllib.request" => matches!(member, "urlopen" | "urlretrieve"),
        "urllib3" | "aiohttp" => member == "request",
        _ => constructor_method(owner, member),
    };
    if http {
        return Some(Family::Http);
    }
    match path {
        "builtins.open" | "io.open" | "os.open" | "os.path.exists" | "os.path.getsize" => {
            Some(Family::Filesystem)
        }
        _ => None,
    }
}

pub(super) fn constructor_method(class: &str, method: &str) -> bool {
    match class {
        "requests.Session" => VERBS.contains(&method) || method == "send",
        "httpx.Client" | "httpx.AsyncClient" => {
            VERBS.contains(&method) || matches!(method, "send" | "stream")
        }
        "aiohttp.ClientSession" => VERBS.contains(&method),
        "urllib3.PoolManager" => matches!(method, "request" | "urlopen"),
        _ => false,
    }
}

pub(super) fn constructor(path: &str) -> bool {
    matches!(
        path,
        "requests.Session"
            | "httpx.Client"
            | "httpx.AsyncClient"
            | "aiohttp.ClientSession"
            | "urllib3.PoolManager"
    )
}

pub(super) fn library(root: &str) -> bool {
    matches!(
        root,
        "requests" | "httpx" | "urllib" | "urllib3" | "aiohttp" | "builtins" | "os" | "io"
    )
}

/// Class namespaces searched for a class-qualified method. The table
/// records the defining owner and the classes traversed to reach it.
pub(super) fn method_owners(owner: &str, method: &str) -> &'static [&'static str] {
    // The Python-level prefixes for the HTTP libraries in eval/uv.lock,
    // through object, which defines their default allocation and lookup.
    if matches!(method, "__new__" | "__getattribute__") {
        return match owner {
            "requests.Session" => &[
                "requests.Session",
                "requests.sessions.SessionRedirectMixin",
                "builtins.object",
            ],
            "httpx.Client" => &[
                "httpx.Client",
                "httpx._client.BaseClient",
                "builtins.object",
            ],
            "httpx.AsyncClient" => &[
                "httpx.AsyncClient",
                "httpx._client.BaseClient",
                "builtins.object",
            ],
            "aiohttp.ClientSession" => &["aiohttp.ClientSession", "builtins.object"],
            "urllib3.PoolManager" => &[
                "urllib3.PoolManager",
                "urllib3._request_methods.RequestMethods",
                "builtins.object",
            ],
            _ => &[],
        };
    }
    match (owner, method) {
        (
            "requests.Session",
            "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "request" | "send"
            | "__init__" | "__enter__" | "__exit__",
        ) => &["requests.Session"],
        ("httpx.Client", _) => &["httpx.Client"],
        ("httpx.AsyncClient", _) => &["httpx.AsyncClient"],
        ("aiohttp.ClientSession", _) => &["aiohttp.ClientSession"],
        ("urllib3.PoolManager", "request") => &[
            "urllib3.PoolManager",
            "urllib3._request_methods.RequestMethods",
        ],
        ("urllib3.PoolManager", "urlopen" | "__init__" | "__enter__" | "__exit__") => {
            &["urllib3.PoolManager"]
        }
        _ => &[],
    }
}

pub(super) fn class_object(path: &str) -> &str {
    match path {
        "requests.sessions.Session" => "requests.Session",
        "httpx._client.Client" => "httpx.Client",
        "httpx._client.AsyncClient" => "httpx.AsyncClient",
        "aiohttp.client.ClientSession" => "aiohttp.ClientSession",
        "urllib3.poolmanager.PoolManager" => "urllib3.PoolManager",
        _ => path,
    }
}
