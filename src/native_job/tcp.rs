//! Match an already-connected loopback stream to its server-side native TCP owner.
//! System table rows not matching this exact connection are discarded, never logged or persisted.
use super::Job;
use crate::error::{AppError, Result};
use crate::process::{Identity, Pinned};
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
        crate::deadline::check(deadline)?;
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
        crate::deadline::check(deadline)?;
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
        crate::deadline::check(deadline)?;
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
    crate::deadline::check(deadline)?;
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
        crate::deadline::check(deadline)?;
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
        crate::deadline::check(deadline)?;
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
mod tests {
    use super::*;
    use crate::native_job::SpawnSpec;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::io::Read;
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }
    fn raw_port(port: u16) -> u32 {
        u32::from_ne_bytes([port.to_be_bytes()[0], port.to_be_bytes()[1], 0, 0])
    }
    fn table4(rows: &[MIB_TCPROW_OWNER_PID]) -> (Vec<u128>, usize) {
        let offset = std::mem::offset_of!(MIB_TCPTABLE_OWNER_PID, table);
        let bytes = offset + std::mem::size_of_val(rows);
        let mut buffer = vec![0u128; bytes.div_ceil(16)];
        // SAFETY: original all-u32 SDK rows have no padding, and the initialized destination is
        // sized for the header/complete rows. This helper never consumes external native memory.
        unsafe {
            *buffer.as_mut_ptr().cast::<u32>() = rows.len() as u32;
            std::ptr::copy_nonoverlapping(
                rows.as_ptr().cast::<u8>(),
                buffer.as_mut_ptr().cast::<u8>().add(offset),
                std::mem::size_of_val(rows),
            );
        }
        (buffer, bytes)
    }
    #[test]
    fn parser_rejects_incomplete_ambiguous_or_nonestablished_tuples_and_preserves_byte_order() {
        let local: SocketAddr = "127.0.0.2:24001".parse().unwrap();
        let peer: SocketAddr = "127.0.0.1:24002".parse().unwrap();
        let row = MIB_TCPROW_OWNER_PID {
            dwState: 5,
            dwLocalAddr: u32::from_ne_bytes([127, 0, 0, 1]),
            dwLocalPort: raw_port(peer.port()) | 0xabcd0000,
            dwRemoteAddr: u32::from_ne_bytes([127, 0, 0, 2]),
            dwRemotePort: raw_port(local.port()),
            dwOwningPid: 20001,
        };
        let (table, bytes) = table4(&[row]);
        assert_eq!(find(&table, bytes, local, peer).unwrap(), 20001);
        assert!(find(&table, bytes, peer, local).is_err());
        assert!(find(&table, bytes - 1, local, peer).is_err());
        let (table, bytes) = table4(&[row, row]);
        assert!(find(&table, bytes, local, peer).is_err());
        let (table, bytes) = table4(&[MIB_TCPROW_OWNER_PID { dwState: 2, ..row }]);
        assert!(find(&table, bytes, local, peer).is_err());
        let (table, bytes) = table4(&[MIB_TCPROW_OWNER_PID {
            dwOwningPid: 0,
            ..row
        }]);
        assert!(find(&table, bytes, local, peer).is_err());
        assert!(find(&[], 0, local, peer).is_err());
        let (mut table, bytes) = table4(&[row]);
        // SAFETY: same initialized original count header, replaced with a deliberately excessive count.
        unsafe {
            *table.as_mut_ptr().cast::<u32>() = u32::MAX;
        }
        assert!(find(&table, bytes, local, peer).is_err());
        assert!(validate(local, local).is_err());
        assert!(validate(local, "192.0.2.1:24002".parse().unwrap()).is_err());
    }
    #[test]
    fn native_ipv4_and_ipv6_snapshots_match_only_the_exact_existing_server_connection() {
        for ip in [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ] {
            let listener = TcpListener::bind(SocketAddr::new(ip, 0)).unwrap();
            let client =
                TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
                    .unwrap();
            let (mut server, _) = listener.accept().unwrap();
            assert_eq!(
                lookup(
                    client.local_addr().unwrap(),
                    client.peer_addr().unwrap(),
                    deadline()
                )
                .unwrap(),
                std::process::id()
            );
            server.set_nonblocking(true).unwrap();
            assert_eq!(
                server.read(&mut [0]).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            assert_eq!(
                lookup(
                    client.local_addr().unwrap(),
                    client.peer_addr().unwrap(),
                    Instant::now()
                )
                .unwrap_err()
                .code,
                "DEADLINE"
            );
        }
    }
    fn wait_file(path: &Path) {
        let until = deadline();
        while !path.exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            path.exists(),
            "original loopback service did not publish its handshake"
        );
    }
    #[test]
    fn binding_uses_owned_server_pid_on_the_same_socket_and_rejects_foreign_connections_without_data()
     {
        const MODE: &str = "QQ_CLI_TEST_TCP_CHILD";
        const ROOT: &str = "QQ_CLI_TEST_TCP_ROOT";
        const NAME: &str = "native_job::tcp::tests::binding_uses_owned_server_pid_on_the_same_socket_and_rejects_foreign_connections_without_data";
        if std::env::var_os(MODE).is_some() {
            let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
            let listeners = [
                TcpListener::bind("127.0.0.1:0").unwrap(),
                TcpListener::bind("[::1]:0").unwrap(),
            ];
            for listener in &listeners {
                listener.set_nonblocking(true).unwrap();
            }
            let addresses: Vec<SocketAddr> =
                listeners.iter().map(|s| s.local_addr().unwrap()).collect();
            crate::state::atomic_json(&root.join("endpoints.json"), &addresses).unwrap();
            let mut sockets = Vec::new();
            let mut bytes = 0usize;
            let until = Instant::now() + Duration::from_secs(15);
            while !root.join("release").exists() && Instant::now() < until {
                for listener in &listeners {
                    loop {
                        match listener.accept() {
                            Ok((socket, _)) => {
                                socket.set_nonblocking(true).unwrap();
                                sockets.push(socket);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(_) => panic!("original listener failed"),
                        }
                    }
                }
                for socket in &mut sockets {
                    match socket.read(&mut [0; 64]) {
                        Ok(n) => bytes += n,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::ConnectionReset
                                    | std::io::ErrorKind::ConnectionAborted
                            ) => {}
                        Err(_) => panic!("original accepted socket failed"),
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            crate::state::atomic_json(&root.join("received.json"), &bytes).unwrap();
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        struct Release(PathBuf);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = std::fs::write(&self.0, b"original release");
            }
        }
        let release = Release(root.join("release"));
        let job = Job::create(deadline()).unwrap();
        let mut environment: BTreeMap<OsString, OsString> = BTreeMap::from([
            (MODE.into(), "1".into()),
            (ROOT.into(), root.as_os_str().to_owned()),
        ]);
        if let Some(system) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), system);
        }
        let executable = std::env::current_exe().unwrap();
        let args = ["--exact".into(), NAME.into(), "--test-threads=1".into()];
        let server = job
            .spawn(
                &SpawnSpec {
                    program: &executable,
                    arguments: &args,
                    environment: &environment,
                    directory: root,
                },
                deadline(),
            )
            .unwrap();
        wait_file(&root.join("endpoints.json"));
        let endpoints: Vec<SocketAddr> =
            crate::config::read_json(&root.join("endpoints.json"), 4096).unwrap();
        assert_eq!(endpoints.len(), 2);
        for endpoint in endpoints {
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                job.identify_connected(stream, server.process().identity().image(), Instant::now())
                    .err()
                    .unwrap()
                    .code,
                "DEADLINE"
            );
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            let local = stream.local_addr().unwrap();
            let (bound, discovered) = job
                .identify_connected(stream, server.process().identity().image(), deadline())
                .unwrap();
            assert!(discovered.identity() == server.process().identity());
            let stream = bound.into_stream();
            assert_eq!(stream.local_addr().unwrap(), local);
            assert_eq!(stream.peer_addr().unwrap(), endpoint);
            drop(stream);
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                job.identify_connected(stream, &root.join("original-wrong-image.exe"), deadline())
                    .err()
                    .unwrap()
                    .code,
                "ENDPOINT_OWNER_MISMATCH"
            );
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                job.identify_connected(stream, std::path::Path::new("relative.exe"), deadline())
                    .err()
                    .unwrap()
                    .code,
                "INVALID_ARGUMENTS"
            );
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            let local = stream.local_addr().unwrap();
            let bound = job
                .bind_connected(stream, server.process(), deadline())
                .unwrap();
            assert_eq!(
                bound.owner().process_id(),
                server.process().identity().process_id()
            );
            let stream = bound.into_stream();
            assert_eq!(stream.local_addr().unwrap(), local);
            assert_eq!(stream.peer_addr().unwrap(), endpoint);
            drop(stream);
            let other = Job::create(deadline()).unwrap();
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                other
                    .identify_connected(stream, server.process().identity().image(), deadline())
                    .err()
                    .unwrap()
                    .code,
                "ENDPOINT_OWNER_MISMATCH"
            );
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                other
                    .bind_connected(stream, server.process(), deadline())
                    .err()
                    .unwrap()
                    .code,
                "ENDPOINT_OWNER_MISMATCH"
            );
            let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
            assert_eq!(
                job.bind_connected(stream, server.process(), Instant::now())
                    .err()
                    .unwrap()
                    .code,
                "DEADLINE"
            );
        }
        let foreign_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let stream = TcpStream::connect(foreign_listener.local_addr().unwrap()).unwrap();
        let (mut accepted, _) = foreign_listener.accept().unwrap();
        assert_eq!(
            job.identify_connected(stream, server.process().identity().image(), deadline())
                .err()
                .unwrap()
                .code,
            "ENDPOINT_OWNER_MISMATCH"
        );
        assert_eq!(accepted.read(&mut [0]).unwrap(), 0);
        let stream = TcpStream::connect(foreign_listener.local_addr().unwrap()).unwrap();
        let (mut accepted, _) = foreign_listener.accept().unwrap();
        assert_eq!(
            job.bind_connected(stream, server.process(), deadline())
                .err()
                .unwrap()
                .code,
            "ENDPOINT_OWNER_MISMATCH"
        );
        assert_eq!(accepted.read(&mut [0]).unwrap(), 0);
        drop(release);
        wait_file(&root.join("received.json"));
        let received: usize = crate::config::read_json(&root.join("received.json"), 4096).unwrap();
        assert_eq!(received, 0);
    }
}
