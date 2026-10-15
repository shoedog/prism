//! Who is on the other end of an accepted connection (spec §4.4: "The daemon checks the peer's
//! uid (`getpeereid`) equals its own and drops the connection otherwise"; §7).
//!
//! This check, not the runtime directory's, is the isolation guarantee between users: the
//! directory checks are hardening by path, and two review rounds of S2a each found another way
//! around them. It is not a defense against a process of the same user (spec §2) or root.
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: the descriptor is an open socket, and `credentials` and `len` are valid for
    // writes of the sizes passed, for the duration of the call.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            std::ptr::addr_of_mut!(credentials).cast(),
            &mut len,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

/// The effective uid of the process that connected `stream`, as the kernel recorded it when it
/// connected.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: the descriptor is an open socket, and `uid` and `gid` are valid for writes for
    // the duration of the call.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

/// No peer credential call is wired for this platform: every connection is refused, which is
/// the closed way to fail.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
pub(super) fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no peer credential check on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::shared::runtime_dir::current_uid;

    /// Both ends of a socket pair this process made are this user's.
    #[test]
    fn the_peer_of_a_connection_this_process_made_is_this_user() {
        let (near, far) = UnixStream::pair().expect("a connected socket pair");
        assert_eq!(peer_uid(&near).expect("the peer's uid"), current_uid());
        assert_eq!(peer_uid(&far).expect("the peer's uid"), current_uid());
    }
}
