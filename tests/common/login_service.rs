//! Original bounded management/OneBot service, running inside the test's owned Job.
use qq_cli::storage::{config::read_json, state::atomic_json};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{Duration, Instant},
};
pub const MODE: &str = "QQ_CLI_TEST_LOGIN_SERVICE";

pub fn child() -> bool {
    let Some(root) = std::env::var_os(MODE) else {
        return false;
    };
    let root = std::path::PathBuf::from(root);
    let spec: Value = read_json(&root.join("service.json"), 16384).unwrap();
    let control = TcpListener::bind(spec["control"].as_str().unwrap()).unwrap();
    control.set_nonblocking(true).unwrap();
    let mut api = None;
    let mut counts = json!({"auth":0,"control":0,"api":0,"empty":0});
    std::fs::write(root.join("service-ready"), b"original ready").unwrap();
    let end = Instant::now() + Duration::from_secs(60);
    while !root.join("release").exists() && Instant::now() < end {
        if root.join("api-stop").exists() && api.take().is_some() {
            std::fs::write(root.join("api-stopped"), b"original API stopped").unwrap();
        }
        if api.is_none() && root.join("api-ready").exists() && !root.join("api-stop").exists() {
            let listener = TcpListener::bind(spec["api"].as_str().unwrap()).unwrap();
            listener.set_nonblocking(true).unwrap();
            api = Some(listener);
            std::fs::write(root.join("api-listening"), b"original API ready").unwrap();
        }
        for (listener, is_control) in
            std::iter::once((&control, true)).chain(api.iter().map(|a| (a, false)))
        {
            while let Ok((stream, _)) = listener.accept() {
                handle(stream, &root, &spec, &mut counts, is_control);
                atomic_json(&root.join("counts.json"), &counts).unwrap();
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    atomic_json(&root.join("counts.json"), &counts).unwrap();
    true
}
fn handle(mut stream: TcpStream, root: &Path, spec: &Value, counts: &mut Value, control: bool) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut byte = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(0) => {
                counts["empty"] = json!(counts["empty"].as_u64().unwrap() + 1);
                return;
            }
            Ok(_) => bytes.push(byte[0]),
            Err(_) => return,
        }
        assert!(bytes.len() < 16384);
    }
    let header = String::from_utf8(bytes).unwrap();
    let path = header
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    let fields: Vec<_> = header
        .lines()
        .filter_map(|s| s.split_once(':'))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim()))
        .collect();
    assert!(
        !fields
            .iter()
            .any(|(k, _)| k == "cookie" || k == "proxy-authorization")
    );
    let length: usize = fields
        .iter()
        .find(|(k, _)| k == "content-length")
        .unwrap()
        .1
        .parse()
        .unwrap();
    assert!(length < 4096);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let mode = std::fs::read_to_string(root.join("mode")).unwrap_or_default();
    let reply = if control {
        match path {
            "/api/auth/login" => {
                assert!(!fields.iter().any(|(k, _)| k == "authorization"));
                assert_eq!(payload, json!({"hash":spec["hash"]}));
                counts["auth"] = json!(counts["auth"].as_u64().unwrap() + 1);
                if mode == "2fa" {
                    json!({"code":0,"data":{"require2FA":true}})
                } else if mode == "reject" {
                    json!({"code":-1,"message":"private-login-error"})
                } else {
                    json!({"code":0,"data":{"Credential":"original-control-credential"}})
                }
            }
            "/api/QQLogin/CheckLoginStatus" => {
                assert!(
                    fields
                        .contains(&("authorization".into(), "Bearer original-control-credential"))
                );
                assert_eq!(payload, json!({}));
                counts["control"] = json!(counts["control"].as_u64().unwrap() + 1);
                if mode == "race" {
                    let store = qq_cli::storage::state::Store::new(root.join("state"));
                    let mut tx = store
                        .transaction(Instant::now() + Duration::from_secs(1))
                        .expect("status must not hold state lock across HTTP");
                    tx.registry.instances[0].run_id = "changed-original-run".into();
                    tx.commit().unwrap();
                }
                json!({"code":0,"data":{"loginPhase":if mode=="control-ready" {"ready"}else if mode=="api-pending" {"initializing"}else{"qrcode_scanned"},"isLogin":mode=="control-ready","coreReady":mode=="control-ready","isOffline":false,"qrLoginAccepted":mode!="control-ready","qrcodeurl":"private-qrcode","loginError":"private-login-error","account":"10009"}})
            }
            _ => panic!("unexpected management route"),
        }
    } else {
        assert!(fields.contains(&(
            "authorization".into(),
            format!("Bearer {}", spec["apiToken"].as_str().unwrap()).as_str()
        )));
        counts["api"] = json!(counts["api"].as_u64().unwrap() + 1);
        let data = match path {
            "/get_version_info" => {
                json!({"app_version":if mode.starts_with("bad-version") {"4.18.39"}else{"4.18.38"}})
            }
            "/get_login_info" => {
                json!({"user_id":if mode=="api-pending" || mode=="bad-version-pending" {"0"}else if mode=="api-bad-identity" {"private-invalid-id"}else{"10001"},"nickname":"original"})
            }
            "/get_status" => json!({"online":true}),
            _ => panic!("unexpected account API route"),
        };
        json!({"status":"ok","retcode":0,"data":data})
    };
    let body = reply.to_string();
    // Publish counts before the response so parent assertions do not race the server.
    atomic_json(&root.join("counts.json"), counts).unwrap();
    if !control && mode == "api-malformed" {
        let _ = stream.write_all(b"original invalid HTTP\r\n\r\n");
        return;
    }
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .as_bytes(),
    );
}
