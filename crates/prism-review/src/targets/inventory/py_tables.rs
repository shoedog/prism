use super::Family;

mod stdlib_classes;
pub(super) use stdlib_classes::instance_classes;

#[cfg(test)]
mod tests;

const VERBS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "request",
];

const PATH_METHODS: &[&str] = &[
    "open",
    "read_text",
    "read_bytes",
    "stat",
    "exists",
    "is_file",
    "is_dir",
    "write_text",
    "write_bytes",
    "mkdir",
    "touch",
    "rename",
    "replace",
    "unlink",
    "rmdir",
    "symlink_to",
];
const CONFIG_PARSER_METHODS: &[&str] = &["read", "read_file", "read_string"];
const METADATA_MEMBERS: &[&str] = &[
    "version",
    "metadata",
    "distribution",
    "files",
    "entry_points",
];
const STDLIB: &[(&str, Family, &[&str])] = &[
    ("builtins", Family::Filesystem, &["open"]),
    ("io", Family::Filesystem, &["open"]),
    ("pathlib.Path", Family::Filesystem, PATH_METHODS),
    (
        "os",
        Family::Filesystem,
        &[
            "stat", "listdir", "scandir", "pathconf", "mkdir", "makedirs", "remove", "unlink",
            "rename", "replace", "rmdir", "write", "fsync", "symlink", "link",
        ],
    ),
    ("os.path", Family::Filesystem, &["getsize", "getmtime"]),
    (
        "shutil",
        Family::Filesystem,
        &["copy", "copy2", "copyfile", "copytree", "move"],
    ),
    (
        "tempfile",
        Family::Filesystem,
        &[
            "mkstemp",
            "mkdtemp",
            "NamedTemporaryFile",
            "TemporaryFile",
            "TemporaryDirectory",
        ],
    ),
    ("json", Family::Parse, &["load", "loads"]),
    ("tomllib", Family::Parse, &["load", "loads"]),
    ("tomli", Family::Parse, &["load", "loads"]),
    (
        "configparser.ConfigParser",
        Family::Parse,
        CONFIG_PARSER_METHODS,
    ),
    (
        "configparser.RawConfigParser",
        Family::Parse,
        CONFIG_PARSER_METHODS,
    ),
    (
        "xml.etree.ElementTree",
        Family::Parse,
        &["parse", "fromstring"],
    ),
    ("xml.dom.minidom", Family::Parse, &["parse", "parseString"]),
    (
        "defusedxml.minidom",
        Family::Parse,
        &["parse", "parseString"],
    ),
    ("pickle", Family::Parse, &["load", "loads"]),
    ("os", Family::Config, &["getenv"]),
    ("os.environ", Family::Config, &["get"]),
    ("importlib.metadata", Family::Config, METADATA_MEMBERS),
    ("importlib_metadata", Family::Config, METADATA_MEMBERS),
    ("pathlib.Path", Family::Config, &["home"]),
    (
        "locale",
        Family::Config,
        &["getpreferredencoding", "getencoding"],
    ),
];

pub(super) fn family(path: &str) -> Option<Family> {
    let (owner, member) = path.rsplit_once('.')?;
    let http = match owner {
        "requests" => VERBS.contains(&member),
        "httpx" => VERBS.contains(&member) || member == "stream",
        "urllib.request" => matches!(member, "urlopen" | "urlretrieve"),
        "urllib3" | "aiohttp" => member == "request",
        _ => http_method(owner, member),
    };
    if http {
        return Some(Family::Http);
    }
    stdlib_family(owner, member)
}

fn stdlib_family(owner: &str, member: &str) -> Option<Family> {
    STDLIB.iter().find_map(|&(namespace, family, members)| {
        (namespace == owner && members.contains(&member)).then_some(family)
    })
}

pub(super) fn method_family(class: &str, method: &str) -> Option<Family> {
    if http_method(class, method) {
        Some(Family::Http)
    } else if stdlib_classes::constructor(class) {
        stdlib_family(class, method)
    } else {
        None
    }
}

pub(super) fn constructor_method(class: &str, method: &str) -> bool {
    method_family(class, method).is_some()
}

fn http_method(class: &str, method: &str) -> bool {
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
    ) || stdlib_classes::constructor(path)
}

pub(super) fn declaration_only_class(path: &str) -> bool {
    stdlib_classes::constructor(path)
}

pub(super) fn context_class(class: &str, asynchronous: bool) -> bool {
    if asynchronous {
        matches!(class, "httpx.AsyncClient" | "aiohttp.ClientSession")
    } else {
        matches!(
            class,
            "requests.Session" | "httpx.Client" | "urllib3.PoolManager" | "pathlib.Path"
        )
    }
}

pub(super) fn library(root: &str) -> bool {
    matches!(
        root,
        "requests" | "httpx" | "urllib" | "urllib3" | "aiohttp"
    ) || STDLIB.iter().any(|&(owner, _, _)| {
        owner == root || owner.split_once('.').is_some_and(|(top, _)| top == root)
    })
}

/// Class namespaces searched for a class-qualified method. The table
/// records the defining owner and the classes traversed to reach it.
pub(super) fn method_owners(owner: &str, method: &str) -> &'static [&'static str] {
    if let Some(owners) = stdlib_classes::method_owners(owner, method) {
        return owners;
    }
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
        "configparser.MutableMapping"
        | "_collections_abc.MutableMapping"
        | "os.MutableMapping"
        | "typing.MutableMapping" => "collections.abc.MutableMapping",
        "_collections_abc.Mapping"
        | "cgi.Mapping"
        | "os.Mapping"
        | "selectors.Mapping"
        | "typing.Mapping" => "collections.abc.Mapping",
        "_collections_abc.Collection" => "collections.abc.Collection",
        "_collections_abc.Sized" => "collections.abc.Sized",
        "_collections_abc.Iterable" => "collections.abc.Iterable",
        "_collections_abc.Container" => "collections.abc.Container",
        _ => path,
    }
}

pub(super) fn typing_mapping_alias(path: &str) -> bool {
    matches!(path, "typing.Mapping" | "typing.MutableMapping")
}

pub(super) fn stdlib_class_or_typing_alias(path: &str) -> bool {
    stdlib_classes::contains(class_object(path))
}
