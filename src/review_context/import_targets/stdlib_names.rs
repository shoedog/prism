//! Top-level module names of the CPython 3.12 standard library
//! (`sys.stdlib_module_names`, including the platform-specific ones). A static
//! snapshot: a name absent here is treated as non-stdlib, so a newer standard
//! module can be attributed to a same-named repo file (a candidate, not a proof).

const STDLIB_TOP_LEVEL: &str = "
    __future__ _abc _aix_support _ast _asyncio _bisect _blake2 _bz2 _codecs _codecs_cn
    _codecs_hk _codecs_iso2022 _codecs_jp _codecs_kr _codecs_tw _collections
    _collections_abc _compat_pickle _compression _contextvars _crypt _csv _ctypes _curses
    _curses_panel _datetime _dbm _decimal _elementtree _frozen_importlib
    _frozen_importlib_external _functools _gdbm _hashlib _heapq _imp _io _json _locale
    _lsprof _lzma _markupbase _md5 _msi _multibytecodec _multiprocessing _opcode _operator
    _osx_support _overlapped _pickle _posixshmem _posixsubprocess _py_abc _pydatetime
    _pydecimal _pyio _pylong _queue _random _scproxy _sha1 _sha2 _sha3 _signal _sitebuiltins
    _socket _sqlite3 _sre _ssl _stat _statistics _string _strptime _struct _symtable _thread
    _threading_local _tkinter _tokenize _tracemalloc _typing _uuid _warnings _weakref
    _weakrefset _winapi _wmi _zoneinfo abc aifc antigravity argparse array ast asyncio
    atexit audioop base64 bdb binascii bisect builtins bz2 cProfile calendar cgi cgitb chunk
    cmath cmd code codecs codeop collections colorsys compileall concurrent configparser
    contextlib contextvars copy copyreg crypt csv ctypes curses dataclasses datetime dbm
    decimal difflib dis doctest email encodings ensurepip enum errno faulthandler fcntl
    filecmp fileinput fnmatch fractions ftplib functools gc genericpath getopt getpass
    gettext glob graphlib grp gzip hashlib heapq hmac html http idlelib imaplib imghdr
    importlib inspect io ipaddress itertools json keyword lib2to3 linecache locale logging
    lzma mailbox mailcap marshal math mimetypes mmap modulefinder msilib msvcrt
    multiprocessing netrc nis nntplib nt ntpath nturl2path numbers opcode operator optparse
    os ossaudiodev pathlib pdb pickle pickletools pipes pkgutil platform plistlib poplib
    posix posixpath pprint profile pstats pty pwd py_compile pyclbr pydoc pydoc_data pyexpat
    queue quopri random re readline reprlib resource rlcompleter runpy sched secrets select
    selectors shelve shlex shutil signal site smtplib sndhdr socket socketserver spwd
    sqlite3 sre_compile sre_constants sre_parse ssl stat statistics string stringprep struct
    subprocess sunau symtable sys sysconfig syslog tabnanny tarfile telnetlib tempfile
    termios textwrap this threading time timeit tkinter token tokenize tomllib trace
    traceback tracemalloc tty turtle turtledemo types typing unicodedata unittest urllib uu
    uuid venv warnings wave weakref webbrowser winreg winsound wsgiref xdrlib xml xmlrpc
    zipapp zipfile zipimport zlib zoneinfo
";

/// Modules CPython 3.12 compiles into the interpreter (`sys.builtin_module_names`,
/// the union over platforms) or freezes (`_frozen_importlib`,
/// `_frozen_importlib_external`, `zipimport`). The finders consult them before
/// any path entry, so no repo file shadows them.
const BUILTIN_OR_FROZEN: &str = "
    _abc _ast _codecs _collections _frozen_importlib _frozen_importlib_external _functools
    _imp _io _locale _operator _overlapped _signal _sre _stat _string _symtable _thread
    _tokenize _tracemalloc _typing _warnings _weakref _winapi atexit builtins errno
    faulthandler gc itertools marshal msvcrt nt posix pwd sys time winreg xxsubtype zipimport
";

/// Whether `top` is a built-in or frozen module: always the interpreter's own,
/// whatever sits on `sys.path`.
pub(super) fn is_builtin_or_frozen_module(top: &str) -> bool {
    BUILTIN_OR_FROZEN.split_whitespace().any(|name| name == top)
}

/// Whether `top` (the first dotted component of an absolute import) names a
/// standard-library module.
pub(super) fn is_stdlib_module(top: &str) -> bool {
    STDLIB_TOP_LEVEL.split_whitespace().any(|name| name == top)
}

#[cfg(test)]
mod tests {
    use super::{is_builtin_or_frozen_module, is_stdlib_module};

    #[test]
    fn builtin_names_are_a_subset_of_the_stdlib_names() {
        for name in [
            "sys",
            "builtins",
            "_io",
            "marshal",
            "posix",
            "_frozen_importlib",
            "zipimport",
        ] {
            assert!(is_builtin_or_frozen_module(name), "{name}");
            assert!(is_stdlib_module(name), "{name}");
        }
        for name in ["json", "os", "re", "sy", "pkg", ""] {
            assert!(!is_builtin_or_frozen_module(name), "{name}");
        }
    }

    #[test]
    fn matches_whole_top_level_names_only() {
        for name in [
            "json",
            "os",
            "logging",
            "subprocess",
            "typing",
            "__future__",
        ] {
            assert!(is_stdlib_module(name), "{name}");
        }
        for name in ["jso", "pkg", "myapp", "requests", "os.path", ""] {
            assert!(!is_stdlib_module(name), "{name}");
        }
    }
}
