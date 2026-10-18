//! Original test-only TCP relay. No production CLI flag, loader, QQ or external service.
use qq_cli::credentials::Credentials;
use qq_cli::native_job::SpawnSpec;
use qq_cli::process::Pinned;
use qq_cli::references::References;
use qq_cli::start_intent::{self, Begin, Target};
use qq_cli::state::Store;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};
const MODE: &str = "QQ_CLI_TEST_MANAGED_RELAY";
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
pub fn child() -> bool {
    let Some(root) = std::env::var_os(MODE) else {
        return false;
    };
    let root = PathBuf::from(root);
    let upstreams: Vec<SocketAddr> =
        qq_cli::config::read_json(&root.join("upstreams.json"), 4096).unwrap();
    assert!(upstreams.len() <= 2 && upstreams.iter().all(|a| a.ip().is_loopback()));
    let listeners: Vec<_> = upstreams
        .iter()
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    for listener in &listeners {
        listener.set_nonblocking(true).unwrap();
    }
    let endpoints: Vec<_> = listeners.iter().map(|l| l.local_addr().unwrap()).collect();
    qq_cli::state::atomic_json(&root.join("endpoints.json"), &endpoints).unwrap();
    let until = Instant::now() + Duration::from_secs(40);
    let mut tasks = Vec::new();
    while !root.join("release").exists() && Instant::now() < until {
        for (listener, upstream) in listeners.iter().zip(&upstreams) {
            while let Ok((mut incoming, _)) = listener.accept() {
                incoming.set_nonblocking(false).unwrap();
                incoming
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                incoming
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let upstream = *upstream;
                tasks.push(std::thread::spawn(move || {
                    // Ownership-only probes close without application data: never forward them.
                    if !matches!(incoming.peek(&mut [0]),Ok(n) if n>0) {
                        return;
                    }
                    let Ok(mut outgoing) =
                        TcpStream::connect_timeout(&upstream, Duration::from_secs(2))
                    else {
                        return;
                    };
                    outgoing
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    outgoing
                        .set_write_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut source = incoming.try_clone().unwrap();
                    let mut destination = outgoing.try_clone().unwrap();
                    let copy = std::thread::spawn(move || {
                        let _ = std::io::copy(&mut source, &mut destination);
                        let _ = destination.shutdown(std::net::Shutdown::Write);
                    });
                    let _ = std::io::copy(&mut outgoing, &mut incoming);
                    let _ = incoming.shutdown(std::net::Shutdown::Both);
                    let _ = outgoing.shutdown(std::net::Shutdown::Both);
                    let _ = copy.join();
                }));
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    for task in tasks {
        task.join().unwrap();
    }
    true
}
pub struct Managed {
    root: tempfile::TempDir,
    process: Pinned,
}
impl Drop for Managed {
    fn drop(&mut self) {
        let _ = std::fs::write(self.root.path().join("release"), b"original relay release");
        let until = deadline();
        while self.process.verify(until).is_ok() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Managed {
    pub fn attach(
        store: &Store,
        credentials: &impl Credentials,
        instance_id: &str,
        test_name: &str,
    ) -> Self {
        let refs = References::open(store, credentials, deadline()).unwrap();
        let mut tx = store.transaction(deadline()).unwrap();
        let instance = tx
            .registry
            .instances
            .iter()
            .find(|i| i.id == instance_id)
            .unwrap()
            .clone();
        let installation = tx
            .registry
            .installations
            .iter()
            .find(|i| i.id == instance.installation_id)
            .unwrap();
        let target = Target {
            instance_id: instance.id.clone(),
            run_id: instance.run_id.clone(),
            installation_id: installation.id.clone(),
            installation_digest: installation.digest.clone(),
            input_hash: "b".repeat(64),
        };
        let mut upstreams = vec![
            instance
                .endpoint
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .parse::<SocketAddr>()
                .unwrap(),
        ];
        if let Some(ws) = &instance.ws_endpoint {
            upstreams.push(qq_cli::websocket::validate_endpoint(ws).unwrap().1);
        }
        let current = tx
            .registry
            .instances
            .iter_mut()
            .find(|i| i.id == instance_id)
            .unwrap();
        current.state = "prepared".into();
        current.account = None;
        tx.commit().unwrap();
        let root = tempfile::tempdir().unwrap();
        qq_cli::state::atomic_json(&root.path().join("upstreams.json"), &upstreams).unwrap();
        let mut environment =
            BTreeMap::from([(OsString::from(MODE), root.path().as_os_str().to_owned())]);
        if let Some(system) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), system);
        }
        let exe = std::env::current_exe().unwrap();
        let args = [
            "--exact".into(),
            test_name.into(),
            "--test-threads=1".into(),
        ];
        let spec = SpawnSpec {
            program: &exe,
            arguments: &args,
            environment: &environment,
            directory: root.path(),
        };
        let Begin::Fresh(intent) =
            start_intent::begin(store, &refs, "original-relay-start", &target, deadline()).unwrap()
        else {
            panic!("fresh relay")
        };
        let mut running = None;
        intent
            .dispatch(|tx| {
                running = Some(qq_cli::run_job::launch_once(
                    tx,
                    &refs,
                    "original-relay-start",
                    &target,
                    &spec,
                    deadline(),
                )?);
                Ok(())
            })
            .unwrap();
        let running = running.unwrap();
        let until = deadline();
        while !root.path().join("endpoints.json").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        let endpoints: Vec<SocketAddr> =
            qq_cli::config::read_json(&root.path().join("endpoints.json"), 4096).unwrap();
        let mut tx = store.transaction(deadline()).unwrap();
        let current = tx
            .registry
            .instances
            .iter_mut()
            .find(|i| i.id == instance_id)
            .unwrap();
        current.endpoint = format!("http://{}/", endpoints[0]);
        if endpoints.len() == 2 {
            current.ws_endpoint = Some(format!("ws://{}/event", endpoints[1]));
        }
        tx.persist().unwrap();
        let stream = TcpStream::connect_timeout(&endpoints[0], Duration::from_secs(2)).unwrap();
        let (bound, owner) = running
            .job()
            .identify_connected(stream, running.root().identity().image(), deadline())
            .unwrap();
        qq_cli::run_job::bind_service(
            &tx,
            &refs,
            &target,
            running.job(),
            bound,
            &owner,
            deadline(),
        )
        .unwrap();
        let current = tx
            .registry
            .instances
            .iter_mut()
            .find(|i| i.id == instance_id)
            .unwrap();
        current.state = instance.state;
        current.account = instance.account;
        tx.commit().unwrap();
        let process = Pinned::reopen(running.root().identity(), deadline()).unwrap();
        drop(owner);
        drop(running);
        Self { root, process }
    }
}
