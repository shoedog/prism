//! Connecting to a daemon's socket without waiting past the shim's deadline (review round 1,
//! sol 4). `UnixStream::connect` blocks, and on Linux a blocking `connect` to a listener whose
//! backlog is full waits for room for as long as it takes: a live daemon that stops accepting
//! would hold every shim for ever. Here the socket is non-blocking while it connects, so a full
//! backlog fails at once (`EAGAIN` on Linux; macOS refuses with `ECONNREFUSED`) and arbitration
//! retries within its deadline; a connection still in progress is waited for up to the deadline.
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Instant;

/// Connect to the socket at `path`, giving up at `deadline`. The stream is returned blocking.
pub(super) fn connect_within(path: &Path, deadline: Instant) -> io::Result<UnixStream> {
    let stream = new_socket()?;
    let (addr, len) = socket_address(path)?;
    // SAFETY: the descriptor is a socket and `addr` a valid `sockaddr_un` of `len` bytes, alive
    // for the call.
    let connected = unsafe {
        libc::connect(
            stream.as_raw_fd(),
            std::ptr::addr_of!(addr).cast::<libc::sockaddr>(),
            len,
        )
    };
    if connected != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error);
        }
        wait_until_connected(&stream, deadline)?;
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}

/// A new Unix stream socket, close-on-exec (so a daemon this shim spawns later does not inherit
/// it) and non-blocking. Linux sets both flags as it creates the socket, as `std` does, so a
/// `fork` on another thread (the lib test binary spawns processes from parallel tests) cannot
/// inherit it in between (review round 2, Opus R2-S2).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn new_socket() -> io::Result<UnixStream> {
    // SAFETY: `socket` takes plain integers and returns a new descriptor or -1.
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` was just returned by `socket` and nothing else owns it.
    Ok(UnixStream::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

/// A new Unix stream socket, close-on-exec and non-blocking. macOS has no `SOCK_CLOEXEC`: the
/// flags are set just after creation, as `std` does there (no other thread of the shim spawns
/// in between).
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn new_socket() -> io::Result<UnixStream> {
    // SAFETY: `socket` takes plain integers and returns a new descriptor or -1.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` was just returned by `socket` and nothing else owns it.
    let stream = UnixStream::from(unsafe { OwnedFd::from_raw_fd(fd) });
    // SAFETY: `F_SETFD` on an open descriptor with a plain flag.
    if unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    stream.set_nonblocking(true)?;
    Ok(stream)
}

/// Wait, up to `deadline`, for a connection in progress to finish, and return its outcome.
fn wait_until_connected(stream: &UnixStream, deadline: Instant) -> io::Result<()> {
    let left = deadline.saturating_duration_since(Instant::now());
    let millis =
        libc::c_int::try_from(left.as_millis().saturating_add(1)).unwrap_or(libc::c_int::MAX);
    let mut poll = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: `poll` reads and writes one valid `pollfd` for the duration of the call.
    let ready = unsafe { libc::poll(&mut poll, 1, millis) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    if ready == 0 {
        return Err(io::Error::from(io::ErrorKind::TimedOut));
    }
    match stream.take_error()? {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The `sockaddr_un` for `path`, refusing a path that does not fit or holds a NUL byte.
fn socket_address(path: &Path) -> io::Result<(libc::sockaddr_un, libc::socklen_t)> {
    // SAFETY: an all-zero `sockaddr_un` is a valid value of the type.
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() >= addr.sun_path.len() || bytes.contains(&0) {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    for (slot, byte) in addr.sun_path.iter_mut().zip(bytes) {
        *slot = *byte as libc::c_char;
    }
    let len = libc::socklen_t::try_from(std::mem::size_of::<libc::sockaddr_un>())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    Ok((addr, len))
}
