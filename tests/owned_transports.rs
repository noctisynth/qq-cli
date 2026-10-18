#![cfg(windows)]
use qq_cli::client::{Action, Backend};
use qq_cli::config::Config;
use qq_cli::native_job::{Job, SpawnSpec, Spawned};
use qq_cli::owned_http::OwnedOneBot;
use qq_cli::watch::Source;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
const MODE: &str = "QQ_CLI_TEST_OWNED_HTTP";
const ROOT: &str = "QQ_CLI_TEST_OWNED_HTTP_ROOT";
const TOKEN: &str = "original-http-token";
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn wait_file(path: &Path) {
    let until = deadline();
    while !path.exists() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(path.exists(), "original service handshake timed out");
}
fn service() -> bool {
    if std::env::var_os(MODE).is_none() {
        return false;
    }
    let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
    let listeners = [
        TcpListener::bind("127.0.0.1:0").unwrap(),
        TcpListener::bind("[::1]:0").unwrap(),
    ];
    for listener in &listeners {
        listener.set_nonblocking(true).unwrap();
    }
    let addresses: Vec<SocketAddr> = listeners.iter().map(|s| s.local_addr().unwrap()).collect();
    qq_cli::state::atomic_json(&root.join("endpoints.json"), &addresses).unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let empty = Arc::new(AtomicUsize::new(0));
    let until = Instant::now() + Duration::from_secs(25);
    while !root.join("release").exists() && Instant::now() < until {
        for listener in &listeners {
            loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let count = requests.clone();
                        let empty = empty.clone();
                        let root = root.clone();
                        std::thread::spawn(move || {
                            handle(stream, &root, &count, &empty);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => panic!("original listener failed"),
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    qq_cli::state::atomic_json(
        &root.join("counts.json"),
        &json!({"requests":requests.load(Ordering::SeqCst),"empty":empty.load(Ordering::SeqCst)}),
    )
    .unwrap();
    true
}
fn handle(mut stream: TcpStream, root: &Path, count: &AtomicUsize, empty: &AtomicUsize) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let header_end;
    loop {
        let mut block = [0; 1024];
        let n = match stream.read(&mut block) {
            Ok(n) => n,
            Err(_) if bytes.is_empty() => 0,
            Err(_) => return,
        };
        if n == 0 {
            empty.fetch_add(1, Ordering::SeqCst);
            qq_cli::state::atomic_json(&root.join("empty.json"), &true).unwrap();
            return;
        }
        bytes.extend_from_slice(&block[..n]);
        if let Some(index) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
        assert!(bytes.len() < 16 * 1024, "original request header limit");
    }
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let fields: Vec<_> = header
        .lines()
        .filter_map(|s| s.split_once(':'))
        .map(|(n, v)| (n.to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let websocket = header.starts_with("GET /event ");
    assert!(
        websocket
            || header.starts_with("POST /get_")
            || header.starts_with("POST /send_private_msg ")
    );
    assert!(fields.contains(&("authorization".into(), format!("Bearer {TOKEN}"))));
    assert!(
        !fields
            .iter()
            .any(|(n, _)| n == "cookie" || n == "proxy-authorization")
    );
    if websocket {
        assert!(
            fields
                .iter()
                .any(|(n, v)| n == "connection" && v.eq_ignore_ascii_case("upgrade"))
        );
        let key = &fields
            .iter()
            .find(|(n, _)| n == "sec-websocket-key")
            .unwrap()
            .1;
        let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
        count.fetch_add(1, Ordering::SeqCst);
        stream.write_all(format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n").as_bytes()).unwrap();
        let text = b"{\"original\":\"event\"}";
        stream.write_all(&[0x81, text.len() as u8]).unwrap();
        stream.write_all(text).unwrap();
        return;
    }
    assert!(fields.contains(&("connection".into(), "close".into())));
    assert!(fields.contains(&("accept-encoding".into(), "identity".into())));
    let size: usize = fields
        .iter()
        .find(|(n, _)| n == "content-length")
        .unwrap()
        .1
        .parse()
        .unwrap();
    assert!(size < 64 * 1024);
    while bytes.len() < header_end + size {
        let mut block = [0; 1024];
        let n = stream.read(&mut block).unwrap();
        assert_ne!(n, 0);
        bytes.extend_from_slice(&block[..n]);
    }
    let payload: Value = serde_json::from_slice(&bytes[header_end..header_end + size]).unwrap();
    count.fetch_add(1, Ordering::SeqCst);
    let mode = payload["mode"].as_str().unwrap_or("cli");
    let data = if mode == "cli" {
        match header
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
        {
            "/get_version_info" => {
                json!({"app_version":if root.join("changed-version").exists(){"4.18.39"}else{"4.18.38"}})
            }
            "/get_login_info" => {
                json!({"user_id":if root.join("changed-account").exists(){"10003"}else{"10001"}})
            }
            "/get_status" => json!({"online":true}),
            "/get_friend_list" => json!([{"user_id":"10002","nickname":"original contact"}]),
            _ => panic!("unexpected original CLI action"),
        }
    } else {
        json!({"id":9007199254740993u64,"text":"original synthetic"})
    };
    let success = json!({"status":"ok","retcode":0,"data":data}).to_string();
    let body=match mode {
        "media"=>json!({"status":"ok","retcode":0,"data":{"base64":"AAAA".repeat(512),"file_size":"1536","file":"ignored-original","url":"https://invalid.example/original"}}).to_string(),
        "rejected"=>json!({"status":"failed","retcode":1200,"message":"original-secret-marker","data":null}).to_string(),
        "async"=>json!({"status":"async","retcode":1,"data":null}).to_string(),
        "bad-json"=>"original-not-json".into(),
        _=>success,
    };
    if mode == "slow" {
        qq_cli::state::atomic_json(&root.join("slow.json"), &true).unwrap();
        std::thread::sleep(Duration::from_millis(1500));
    }
    let wire = match mode {
        "chunked" => {
            let mut s = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_owned();
            for bytes in body.as_bytes().chunks(17) {
                s.push_str(&format!(
                    "{:x}\r\n{}\r\n",
                    bytes.len(),
                    std::str::from_utf8(bytes).unwrap()
                ));
            }
            s.push_str("0\r\nX-Original: bounded-trailer\r\n\r\n");
            s
        }
        "oversized" => "HTTP/1.1 200 OK\r\nContent-Length: 1025\r\n\r\n".into(),
        "chunk-limit" => format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n401\r\n{}\r\n0\r\n\r\n",
            "x".repeat(1025)
        ),
        "media-limit" => format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            qq_cli::media::MAX_BASE64_BYTES + 1025
        ),
        "headers" => format!(
            "HTTP/1.1 200 OK\r\nX-Original: {}\r\nContent-Length: 2\r\n\r\n{}",
            "x".repeat(40 * 1024),
            "{}"
        ),
        "duplicate-length" => {
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\n{}".into()
        }
        "truncated" => "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{".into(),
        "auth" => {
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 22\r\n\r\noriginal-secret-marker".into()
        }
        "redirect" => format!(
            "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Length: 0\r\n\r\n",
            payload["redirect"].as_str().unwrap()
        ),
        "encoding" => format!(
            "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
        _ => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    };
    let _ = stream.write_all(wire.as_bytes());
}
struct Fixture {
    temp: tempfile::TempDir,
    job: Job,
    server: Spawned,
    endpoints: Vec<SocketAddr>,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let job = Job::create(deadline()).unwrap();
        let mut environment: BTreeMap<OsString, OsString> = BTreeMap::from([
            (MODE.into(), "1".into()),
            (ROOT.into(), root.as_os_str().to_owned()),
        ]);
        if let Some(system) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), system);
        }
        let executable = std::env::current_exe().unwrap();
        let args = ["--exact".into(), name.into(), "--test-threads=1".into()];
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
        let endpoints = qq_cli::config::read_json(&root.join("endpoints.json"), 4096).unwrap();
        Self {
            temp,
            job,
            server,
            endpoints,
        }
    }
    fn client(&self, index: usize) -> OwnedOneBot<'_> {
        OwnedOneBot::new(
            &self.job,
            self.server.process(),
            &format!("http://{}/", self.endpoints[index]),
            TOKEN,
            &Config {
                response_bytes: 1024,
                ..Config::default()
            },
        )
        .unwrap()
    }
    fn counts(&self) -> Value {
        std::fs::write(self.temp.path().join("release"), b"original release").unwrap();
        wait_file(&self.temp.path().join("counts.json"));
        qq_cli::config::read_json(&self.temp.path().join("counts.json"), 4096).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.temp.path().join("release"), b"original release");
    }
}

#[test]
fn websocket_upgrade_authentication_and_frames_use_the_verified_ipv4_and_ipv6_socket() {
    if service() {
        return;
    }
    let fixture = Fixture::new(
        "websocket_upgrade_authentication_and_frames_use_the_verified_ipv4_and_ipv6_socket",
    );
    for endpoint in &fixture.endpoints {
        let endpoint = format!("ws://{endpoint}/event");
        let mut connection = qq_cli::watch::SocketSource::owned(
            &endpoint,
            TOKEN,
            &fixture.job,
            fixture.server.process(),
        );
        connection.connect(deadline()).unwrap();
        match connection.receive().unwrap() {
            qq_cli::websocket::Frame::Text(bytes) => assert_eq!(
                serde_json::from_slice::<Value>(&bytes).unwrap(),
                json!({"original":"event"})
            ),
            _ => panic!("original text frame expected"),
        }
        // Replacement failure discards the prior socket; it cannot consume stale frames.
        assert_eq!(
            connection.connect(Instant::now()).unwrap_err().code,
            "DEADLINE"
        );
        assert!(connection.receive().is_err());
    }
    assert_eq!(fixture.counts()["requests"], 2);
}

#[test]
fn managed_cli_recovers_signed_service_and_withholds_reads_after_account_or_record_changes() {
    if service() {
        return;
    }
    use qq_cli::credentials::{Credentials, SystemCredentials};
    use qq_cli::start_intent::{self, Begin, Target};
    use qq_cli::state::{Installation, Instance, Store};
    struct Cleanup {
        credentials: SystemCredentials,
        key: String,
        root: PathBuf,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::write(self.root.join("release"), b"original release");
            let _ = self.credentials.delete(&self.key);
            let _ = self.credentials.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let store = Store::new(root.join("state"));
    let target = Target {
        instance_id: "original-instance".into(),
        run_id: "original-run".into(),
        installation_id: "original-installation".into(),
        installation_digest: "a".repeat(64),
        input_hash: "b".repeat(64),
    };
    let key =
        qq_cli::runtime_store::api_credential_key(&target.instance_id, &target.run_id).unwrap();
    let cleanup = Cleanup {
        credentials: SystemCredentials::new(store.root()).unwrap(),
        key: key.clone(),
        root: root.to_owned(),
    };
    let refs =
        qq_cli::references::References::open(&store, &cleanup.credentials, deadline()).unwrap();
    cleanup.credentials.set(&key, TOKEN).unwrap();
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations.push(Installation {
        id: target.installation_id.clone(),
        version: "v4.18.38".into(),
        digest: target.installation_digest.clone(),
        directory: root.join("unused-original-installation"),
    });
    tx.registry.instances.push(Instance {
        id: target.instance_id.clone(),
        run_id: target.run_id.clone(),
        installation_id: target.installation_id.clone(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: key,
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let mut environment = BTreeMap::from([
        (OsString::from(MODE), "1".into()),
        (OsString::from(ROOT), root.as_os_str().to_owned()),
    ]);
    if let Some(system) = std::env::var_os("SystemRoot") {
        environment.insert("SYSTEMROOT".into(), system);
    }
    let exe = std::env::current_exe().unwrap();
    let args = [
        "--exact".into(),
        "managed_cli_recovers_signed_service_and_withholds_reads_after_account_or_record_changes"
            .into(),
        "--test-threads=1".into(),
    ];
    let spec = SpawnSpec {
        program: &exe,
        arguments: &args,
        environment: &environment,
        directory: root,
    };
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "original-start", &target, deadline()).unwrap()
    else {
        panic!("fresh original intent")
    };
    let mut running = None;
    intent
        .dispatch(|tx| {
            running = Some(qq_cli::run_job::launch_once(
                tx,
                &refs,
                "original-start",
                &target,
                &spec,
                deadline(),
            )?);
            Ok(())
        })
        .unwrap();
    let running = running.unwrap();
    wait_file(&root.join("endpoints.json"));
    let endpoints: Vec<SocketAddr> =
        qq_cli::config::read_json(&root.join("endpoints.json"), 4096).unwrap();
    let unbound_config = root.join("unbound-config.json");
    std::fs::write(
        &unbound_config,
        serde_json::to_vec(&Config {
            state_dir: Some(store.root().to_owned()),
            ..Config::default()
        })
        .unwrap(),
    )
    .unwrap();
    let (code, unbound) = run_managed_cli(&unbound_config, &["status"]);
    assert_eq!(code, 0, "{unbound}");
    assert_eq!(unbound["data"]["state"], "starting");
    assert_eq!(unbound["data"]["identityVerified"], false);
    assert!(store.read().unwrap().instances[0].account.is_none());
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances[0].endpoint = format!("http://{}/", endpoints[0]);
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
    tx.registry.instances[0].state = "login_required".into();
    tx.commit().unwrap();
    drop(owner);
    drop(running); // CLI must recover independently of all parent Job/process handles.
    let config = root.join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&Config {
            state_dir: Some(store.root().to_owned()),
            ..Config::default()
        })
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("changed-version"),
        b"original mismatch before login",
    )
    .unwrap();
    let (code, mismatch) = run_managed_cli(&config, &["status"]);
    assert_eq!(code, 3, "{mismatch}");
    assert_eq!(mismatch["error"]["code"], "VERSION_MISMATCH");
    assert_eq!(store.read().unwrap().instances[0].state, "login_required");
    assert!(store.read().unwrap().instances[0].account.is_none());
    std::fs::remove_file(root.join("changed-version")).unwrap();
    let (code, status) = run_managed_cli(&config, &["status"]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(status["meta"]["scope"]["account"], "10001");
    assert_eq!(store.read().unwrap().instances[0].state, "ready");
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    let intent_count = std::fs::read_dir(store.root().join("start-requests"))
        .unwrap()
        .count();
    let (code, reused) = run_managed_cli(&config, &["napcat", "start"]);
    assert_eq!(code, 0, "{reused}");
    assert_eq!(reused["data"]["reused"], true);
    assert_eq!(reused["data"]["state"], "ready");
    assert_eq!(reused["meta"]["scope"]["account"], "10001");
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    let (code, conflict) = run_managed_cli(&config, &["napcat", "start", "--version", "v4.18.39"]);
    assert_eq!(code, 4, "{conflict}");
    assert_eq!(conflict["error"]["code"], "INSTANCE_VERSION_CONFLICT");
    let (code, new) = run_managed_cli(
        &config,
        &[
            "napcat",
            "start",
            "--new",
            "--request-id",
            "original-new-run",
        ],
    );
    assert_eq!(code, 4, "{new}");
    assert_eq!(new["error"]["code"], "LICENSE_UNCONFIRMED");
    assert_eq!(
        std::fs::read_dir(store.root().join("start-requests"))
            .unwrap()
            .count(),
        intent_count
    );
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    let (code, contacts) = run_managed_cli(&config, &["contact", "list"]);
    assert_eq!(code, 0, "{contacts}");
    assert_eq!(contacts["meta"]["scope"]["account"], "10001");
    assert!(contacts.to_string().contains("original contact"));
    std::fs::write(root.join("changed-account"), b"original change").unwrap();
    std::fs::write(root.join("changed-version"), b"original mismatch").unwrap();
    let (code, mismatch) = run_managed_cli(&config, &["status"]);
    assert_eq!(code, 3, "{mismatch}");
    assert_eq!(mismatch["error"]["code"], "VERSION_MISMATCH");
    assert_eq!(
        store.read().unwrap().instances[0].account.as_deref(),
        Some("10001")
    );
    std::fs::remove_file(root.join("changed-version")).unwrap();
    let (code, changed_start) = run_managed_cli(&config, &["napcat", "start"]);
    assert_eq!(code, 4, "{changed_start}");
    assert_eq!(changed_start["error"]["code"], "ACCOUNT_CHANGED");
    assert_eq!(
        store.read().unwrap().instances[0].account.as_deref(),
        Some("10001")
    );
    let (code, changed) = run_managed_cli(&config, &["contact", "list"]);
    assert_eq!(code, 4, "{changed}");
    assert_eq!(changed["error"]["code"], "ACCOUNT_CHANGED");
    assert!(!changed.to_string().contains("original contact"));
    let journal = std::fs::read_dir(store.root().join("run-jobs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = std::fs::read(&journal).unwrap();
    std::fs::write(&journal, b"original corrupt pointer").unwrap();
    let (code, error) = run_managed_cli(&config, &["status"]);
    assert_ne!(code, 0, "{error}");
    assert_eq!(
        std::fs::read(&journal).unwrap(),
        b"original corrupt pointer"
    );
    std::fs::write(&journal, &original).unwrap();
    let saved = store.root().join("original-saved-run.json");
    assert!(journal.starts_with(root) && saved.starts_with(root));
    std::fs::rename(&journal, &saved).unwrap();
    let (code, error) = run_managed_cli(&config, &["status"]);
    assert_eq!(code, 4, "{error}");
    assert_eq!(error["error"]["code"], "RUNTIME_RECORD_REQUIRED");
    assert!(!journal.exists());
    std::fs::rename(&saved, &journal).unwrap();
    std::fs::write(root.join("release"), b"original release").unwrap();
    wait_file(&root.join("counts.json"));
    let counts: Value = qq_cli::config::read_json(&root.join("counts.json"), 4096).unwrap();
    assert_eq!(counts["requests"], 25);
    assert_eq!(counts["empty"], 1);
}

fn run_managed_cli(config: &Path, args: &[&str]) -> (i32, Value) {
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .env_remove("QQ_CLI_CONFIG")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = deadline();
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("original CLI deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains(TOKEN));
    (
        output.status.code().unwrap(),
        serde_json::from_str(text).unwrap(),
    )
}

#[test]
fn websocket_wrong_job_wrong_service_and_expiry_send_no_upgrade_or_credentials() {
    if service() {
        return;
    }
    let fixture =
        Fixture::new("websocket_wrong_job_wrong_service_and_expiry_send_no_upgrade_or_credentials");
    let other = Job::create(deadline()).unwrap();
    let endpoint = format!("ws://{}/event", fixture.endpoints[0]);
    assert_eq!(
        qq_cli::websocket::Connection::connect_owned(
            &endpoint,
            TOKEN,
            &other,
            fixture.server.process(),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "ENDPOINT_OWNER_MISMATCH"
    );
    assert_eq!(
        qq_cli::websocket::Connection::connect_owned(
            &endpoint,
            TOKEN,
            &fixture.job,
            fixture.server.process(),
            Instant::now()
        )
        .err()
        .unwrap()
        .code,
        "DEADLINE"
    );
    assert_eq!(
        qq_cli::websocket::Connection::connect_owned(
            &endpoint,
            "original\r\ninvalid",
            &fixture.job,
            fixture.server.process(),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "CREDENTIAL_INVALID"
    );
    let foreign = TcpListener::bind("127.0.0.1:0").unwrap();
    assert_eq!(
        qq_cli::websocket::Connection::connect_owned(
            &format!("ws://{}/event", foreign.local_addr().unwrap()),
            TOKEN,
            &fixture.job,
            fixture.server.process(),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "ENDPOINT_OWNER_MISMATCH"
    );
    let (mut accepted, _) = foreign.accept().unwrap();
    assert_eq!(accepted.read(&mut [0]).unwrap(), 0);
    wait_file(&fixture.temp.path().join("empty.json"));
    let counts = fixture.counts();
    assert_eq!(counts["requests"], 0);
    assert_eq!(counts["empty"], 1);
}
#[test]
fn authenticated_fixed_chunked_and_media_requests_use_the_verified_ipv4_and_ipv6_connection() {
    if service() {
        return;
    }
    let fixture = Fixture::new(
        "authenticated_fixed_chunked_and_media_requests_use_the_verified_ipv4_and_ipv6_connection",
    );
    for index in 0..2 {
        let client = fixture.client(index);
        for mode in ["fixed", "chunked"] {
            let value = client
                .call(Action::Status, &json!({"mode":mode}), deadline())
                .unwrap();
            assert_eq!(value["id"].as_u64(), Some(9007199254740993));
        }
        assert_eq!(
            client
                .call(Action::File, &json!({"mode":"media"}), deadline())
                .unwrap_err()
                .code,
            "RESPONSE_LIMIT"
        );
        let media = client
            .media_bytes(Action::File, &json!({"mode":"media"}), deadline())
            .unwrap();
        assert_eq!(media.encoded.len(), 2048);
        assert_eq!(media.size, Some(1536));
        assert_eq!(
            client
                .media_bytes(Action::Status, &json!({}), deadline())
                .err()
                .unwrap()
                .code,
            "INVALID_ARGUMENTS"
        );
    }
    assert_eq!(fixture.counts()["requests"], 8);
}
#[test]
fn malformed_status_framing_limits_and_timeout_are_bounded_without_retries_or_raw_errors() {
    if service() {
        return;
    }
    let fixture = Fixture::new(
        "malformed_status_framing_limits_and_timeout_are_bounded_without_retries_or_raw_errors",
    );
    let client = fixture.client(0);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert_eq!(
            OwnedOneBot::new(
                &fixture.job,
                fixture.server.process(),
                &format!("http://{}/", fixture.endpoints[0]),
                TOKEN,
                &Config::default()
            )
            .err()
            .unwrap()
            .code,
            "HTTP_RUNTIME_CONTEXT"
        );
        assert_eq!(
            client
                .call(Action::Status, &json!({}), deadline())
                .unwrap_err()
                .code,
            "HTTP_RUNTIME_CONTEXT"
        );
    });
    let trap = TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let cases = [
        ("auth", "BACKEND_AUTH"),
        ("redirect", "BACKEND_HTTP"),
        ("rejected", "BACKEND_REJECTED"),
        ("async", "BACKEND_ASYNC"),
        ("bad-json", "BACKEND_PROTOCOL"),
        ("oversized", "RESPONSE_LIMIT"),
        ("chunk-limit", "RESPONSE_LIMIT"),
        ("encoding", "BACKEND_PROTOCOL"),
        ("headers", "BACKEND_TRANSPORT"),
        ("duplicate-length", "BACKEND_TRANSPORT"),
        ("truncated", "BACKEND_TRANSPORT"),
    ];
    for (mode, code) in cases {
        let error = client
            .call(
                Action::Status,
                &json!({"mode":mode,"redirect":format!("http://{}/",trap.local_addr().unwrap())}),
                deadline(),
            )
            .unwrap_err();
        assert_eq!(error.code, code, "original response case {mode}");
        let output = serde_json::to_string(&error).unwrap();
        assert!(!output.contains(TOKEN) && !output.contains("original-secret-marker"));
        assert!(!error.retry_safe);
    }
    assert_eq!(
        client
            .media_bytes(Action::File, &json!({"mode":"media-limit"}), deadline())
            .err()
            .unwrap()
            .code,
        "RESPONSE_LIMIT"
    );
    assert_eq!(
        client
            .call(
                Action::SendPrivate,
                &json!({"mode":"truncated","message":"original synthetic"}),
                deadline()
            )
            .unwrap_err()
            .code,
        "BACKEND_TRANSPORT"
    );
    let started = Instant::now();
    assert_eq!(
        client
            .call(
                Action::Status,
                &json!({"mode":"slow"}),
                started + Duration::from_millis(500)
            )
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    wait_file(&fixture.temp.path().join("slow.json"));
    assert_eq!(
        trap.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(fixture.counts()["requests"], 14);
}
#[test]
fn wrong_owner_expiry_and_invalid_inputs_do_not_send_authentication_or_create_extra_requests() {
    if service() {
        return;
    }
    let fixture = Fixture::new(
        "wrong_owner_expiry_and_invalid_inputs_do_not_send_authentication_or_create_extra_requests",
    );
    let other = Job::create(deadline()).unwrap();
    let endpoint = format!("http://{}/", fixture.endpoints[0]);
    let wrong = OwnedOneBot::new(
        &other,
        fixture.server.process(),
        &endpoint,
        TOKEN,
        &Config::default(),
    )
    .unwrap();
    assert_eq!(
        wrong
            .call(Action::Status, &json!({}), deadline())
            .unwrap_err()
            .code,
        "ENDPOINT_OWNER_MISMATCH"
    );
    let client = fixture.client(0);
    assert_eq!(
        client
            .call(Action::Status, &json!({}), Instant::now())
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    assert_eq!(
        client
            .call(
                Action::Status,
                &json!({"data":"x".repeat(4*1024*1024)}),
                deadline()
            )
            .unwrap_err()
            .code,
        "REQUEST_LIMIT"
    );
    assert_eq!(
        OwnedOneBot::new(
            &fixture.job,
            fixture.server.process(),
            &endpoint,
            "original\r\ninvalid",
            &Config::default()
        )
        .err()
        .unwrap()
        .code,
        "CREDENTIAL_INVALID"
    );
    let foreign = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = OwnedOneBot::new(
        &fixture.job,
        fixture.server.process(),
        &format!("http://{}/", foreign.local_addr().unwrap()),
        TOKEN,
        &Config::default(),
    )
    .unwrap();
    assert_eq!(
        client
            .call(Action::Status, &json!({}), deadline())
            .unwrap_err()
            .code,
        "ENDPOINT_OWNER_MISMATCH"
    );
    let (mut accepted, _) = foreign.accept().unwrap();
    assert_eq!(accepted.read(&mut [0]).unwrap(), 0);
    wait_file(&fixture.temp.path().join("empty.json"));
    let counts = fixture.counts();
    assert_eq!(counts["requests"], 0);
    assert_eq!(counts["empty"], 1);
}
