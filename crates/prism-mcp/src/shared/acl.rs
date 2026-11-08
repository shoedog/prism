//! What kind of access control list a path carries (macOS).
//!
//! On macOS a list is consulted before the permission bits, so an "allow" entry can grant
//! another user what the mode denies: adding and deleting entries of a directory whatever its
//! mode or sticky bit, or changing the mode itself. A "deny" entry can only take access away.
//!
//! On Linux a POSIX list cannot grant more than the group permission bits allow, so the mode
//! checks made elsewhere already bound it; there is nothing to read here.
use std::io;
use std::path::Path;

/// The entries of a path's own list (not of what a symlink there points to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// No list, or a list with no entries.
    None,
    /// Every entry denies.
    DenyOnly,
    /// At least one entry allows something, or is of a kind this code does not know.
    Allows,
}

#[cfg(target_os = "macos")]
pub(crate) fn list_kind(path: &Path) -> io::Result<ListKind> {
    use std::os::raw::{c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    extern "C" {
        fn acl_get_link_np(path: *const c_char, kind: c_int) -> *mut c_void;
        fn acl_get_entry(acl: *mut c_void, entry_id: c_int, entry: *mut *mut c_void) -> c_int;
        fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
        fn acl_free(object: *mut c_void) -> c_int;
    }
    // <sys/acl.h>
    const ACL_TYPE_EXTENDED: c_int = 0x0000_0100;
    const ACL_FIRST_ENTRY: c_int = 0;
    const ACL_NEXT_ENTRY: c_int = -1;
    const ACL_EXTENDED_DENY: c_int = 2;

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
    // SAFETY: `c_path` is a valid NUL-terminated string for the duration of the call; the
    // returned object, when not null, is owned by this function and freed below.
    let acl = unsafe { acl_get_link_np(c_path.as_ptr(), ACL_TYPE_EXTENDED) };
    if acl.is_null() {
        let error = io::Error::last_os_error();
        // A file without a list reports "no such entry".
        return if error.raw_os_error() == Some(libc::ENOENT) {
            Ok(ListKind::None)
        } else {
            Err(error)
        };
    }
    let mut kind = ListKind::None;
    let mut entry: *mut c_void = std::ptr::null_mut();
    let mut which = ACL_FIRST_ENTRY;
    // SAFETY: `acl` is the live object returned above and `entry` is a valid out-pointer. Each
    // entry it receives points into `acl` and is used only before `acl_free`. `acl_get_entry`
    // returns 0 while there is an entry and -1 after the last one.
    while unsafe { acl_get_entry(acl, which, &mut entry) } == 0 {
        which = ACL_NEXT_ENTRY;
        let mut tag: c_int = 0;
        // SAFETY: `entry` was just returned for the live `acl`; `tag` is a valid out-pointer
        // of the enum's size.
        let known = unsafe { acl_get_tag_type(entry, &mut tag) } == 0;
        if known && tag == ACL_EXTENDED_DENY {
            kind = ListKind::DenyOnly;
        } else {
            // An allow entry, or one whose kind could not be read: assume it grants.
            kind = ListKind::Allows;
            break;
        }
    }
    // SAFETY: `acl` came from `acl_get_link_np` and is freed exactly once.
    unsafe { acl_free(acl) };
    Ok(kind)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn list_kind(_path: &Path) -> io::Result<ListKind> {
    Ok(ListKind::None)
}
