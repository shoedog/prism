use super::{CONFIG_PARSER_METHODS, PATH_METHODS};

const PATH: &str = "pathlib.Path";
const POSIX_PATH: &str = "pathlib.PosixPath";
const WINDOWS_PATH: &str = "pathlib.WindowsPath";
const CONFIG: &str = "configparser.ConfigParser";
const RAW_CONFIG: &str = "configparser.RawConfigParser";
const ENVIRON: &str = "os._Environ";

// CPython 3.11.11, Lib/pathlib.py and Lib/configparser.py. Record the
// full MRO once, then stop at the defining owner for each required read.
const PATH_MRO: &[&str] = &[PATH, "pathlib.PurePath", "builtins.object"];
const POSIX_MRO: &[&str] = &[
    POSIX_PATH,
    PATH,
    "pathlib.PurePosixPath",
    "pathlib.PurePath",
    "builtins.object",
];
const WINDOWS_MRO: &[&str] = &[
    WINDOWS_PATH,
    PATH,
    "pathlib.PureWindowsPath",
    "pathlib.PurePath",
    "builtins.object",
];
const CONFIG_MRO: &[&str] = &[
    CONFIG,
    RAW_CONFIG,
    "collections.abc.MutableMapping",
    "collections.abc.Mapping",
    "collections.abc.Collection",
    "collections.abc.Sized",
    "collections.abc.Iterable",
    "collections.abc.Container",
    "builtins.object",
];
// CPython 3.11 Lib/os.py and Lib/_collections_abc.py.
const ENVIRON_MRO: &[&str] = &[
    ENVIRON,
    "collections.abc.MutableMapping",
    "collections.abc.Mapping",
    "collections.abc.Collection",
    "collections.abc.Sized",
    "collections.abc.Iterable",
    "collections.abc.Container",
    "builtins.object",
];

pub(super) fn contains(class: &str) -> bool {
    [POSIX_MRO, WINDOWS_MRO, CONFIG_MRO, ENVIRON_MRO]
        .iter()
        .any(|mro| mro.contains(&class))
}

pub(super) fn constructor(class: &str) -> bool {
    matches!(class, PATH | CONFIG | RAW_CONFIG)
}

pub(in crate::targets::inventory) fn instance_classes(class: &str) -> &[&str] {
    if class == PATH {
        &[PATH, POSIX_PATH, WINDOWS_PATH]
    } else {
        &[]
    }
}

pub(super) fn method_owners(class: &str, member: &str) -> Option<&'static [&'static str]> {
    let (mro, class_owner) = match class {
        PATH => (PATH_MRO, 1),
        POSIX_PATH => (POSIX_MRO, 2),
        WINDOWS_PATH => (WINDOWS_MRO, 2),
        CONFIG => (CONFIG_MRO, 2),
        RAW_CONFIG => (&CONFIG_MRO[1..], 1),
        ENVIRON => (ENVIRON_MRO, 3),
        _ => return None,
    };
    let end = match class {
        PATH | POSIX_PATH | WINDOWS_PATH => match member {
            "__getattribute__" | "__init__" => mro.len(),
            "__new__" | "__enter__" | "__exit__" | "home" => class_owner,
            _ if PATH_METHODS.contains(&member) => class_owner,
            _ => return None,
        },
        ENVIRON => match member {
            "__getattribute__" => mro.len(),
            "get" => class_owner,
            _ => return None,
        },
        _ => match member {
            "__new__" | "__getattribute__" => mro.len(),
            "__init__" => class_owner,
            _ if CONFIG_PARSER_METHODS.contains(&member) => class_owner,
            _ => return None,
        },
    };
    Some(&mro[..end])
}
