//! Match an already-connected loopback stream to its server-side native TCP owner.
//! System table rows not matching this exact connection are discarded, never logged or persisted.
use super::Job;
use crate::core::error::{AppError, Result};
use crate::native::process::{Identity, Pinned};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Instant;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::*;
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};
const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_ROWS: usize = 8192;
/// No Debug/Serialize: this is a connection capability, not a public endpoint report.
pub struct BoundStream {
    stream: TcpStream,
    owner: Identity,
}
impl BoundStream {
    pub fn owner(&self) -> &Identity {
        &self.owner
    }
    /// HTTP/WS must use this exact connected socket; creating another connection discards the proof.
    pub fn into_stream(self) -> TcpStream {
        self.stream
    }
}
impl Job {
    /// Discovers only this connected stream's server, inside an authenticated run Job.
    /// The expected full image path comes from independent verified launch inputs.
    /// Exact path equality deliberately refuses aliases/case differences rather than guessing.
    /// No protocol bytes or credentials are sent; the returned stream is unchanged.
    pub fn identify_connected(
        &self,
        stream: TcpStream,
        expected_image: &std::path::Path,
        deadline: Instant,
    ) -> Result<(BoundStream, Pinned)> {
        crate::core::deadline::check(deadline)?;
        if !expected_image.is_absolute()
            || expected_image
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control))
        {
            return Err(AppError::invalid());
        }
        let local = stream.local_addr().map_err(|_| bad())?;
        let peer = stream.peer_addr().map_err(|_| bad())?;
        validate(local, peer)?;
        let pid = lookup(local, peer, deadline)?;
        // Never open or inspect a foreign PID just because it owns a supplied port.
        if !self.members(deadline)?.contains(&pid) {
            return Err(mismatch());
        }
        let owner = Pinned::observe(pid, deadline)?;
        if owner.identity().image() != expected_image {
            return Err(mismatch());
        }
        let bound = self.bind_connected(stream, &owner, deadline)?;
        Ok((bound, owner))
    }
    /// Sends/reads no application bytes. Call before any protocol write; caller authenticates
    /// Job/Pinned from private lifecycle state. The returned socket must not be replaced.
    pub fn bind_connected(
        &self,
        stream: TcpStream,
        owner: &Pinned,
        deadline: Instant,
    ) -> Result<BoundStream> {
        crate::core::deadline::check(deadline)?;
        let local = stream.local_addr().map_err(|_| bad())?;
        let peer = stream.peer_addr().map_err(|_| bad())?;
        validate(local, peer)?;
        if !self.contains(owner, deadline)? {
            return Err(mismatch());
        }
        let pid = owner.identity().process_id();
        if lookup(local, peer, deadline)? != pid {
            return Err(mismatch());
        }
        owner.verify(deadline)?;
        if !self.contains(owner, deadline)? || lookup(local, peer, deadline)? != pid {
            return Err(mismatch());
        }
        owner.verify(deadline)?;
        if stream.local_addr().map_err(|_| bad())? != local
            || stream.peer_addr().map_err(|_| bad())? != peer
        {
            return Err(mismatch());
        }
        crate::core::deadline::check(deadline)?;
        Ok(BoundStream {
            stream,
            owner: owner.identity().clone(),
        })
    }
}
fn validate(local: SocketAddr, peer: SocketAddr) -> Result<()> {
    let valid = |addr: SocketAddr| {
        addr.ip().is_loopback()
            && addr.port() != 0
            && match addr {
                SocketAddr::V4(_) => true,
                SocketAddr::V6(v6) => v6.scope_id() == 0 && v6.flowinfo() == 0,
            }
    };
    if !valid(local) || !valid(peer) || local.is_ipv4() != peer.is_ipv4() || local == peer {
        return Err(bad());
    }
    Ok(())
}
fn lookup(local: SocketAddr, peer: SocketAddr, deadline: Instant) -> Result<u32> {
    crate::core::deadline::check(deadline)?;
    let family = if peer.is_ipv4() { AF_INET } else { AF_INET6 };
    let mut size = 0u32;
    // SAFETY: documented sizing call, null table and valid writable byte count; no payload access.
    let code = unsafe {
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            family.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };
    if code != ERROR_INSUFFICIENT_BUFFER.0 {
        return Err(bad());
    }
    for _ in 0..3 {
        crate::core::deadline::check(deadline)?;
        if size < 4 || size as usize > MAX_BYTES {
            return Err(bad());
        }
        let mut storage = vec![0u128; (size as usize).div_ceil(16)];
        size = std::mem::size_of_val(storage.as_slice()) as u32;
        // SAFETY: initialized 16-byte aligned allocation, exact writable capacity passed to SDK;
        // family fixes the all-numeric C row layout used by find below. Buffer lives through call/parse.
        let code = unsafe {
            GetExtendedTcpTable(
                Some(storage.as_mut_ptr().cast()),
                &mut size,
                false,
                family.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if code == ERROR_INSUFFICIENT_BUFFER.0 {
            continue;
        }
        if code != ERROR_SUCCESS.0 {
            return Err(bad());
        }
        let result = find(&storage, size as usize, local, peer)?;
        crate::core::deadline::check(deadline)?;
        return Ok(result);
    }
    Err(bad())
}
fn find(storage: &[u128], bytes: usize, local: SocketAddr, peer: SocketAddr) -> Result<u32> {
    validate(local, peer)?;
    let v4 = peer.is_ipv4();
    let (offset, row_size) = if v4 {
        (
            std::mem::offset_of!(MIB_TCPTABLE_OWNER_PID, table),
            std::mem::size_of::<MIB_TCPROW_OWNER_PID>(),
        )
    } else {
        (
            std::mem::offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
            std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>(),
        )
    };
    if bytes > std::mem::size_of_val(storage) || bytes < offset {
        return Err(bad());
    }
    // SAFETY: the checked initialized allocation contains at least the u32 count header.
    let count = unsafe { *storage.as_ptr().cast::<u32>() } as usize;
    if count > MAX_ROWS || offset + count * row_size > bytes {
        return Err(bad());
    }
    let mut found = None;
    for index in 0..count {
        // SAFETY: complete row extent was checked above. Both SDK layouts contain only integer
        // scalars/byte arrays (all bit patterns valid); read_unaligned avoids padding/alignment assumptions.
        let pid = unsafe {
            let row = storage.as_ptr().cast::<u8>().add(offset + index * row_size);
            if v4 {
                let row = row.cast::<MIB_TCPROW_OWNER_PID>().read_unaligned();
                (row.dwState == 5
                    && IpAddr::V4(Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes())) == peer.ip()
                    && IpAddr::V4(Ipv4Addr::from(row.dwRemoteAddr.to_ne_bytes())) == local.ip()
                    && port(row.dwLocalPort) == peer.port()
                    && port(row.dwRemotePort) == local.port())
                .then_some(row.dwOwningPid)
            } else {
                let row = row.cast::<MIB_TCP6ROW_OWNER_PID>().read_unaligned();
                (row.dwState == 5
                    && IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)) == peer.ip()
                    && IpAddr::V6(Ipv6Addr::from(row.ucRemoteAddr)) == local.ip()
                    && row.dwLocalScopeId == 0
                    && row.dwRemoteScopeId == 0
                    && port(row.dwLocalPort) == peer.port()
                    && port(row.dwRemotePort) == local.port())
                .then_some(row.dwOwningPid)
            }
        };
        if let Some(pid) = pid
            && (pid == 0 || found.replace(pid).is_some())
        {
            return Err(bad());
        }
    }
    found.ok_or_else(bad)
}
fn port(raw: u32) -> u16 {
    let bytes = raw.to_ne_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}
fn bad() -> AppError {
    AppError::new(
        "ENDPOINT_UNVERIFIABLE",
        "Connected loopback endpoint ownership could not be verified; no credentials were sent.",
        4,
    )
}
fn mismatch() -> AppError {
    AppError::new(
        "ENDPOINT_OWNER_MISMATCH",
        "Connected endpoint does not match the verified run process; no credentials were sent.",
        4,
    )
}

#[cfg(test)]
mod tests;
