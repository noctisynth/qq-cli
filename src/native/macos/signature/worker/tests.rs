use super::*;

#[test]
fn worker_budget_preserves_interactive_time_without_extending_static_checks() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(900);
    let interactive = worker_budget(deadline, Duration::from_secs(1800)).unwrap();
    assert!(interactive > Duration::from_secs(300));
    assert!(interactive <= Duration::from_secs(900));
    assert_eq!(
        worker_budget(deadline, Duration::from_secs(300)).unwrap(),
        Duration::from_secs(300)
    );
    let short = worker_budget(now + Duration::from_secs(5), Duration::from_secs(1800)).unwrap();
    assert!(short <= Duration::from_secs(5));
    assert!(worker_budget(now, Duration::from_secs(1800)).is_err());
}

const HELPER: &str = "native::macos_signature::worker::tests::worker_fixture";
fn child(mode: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", HELPER, "--nocapture"])
        .env("QQ_CLI_SIGNATURE_TEST", mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}
#[test]
fn worker_fixture() {
    let Ok(mode) = std::env::var("QQ_CLI_SIGNATURE_TEST") else {
        return;
    };
    match mode.as_str() {
        "store" => std::process::exit(i32::from(STORE)),
        "developer" => std::process::exit(i32::from(DEVELOPER)),
        "failure" => std::process::exit(i32::from(FAILURE)),
        "unknown" => std::process::exit(0),
        "watch-parent" | "watch-timeout" => {
            watchdogs(if mode == "watch-timeout" {
                Duration::from_millis(30)
            } else {
                Duration::from_secs(30)
            })
            .unwrap();
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        "hang" => loop {
            std::thread::sleep(Duration::from_secs(1));
        },
        _ => std::process::exit(3),
    }
}
#[test]
fn only_exact_worker_result_codes_are_accepted() {
    for (mode, expected) in [
        ("store", Origin::AppleAppStore),
        ("developer", Origin::TencentDeveloperId),
    ] {
        assert_eq!(
            wait(child(mode), Instant::now() + Duration::from_secs(5)).unwrap(),
            expected
        );
    }
    for mode in ["failure", "unknown"] {
        assert_eq!(
            wait(child(mode), Instant::now() + Duration::from_secs(5))
                .unwrap_err()
                .code,
            "QQ_SIGNATURE_UNVERIFIED"
        );
    }
}
#[test]
fn timeout_reaps_only_the_original_worker_and_does_not_wait_for_its_work() {
    let child = child("hang");
    let pid = child.id();
    let observed =
        crate::native::process::Pinned::observe(pid, Instant::now() + Duration::from_secs(5))
            .unwrap();
    let started = Instant::now();
    assert_eq!(
        wait(child, started + Duration::from_millis(50))
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        observed
            .verify(Instant::now() + Duration::from_secs(1))
            .is_err()
    );
}
#[test]
fn a_success_code_after_the_deadline_is_not_accepted() {
    assert_eq!(
        wait(child("store"), Instant::now()).unwrap_err().code,
        "DEADLINE"
    );
}

#[test]
fn worker_watchdogs_end_blocked_work_on_timeout_or_parent_loss() {
    let started = Instant::now();
    assert_eq!(
        wait(
            child("watch-timeout"),
            Instant::now() + Duration::from_secs(5)
        )
        .unwrap_err()
        .code,
        "DEADLINE"
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    let mut worker = child("watch-parent");
    worker.stdin.take();
    assert_eq!(
        wait(worker, Instant::now() + Duration::from_secs(5))
            .unwrap_err()
            .code,
        "QQ_SIGNATURE_UNVERIFIED"
    );
}

#[test]
fn guard_loss_reaps_checker_without_accepting_later_results() {
    let child = child("hang");
    let deadline = Instant::now() + Duration::from_secs(5);
    let observed = crate::native::process::Pinned::observe(child.id(), deadline).unwrap();
    let result = wait_code(child, deadline, &|| {
        Err(AppError::new("GUARD_LOST", "Synthetic guard loss", 3))
    });
    assert_eq!(result.unwrap_err().code, "GUARD_LOST");
    assert!(observed.verify(deadline).is_err());
}
