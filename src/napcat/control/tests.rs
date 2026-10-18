use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::time::Duration;

struct Fake {
    replies: RefCell<VecDeque<Result<Vec<u8>>>>,
    calls: Cell<usize>,
    expire_after_auth: bool,
}
impl Fake {
    fn new(replies: Vec<Value>) -> Self {
        Self {
            replies: RefCell::new(
                replies
                    .into_iter()
                    .map(|v| Ok(serde_json::to_vec(&v).unwrap()))
                    .collect(),
            ),
            calls: Cell::new(0),
            expire_after_auth: false,
        }
    }
}
impl Backend for Fake {
    fn post(
        &self,
        route: Route,
        payload: &Value,
        credential: Option<&str>,
        deadline: Instant,
    ) -> Result<Vec<u8>> {
        let count = self.calls.get();
        self.calls.set(count + 1);
        match count {
            0 => {
                assert_eq!(route, Route::Authenticate);
                assert_eq!(route.path(), "/api/auth/login");
                assert_eq!(
                    payload,
                    &json!({"hash":"56038ae4b96c7fe3c58a9f6a145c474ae717c780c9c73eb92070b899f6d41aed"})
                );
                assert!(credential.is_none());
                if self.expire_after_auth {
                    std::thread::sleep(
                        deadline.saturating_duration_since(Instant::now())
                            + Duration::from_millis(1),
                    );
                }
            }
            1 => {
                assert_eq!(route, Route::LoginStatus);
                assert_eq!(route.path(), "/api/QQLogin/CheckLoginStatus");
                assert_eq!(payload, &json!({}));
                assert_eq!(credential, Some("original-server-credential"));
            }
            _ => panic!("observation must never retry"),
        }
        self.replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected request")
    }
}
fn auth() -> Value {
    json!({"code":0,"data":{"Credential":"original-server-credential"}})
}
fn status(phase: &str) -> Value {
    json!({"code":0,"data":{"loginPhase":phase,"isLogin":false,"isOffline":false,"coreReady":false,"qrLoginAccepted":false,
        "qrcodeurl":"private-challenge","loginError":"untrusted-private-error","account":"10001"}})
}
fn until() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn run(fake: &Fake) -> Result<Observation> {
    observe(fake, &"1".repeat(64), until())
}

#[test]
fn fixed_hash_and_routes_observe_only_allowlisted_fields_without_identity() {
    for (raw, phase) in [
        ("waiting_qrcode", Phase::WaitingQrcode),
        ("generating_qrcode", Phase::GeneratingQrcode),
        ("qrcode_scanned", Phase::QrcodeScanned),
        ("initializing", Phase::Initializing),
        ("ready", Phase::Ready),
        ("offline", Phase::Offline),
        ("reconnecting", Phase::Reconnecting),
        ("private-instruction-unknown", Phase::Unknown),
    ] {
        let fake = Fake::new(vec![auth(), status(raw)]);
        let observation = run(&fake).unwrap();
        assert_eq!(observation.phase, phase);
        // Phase alone cannot upgrade online state or identify an account.
        assert!(!observation.core_ready);
        let output = serde_json::to_string(&observation).unwrap();
        for private in [
            "private",
            "account",
            "10001",
            "Credential",
            "qrcodeurl",
            "loginError",
        ] {
            assert!(!output.contains(private));
        }
        assert_eq!(fake.calls.get(), 2);
    }
}

#[test]
fn management_rejection_and_second_factor_do_not_attempt_status_or_retry() {
    for (reply, code) in [
        (
            json!({"code":-1,"message":"private-token"}),
            "CONTROL_REJECTED",
        ),
        (
            json!({"code":0,"data":{"require2FA":true,"Credential":"must-not-use"}}),
            "CONTROL_2FA_REQUIRED",
        ),
        (json!({"code":0,"data":{}}), "CONTROL_PROTOCOL"),
        (
            json!({"code":0,"data":{"Credential":"bad\r\nheader"}}),
            "CONTROL_PROTOCOL",
        ),
        (
            json!({"code":0,"data":{"Credential":""}}),
            "CONTROL_PROTOCOL",
        ),
    ] {
        let fake = Fake::new(vec![reply]);
        let error = run(&fake).unwrap_err();
        assert_eq!(error.code, code);
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("private-token")
        );
        assert_eq!(fake.calls.get(), 1);
    }
}

#[test]
fn malformed_status_and_conflicting_boolean_hints_are_not_silently_coerced() {
    let mut cases = vec![
        json!({"code":0,"data":{}}),
        json!({"code":-1,"message":"private-response"}),
    ];
    for (field, value) in [
        ("isLogin", json!("true")),
        ("coreReady", json!(true)),
        ("isOffline", Value::Null),
    ] {
        let mut reply = status("ready");
        reply["data"][field] = value;
        cases.push(reply);
    }
    let mut conflict = status("ready");
    for field in ["isLogin", "isOffline", "coreReady"] {
        conflict["data"][field] = json!(true);
    }
    cases.push(conflict);
    for reply in cases {
        let fake = Fake::new(vec![auth(), reply]);
        let error = run(&fake).unwrap_err();
        assert!(matches!(
            error.code,
            "CONTROL_PROTOCOL" | "CONTROL_REJECTED"
        ));
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("private-response")
        );
        assert_eq!(fake.calls.get(), 2);
    }
    let mut ready = status("initializing");
    for field in ["isLogin", "coreReady", "qrLoginAccepted"] {
        ready["data"][field] = json!(true);
    }
    assert!(run(&Fake::new(vec![auth(), ready])).unwrap().core_ready);
}

#[test]
fn budgets_duplicates_bad_json_transport_failures_and_deadlines_never_replay() {
    for bytes in [
        vec![b'x'; RESPONSE_LIMIT + 1],
        b"not json".to_vec(),
        b"{\"code\":0,\"code\":0,\"data\":{}}".to_vec(),
    ] {
        let fake = Fake::new(vec![]);
        fake.replies.borrow_mut().push_back(Ok(bytes));
        assert!(run(&fake).is_err());
        assert_eq!(fake.calls.get(), 1);
    }
    let fake = Fake::new(vec![]);
    fake.replies.borrow_mut().push_back(Err(AppError::new(
        "BACKEND_TRANSPORT",
        "Original transport refusal.",
        3,
    )));
    assert_eq!(run(&fake).unwrap_err().code, "BACKEND_TRANSPORT");
    assert_eq!(fake.calls.get(), 1);
    let fake = Fake::new(vec![]);
    for token in ["", "bad token", "bad\nheader"] {
        assert_eq!(
            observe(&fake, token, until()).unwrap_err().code,
            "CONTROL_CREDENTIAL_INVALID"
        );
    }
    assert_eq!(
        observe(&fake, &"1".repeat(64), Instant::now())
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    assert_eq!(fake.calls.get(), 0);
    let mut fake = Fake::new(vec![auth()]);
    fake.expire_after_auth = true;
    assert_eq!(
        observe(
            &fake,
            &"1".repeat(64),
            Instant::now() + Duration::from_millis(5)
        )
        .unwrap_err()
        .code,
        "DEADLINE"
    );
    assert_eq!(fake.calls.get(), 1);
}
