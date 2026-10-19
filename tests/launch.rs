use std::ffi::OsString;
use std::path::{Path, PathBuf};

use qq_cli::napcat::runtime::launch;

fn ordinary(path: &Path) -> OsString {
    let path = path.canonicalize().unwrap();
    let text = path.to_str().unwrap();
    text.strip_prefix("\\\\?\\").unwrap_or(text).into()
}

fn entries(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect()
}

#[test]
fn native_bootstrap_preserves_three_literal_arguments_without_starting_any_file() {
    let temp = tempfile::tempdir().unwrap();
    let installation = temp.path().join("framework with spaces");
    let qq = temp.path().join("qq with spaces");
    for dir in [&installation, &qq] {
        std::fs::create_dir(dir).unwrap();
    }
    for name in ["napimain.exe", "napiloader.dll", "nativeLoader.cjs"] {
        std::fs::write(installation.join(name), b"original inert synthetic fixture").unwrap();
    }
    let executable = qq.join("QQ.exe");
    std::fs::write(&executable, b"original inert synthetic fixture").unwrap();
    let plan = launch::bootstrap(&executable, &installation).unwrap();
    assert_eq!(
        plan.program,
        installation.canonicalize().unwrap().join("napimain.exe")
    );
    assert_eq!(plan.arguments[0], ordinary(&executable));
    assert_eq!(
        plan.arguments[1],
        ordinary(&installation.join("napiloader.dll"))
    );
    let expected = installation
        .canonicalize()
        .unwrap()
        .join("nativeLoader.cjs");
    let expected = expected.to_str().unwrap();
    assert_eq!(
        plan.arguments[2],
        OsString::from(
            expected
                .strip_prefix("\\\\?\\")
                .unwrap_or(expected)
                .replace('\\', "/")
        )
    );
    assert_eq!(plan.current_directory, installation.canonicalize().unwrap());
    std::fs::remove_file(installation.join("napiloader.dll")).unwrap();
    assert_eq!(
        launch::bootstrap(&executable, &installation)
            .err()
            .unwrap()
            .code,
        "LAUNCH_ENV_INVALID"
    );
}

#[test]
fn child_environment_drops_injection_overrides_and_replaces_search_and_temporary_paths() {
    let temp = tempfile::tempdir().unwrap();
    let qq = temp.path().join("qq");
    let work = temp.path().join("run");
    let system = temp.path().join("system");
    for path in [&qq, &work.join("tmp"), &system.join("System32")] {
        std::fs::create_dir_all(path).unwrap();
    }
    let system_text = system.to_str().unwrap();
    let input = entries(&[
        ("SystemRoot", system_text),
        ("USERPROFILE", "synthetic-user-home"),
        ("node_options", "--require=untrusted"),
        ("NAPCAT_QQ_VERSION_CONFIG_PATH", "untrusted"),
        ("NAPCAT_WORKDIR", "untrusted"),
        ("PATH", "untrusted"),
        ("TEMP", "untrusted"),
        ("HTTP_PROXY", "untrusted"),
        ("UNRELATED_SECRET", "synthetic secret"),
    ]);
    let environment = launch::environment(input.clone(), &qq, &work).unwrap();
    assert_eq!(
        environment[&OsString::from("USERPROFILE")],
        "synthetic-user-home"
    );
    for name in [
        "NODE_OPTIONS",
        "NAPCAT_QQ_VERSION_CONFIG_PATH",
        "HTTP_PROXY",
        "UNRELATED_SECRET",
    ] {
        assert!(!environment.contains_key(&OsString::from(name)));
    }
    assert_eq!(
        environment[&OsString::from("NAPCAT_WORKDIR")],
        ordinary(&work)
    );
    assert_eq!(
        environment[&OsString::from("TEMP")],
        ordinary(&work.join("tmp"))
    );
    assert_eq!(
        std::env::split_paths(&environment[&OsString::from("PATH")]).collect::<Vec<_>>(),
        [
            PathBuf::from(ordinary(&qq)),
            PathBuf::from(ordinary(&system.join("System32"))),
            PathBuf::from(ordinary(&system))
        ]
    );
    assert_eq!(
        launch::environment(input.into_iter().rev(), &qq, &work).unwrap(),
        environment
    );
    // This constructor does not set the current process environment or create runtime files.
    assert_eq!(std::fs::read_dir(&work).unwrap().count(), 1);
}

#[test]
fn child_environment_rejects_ambiguous_system_roots_and_unisolated_directories() {
    let temp = tempfile::tempdir().unwrap();
    let qq = temp.path().join("qq");
    let work = temp.path().join("run");
    let system = temp.path().join("system");
    for path in [&qq, &work.join("tmp"), &system.join("System32")] {
        std::fs::create_dir_all(path).unwrap();
    }
    let root = system.to_str().unwrap();
    for input in [
        entries(&[]),
        entries(&[("SYSTEMROOT", root), ("systemroot", "different")]),
        entries(&[("SYSTEMROOT", root), ("WINDIR", qq.to_str().unwrap())]),
        entries(&[("SYSTEMROOT", root), ("APPDATA", "invalid\0value")]),
    ] {
        assert_eq!(
            launch::environment(input, &qq, &work).unwrap_err().code,
            "LAUNCH_ENV_INVALID"
        );
    }
    let input = entries(&[("SYSTEMROOT", root)]);
    assert!(launch::environment(input.clone(), &qq, &qq).is_err());
    assert!(launch::environment(input.clone(), Path::new("relative"), &work).is_err());
    std::fs::remove_dir(work.join("tmp")).unwrap();
    assert!(launch::environment(input, &qq, &work).is_err());
}
