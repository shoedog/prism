use super::Family;

const VERBS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "request",
];

pub(super) fn family(path: &str) -> Option<Family> {
    let (owner, member) = path.rsplit_once('.')?;
    let http = match owner {
        "requests" | "aiohttp.ClientSession" => VERBS.contains(&member),
        "httpx" => VERBS.contains(&member) || member == "stream",
        "urllib.request" => matches!(member, "urlopen" | "urlretrieve"),
        "urllib3" | "aiohttp" => member == "request",
        "requests.Session" => VERBS.contains(&member) || member == "send",
        "httpx.Client" | "httpx.AsyncClient" => {
            VERBS.contains(&member) || matches!(member, "send" | "stream")
        }
        "urllib3.PoolManager" => matches!(member, "request" | "urlopen"),
        _ => false,
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

pub(super) fn library(root: &str) -> bool {
    matches!(
        root,
        "requests" | "httpx" | "urllib" | "urllib3" | "aiohttp" | "builtins" | "os" | "io"
    )
}

/// Class namespaces searched for a class-qualified method. The table
/// records the defining owner and the classes traversed to reach it.
pub(super) fn method_owners(owner: &str, method: &str) -> &'static [&'static str] {
    match (owner, method) {
        (
            "requests.Session",
            "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "request" | "send",
        ) => &["requests.Session"],
        ("httpx.Client", _) => &["httpx.Client"],
        ("httpx.AsyncClient", _) => &["httpx.AsyncClient"],
        ("aiohttp.ClientSession", _) => &["aiohttp.ClientSession"],
        ("urllib3.PoolManager", "request") => &[
            "urllib3.PoolManager",
            "urllib3._request_methods.RequestMethods",
        ],
        ("urllib3.PoolManager", "urlopen") => &["urllib3.PoolManager"],
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
