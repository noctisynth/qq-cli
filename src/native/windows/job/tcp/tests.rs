use super::*;
use crate::native::job::SpawnSpec;
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
fn binding_uses_owned_server_pid_on_the_same_socket_and_rejects_foreign_connections_without_data() {
    const MODE: &str = "QQ_CLI_TEST_TCP_CHILD";
    const ROOT: &str = "QQ_CLI_TEST_TCP_ROOT";
    const NAME: &str = "native::job::tcp::tests::binding_uses_owned_server_pid_on_the_same_socket_and_rejects_foreign_connections_without_data";
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
        crate::storage::state::atomic_json(&root.join("endpoints.json"), &addresses).unwrap();
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
        crate::storage::state::atomic_json(&root.join("received.json"), &bytes).unwrap();
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
        crate::storage::config::read_json(&root.join("endpoints.json"), 4096).unwrap();
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
    let received: usize =
        crate::storage::config::read_json(&root.join("received.json"), 4096).unwrap();
    assert_eq!(received, 0);
}
