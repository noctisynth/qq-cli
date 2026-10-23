//! Bound, read-only target-process socket inspection. Does not establish run ownership.
use crate::core::error::{AppError, Result};
use crate::native::process::Pinned;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};
const MAX_FDS: usize = 8192;

#[repr(C)]
#[derive(Default, PartialEq, Eq)]
struct Snapshot {
    socket_id: u64,
    generation: u64,
    family: u32,
    local_port: u16,
    remote_port: u16,
    local_addr: [u8; 16],
    remote_addr: [u8; 16],
}
const _: () = assert!(std::mem::size_of::<Snapshot>() == 56);
const _: () = assert!(std::mem::offset_of!(Snapshot, local_addr) == 24);
unsafe extern "C" {
    fn qq_cli_tcp_snapshot(pid: i32, fd: i32, out: *mut Snapshot) -> i32;
}
impl Snapshot {
    fn addresses(&self) -> Result<(SocketAddr, SocketAddr)> {
        let ip = |bytes: [u8; 16]| -> Result<IpAddr> {
            match self.family {
                4 => Ok(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]).into()),
                6 => Ok(Ipv6Addr::from(bytes).into()),
                _ => Err(unverifiable()),
            }
        };
        Ok((
            SocketAddr::new(ip(self.local_addr)?, self.local_port),
            SocketAddr::new(ip(self.remote_addr)?, self.remote_port),
        ))
    }
}
/// Consumes the original socket and returns that socket only after proving its peer's identity.
/// No I/O payload or credentials are sent. Caller must obtain `owner` from authenticated run state.
pub fn bind_connected(stream: TcpStream, owner: &Pinned, deadline: Instant) -> Result<TcpStream> {
    crate::core::deadline::check(deadline)?;
    let local = stream.local_addr().map_err(|_| unverifiable())?;
    let peer = stream.peer_addr().map_err(|_| unverifiable())?;
    validate(local, peer)?;
    let wait_until = deadline.min(Instant::now() + Duration::from_secs(2));
    for _ in 0..200 {
        owner.verify(deadline)?;
        if let Some((fd, first)) = find(owner.identity().process_id(), local, peer, deadline)? {
            owner.verify(deadline)?;
            let second = snapshot(owner.identity().process_id(), fd)?.ok_or_else(unverifiable)?;
            if first != second || second.addresses()? != (peer, local) {
                return Err(unverifiable());
            }
            owner.verify(deadline)?;
            if stream.local_addr().map_err(|_| unverifiable())? != local
                || stream.peer_addr().map_err(|_| unverifiable())? != peer
            {
                return Err(unverifiable());
            }
            crate::core::deadline::check(deadline)?;
            return Ok(stream);
        }
        let Some(remaining) = wait_until.checked_duration_since(Instant::now()) else {
            break;
        };
        std::thread::sleep(remaining.min(Duration::from_millis(10)));
    }
    crate::core::deadline::check(deadline)?;
    Err(unverifiable())
}
fn find(
    pid: u32,
    local: SocketAddr,
    peer: SocketAddr,
    deadline: Instant,
) -> Result<Option<(i32, Snapshot)>> {
    // One extra row detects truncation/capacity exhaustion without trusting a racing sizing query.
    let mut fds = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0
        };
        MAX_FDS + 1
    ];
    let size = std::mem::size_of_val(fds.as_slice());
    // SAFETY: exact libc SDK layout, initialized allocation and bounded byte length; no retained pointers.
    let count = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if count <= 0
        || count as usize >= size
        || !(count as usize).is_multiple_of(std::mem::size_of::<libc::proc_fdinfo>())
    {
        return Err(unverifiable());
    }
    let mut found = None;
    for fd in &fds[..count as usize / std::mem::size_of::<libc::proc_fdinfo>()] {
        crate::core::deadline::check(deadline)?;
        if fd.proc_fd < 0 {
            return Err(unverifiable());
        }
        if fd.proc_fdtype != libc::PROX_FDTYPE_SOCKET as u32 {
            continue;
        }
        if let Some(value) = snapshot(pid, fd.proc_fd)?
            && value.addresses()? == (peer, local)
            && found.replace((fd.proc_fd, value)).is_some()
        {
            return Err(unverifiable());
        }
    }
    Ok(found)
}
fn snapshot(pid: u32, fd: i32) -> Result<Option<Snapshot>> {
    let mut output = Snapshot::default();
    // SAFETY: original C adapter writes exactly this repr(C) integer/byte-array struct, retains no pointer.
    match unsafe { qq_cli_tcp_snapshot(pid as i32, fd, &mut output) } {
        0 => Ok(None),
        1 => Ok(Some(output)),
        _ => Err(unverifiable()),
    }
}
fn validate(local: SocketAddr, peer: SocketAddr) -> Result<()> {
    let valid = |addr: SocketAddr| {
        addr.ip().is_loopback()
            && addr.port() != 0
            && match addr {
                SocketAddr::V4(_) => true,
                SocketAddr::V6(v) => v.scope_id() == 0 && v.flowinfo() == 0,
            }
    };
    if !valid(local) || !valid(peer) || local == peer || local.is_ipv4() != peer.is_ipv4() {
        return Err(unverifiable());
    }
    Ok(())
}
fn unverifiable() -> AppError {
    AppError::new(
        "ENDPOINT_UNVERIFIABLE",
        "Connected endpoint could not be bound to the recorded process; no credentials were sent.",
        4,
    )
}
