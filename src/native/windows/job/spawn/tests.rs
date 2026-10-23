use super::*;
use std::time::Duration;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
#[test]
fn command_and_environment_encoding_obey_boundaries_without_inheriting_defaults() {
    assert_eq!(
        String::from_utf16(
            &command(
                OsStr::new("original.exe"),
                &[
                    "".into(),
                    "space value".into(),
                    "quote\"value".into(),
                    "tail\\".into()
                ]
            )
            .unwrap()
        )
        .unwrap(),
        "\"original.exe\" \"\" \"space value\" \"quote\\\"value\" \"tail\\\\\"\0"
    );
    assert_eq!(environment(&BTreeMap::new()).unwrap(), vec![0, 0]);
    let values = BTreeMap::from([
        ("z_key".into(), "值".into()),
        ("A_KEY".into(), "space value".into()),
        ("ProgramFiles(x86)".into(), "synthetic program root".into()),
        (
            "CommonProgramFiles(x86)".into(),
            "synthetic common root".into(),
        ),
    ]);
    assert_eq!(
        String::from_utf16(&environment(&values).unwrap()).unwrap(),
        "A_KEY=space value\0COMMONPROGRAMFILES(X86)=synthetic common root\0PROGRAMFILES(X86)=synthetic program root\0Z_KEY=值\0\0"
    );
    for values in [
        BTreeMap::from([("same".into(), "one".into()), ("SAME".into(), "two".into())]),
        BTreeMap::from([("bad=name".into(), "value".into())]),
        BTreeMap::from([("KEY".into(), "embedded\0value".into())]),
    ] {
        assert!(environment(&values).is_err());
    }
    assert!(command(OsStr::new("original.exe"), &["x".repeat(32767).into()]).is_err());
    assert!(command(OsStr::new("original.exe"), &vec![OsString::new(); 129]).is_err());
    assert!(environment(&BTreeMap::from([("KEY".into(), "x".repeat(32767).into())])).is_err());
    for path in [
        r"\\server\share\original.exe",
        r"\\.\device",
        r"\relative",
        r"relative",
    ] {
        assert!(!local_path(Path::new(path)));
    }
}

#[test]
fn invalid_expired_and_query_only_launches_never_start_a_program() {
    let temp = tempfile::tempdir().unwrap();
    let executable = std::env::current_exe().unwrap();
    let job = Job::create(deadline()).unwrap();
    let spec = SpawnSpec {
        program: &executable,
        arguments: &[],
        environment: &BTreeMap::new(),
        directory: temp.path(),
    };
    assert_eq!(
        job.spawn(&spec, Instant::now()).err().unwrap().code,
        "DEADLINE"
    );
    let queried = Job::reopen(job.id(), deadline()).unwrap();
    assert_eq!(
        queried.spawn(&spec, deadline()).err().unwrap().code,
        "JOB_UNVERIFIABLE"
    );
    let invalid_env = BTreeMap::from([("bad=name".into(), "synthetic".into())]);
    assert_eq!(
        job.spawn(
            &SpawnSpec {
                environment: &invalid_env,
                ..spec
            },
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "LAUNCH_INPUT_INVALID"
    );
    let relative = std::path::PathBuf::from("relative-original.exe");
    assert_eq!(
        job.spawn(
            &SpawnSpec {
                program: &relative,
                ..spec
            },
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "LAUNCH_INPUT_INVALID"
    );
    let not_executable = temp.path().join("original-invalid.exe");
    std::fs::write(&not_executable, b"original non-executable synthetic input").unwrap();
    assert_eq!(
        job.spawn(
            &SpawnSpec {
                program: &not_executable,
                ..spec
            },
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "START_FAILED"
    );
    assert!(job.members(deadline()).unwrap().is_empty());
}

#[test]
fn atomic_creation_preserves_arguments_environment_and_root_job_name_after_parent_handles_close() {
    const MODE: &str = "QQ_CLI_TEST_ATOMIC_MODE";
    if std::env::var_os(MODE).is_some() {
        let root = std::path::PathBuf::from(std::env::var_os("QQ_CLI_TEST_ATOMIC_ROOT").unwrap());
        let id: super::super::JobId =
            serde_json::from_str(&std::env::var("QQ_CLI_TEST_ATOMIC_JOB").unwrap()).unwrap();
        let job = Job::reopen(&id, deadline()).unwrap();
        assert!(
            job.members(deadline())
                .unwrap()
                .contains(&std::process::id())
        );
        drop(job); // Only the inherited raw QUERY handle retains the name from here.
        let args: Vec<String> = std::env::args().collect();
        use std::os::windows::io::AsRawHandle;
        std::fs::write(
            root.join("startup.json"),
            serde_json::to_vec(&serde_json::json!({
                "cwdMatches":std::env::current_dir().unwrap()==root,
                "sameNullStream":std::io::stdin().as_raw_handle()==std::io::stdout().as_raw_handle()
                    && std::io::stdout().as_raw_handle()==std::io::stderr().as_raw_handle()
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(std::env::current_dir().unwrap(), root);
        assert_eq!(
            std::io::stdin().as_raw_handle(),
            std::io::stdout().as_raw_handle()
        );
        assert_eq!(
            std::io::stdout().as_raw_handle(),
            std::io::stderr().as_raw_handle()
        );
        use std::io::{Read, Write};
        assert_eq!(std::io::stdin().read(&mut [0]).unwrap(), 0);
        std::io::stdout()
            .write_all(b"original-null-stream-probe")
            .unwrap();
        std::io::stderr()
            .write_all(b"original-null-stream-probe")
            .unwrap();
        assert!(std::env::var_os("PATH").is_none());
        assert_eq!(
            std::env::var("QQ_CLI_TEST_ATOMIC_VALUE").unwrap(),
            "原创 synthetic value"
        );
        std::fs::write(root.join("report.json"), serde_json::to_vec(&args).unwrap()).unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        while !root.join("release").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("original space 空间");
    std::fs::create_dir(&root).unwrap();
    struct Release(std::path::PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, b"release original child");
        }
    }
    let release = Release(root.join("release"));
    let job = Job::create(deadline()).unwrap();
    let id = job.id().clone();
    let mut environment = BTreeMap::from([
        (MODE.into(), "1".into()),
        (
            "QQ_CLI_TEST_ATOMIC_ROOT".into(),
            root.clone().into_os_string(),
        ),
        (
            "QQ_CLI_TEST_ATOMIC_JOB".into(),
            serde_json::to_string(&id).unwrap().into(),
        ),
        (
            "QQ_CLI_TEST_ATOMIC_VALUE".into(),
            "原创 synthetic value".into(),
        ),
    ]);
    if let Some(system) = std::env::var_os("SystemRoot") {
        environment.insert("SYSTEMROOT".into(), system);
    }
    let executable = std::env::current_exe().unwrap();
    let arguments: Vec<OsString> = ["--exact","native::job::spawn::tests::atomic_creation_preserves_arguments_environment_and_root_job_name_after_parent_handles_close",
            "--test-threads=1","--","", "space value", "quote\"value", "tail\\", "原创"].into_iter().map(Into::into).collect();
    // Extra arguments after -- are test filters, but remain observable through std::env::args.
    let spawned = job
        .spawn(
            &SpawnSpec {
                program: &executable,
                arguments: &arguments,
                environment: &environment,
                directory: &root,
            },
            deadline(),
        )
        .unwrap();
    let until = deadline();
    while !root.join("report.json").exists() && Instant::now() < until {
        if spawned.process().verify(deadline()).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        root.join("report.json").exists(),
        "original startup observation: {}",
        std::fs::read_to_string(root.join("startup.json"))
            .unwrap_or_else(|_| "no startup observation".into())
    );
    let report: Vec<String> =
        serde_json::from_slice(&std::fs::read(root.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report[1..],
        arguments
            .iter()
            .map(|a| a.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert!(job.contains(spawned.process(), deadline()).unwrap());
    drop(job);
    let reopened = Job::reopen(&id, deadline()).unwrap();
    assert!(reopened.contains(spawned.process(), deadline()).unwrap());
    drop(reopened);
    drop(release);
    let until = deadline();
    while Instant::now() < until {
        match spawned.process().verify(deadline()) {
            Err(e) if e.code == "PROCESS_EXITED" => break,
            Err(e) if e.code != "PROCESS_UNVERIFIABLE" => {
                panic!("original child observation: {}", e.code)
            }
            _ => (),
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        spawned.process().verify(deadline()).unwrap_err().code,
        "PROCESS_EXITED"
    );
    let until = deadline();
    let mut observations = 0;
    let mut stale_members = 0;
    loop {
        let query = match Job::reopen(&id, until) {
            Ok(query) => query,
            Err(error) => {
                assert_eq!(error.code, "JOB_UNVERIFIABLE");
                break;
            }
        };
        // Process signaling and namespace cleanup are separate observations.
        // Drop each query before waiting so the observer does not retain the name.
        let members = query.members(until).unwrap();
        let unexpected: Vec<_> = members
            .iter()
            .filter(|pid| **pid != spawned.process().identity().process_id())
            .map(|pid| {
                (
                    *pid,
                    Pinned::observe(*pid, until)
                        .map(|p| {
                            p.identity()
                                .image()
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned()
                        })
                        .map_err(|e| e.code),
                )
            })
            .collect();
        // Observed Windows conhost can outlive the test root briefly in this Job.
        // The root's verified exit does not prove every descendant has exited.
        if !unexpected.is_empty() {
            println!("original post-exit auxiliary observations: {unexpected:?}");
        }
        if !members.is_empty() {
            stale_members += 1;
        }
        drop(query);
        observations += 1;
        assert!(
            Instant::now() < until,
            "original Job name remained after root exit"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    println!("original post-exit Job cleanup observations: {observations}");
    println!("original post-exit stale-member observations: {stale_members}");
}
