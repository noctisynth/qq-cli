#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::{
    fs::File,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const FLAG: &str = "--internal-macos-mount-v1";
fn wait(child: &mut Child) -> i32 {
    let end = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap();
        }
        assert!(
            Instant::now() < end,
            "retain fixture: worker exit was not confirmed"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn spawn(image: &Path, mount: &Path, input: File, root: File, budget: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .arg(FLAG)
        .arg(image)
        .arg(mount)
        .arg(budget)
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(input))
        .stderr(Stdio::from(root))
        .spawn()
        .unwrap()
}
#[test]
fn malformed_worker_requests_are_silent() {
    for args in [
        vec![FLAG],
        vec![FLAG, "/missing", "/missing", "0"],
        vec![FLAG, "/missing", "/missing", "100", "extra"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(114));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
}
fn private_dir(path: &Path) {
    std::fs::create_dir(path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
struct Fixture {
    base: PathBuf,
    image: PathBuf,
    mount: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap().keep().canonicalize().unwrap();
        let source = base.join("source");
        private_dir(&source);
        std::fs::write(source.join("fixture.txt"), b"original synthetic fixture").unwrap();
        let image = base.join("synthetic.dmg");
        let mount = base.join("mount");
        private_dir(&mount);
        let out = Command::new("/usr/bin/hdiutil")
            .args(["create", "-srcfolder"])
            .arg(source)
            .args(["-fs", "HFS+", "-format", "UDZO"])
            .arg(&image)
            .output()
            .unwrap();
        assert!(out.status.success(), "synthetic image creation failed");
        std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o600)).unwrap();
        Self { base, image, mount }
    }
    fn spawn(&self, budget: &str) -> Child {
        spawn(
            &self.image,
            &self.mount,
            File::open(&self.image).unwrap(),
            File::open(&self.mount).unwrap(),
            budget,
        )
    }
}
#[test]
fn worker_requires_permission_and_original_descriptors_before_real_synthetic_attach() {
    let f = Fixture::new();
    let original = std::fs::read(&f.image).unwrap();
    for mode in 0..4 {
        let mut child = f.spawn(if mode == 3 { "100" } else { "10000" });
        match mode {
            0 => {
                child.stdin.take();
            }
            1 => child.stdin.as_mut().unwrap().write_all(&[0]).unwrap(),
            2 => child.stdin.as_mut().unwrap().write_all(&[1, 2]).unwrap(),
            _ => (),
        }
        assert_eq!(wait(&mut child), if mode == 3 { 115 } else { 114 });
        assert!(!f.mount.join("fixture.txt").exists());
    }
    for replace_mount in [false, true] {
        let input = File::open(&f.image).unwrap();
        let root = File::open(&f.mount).unwrap();
        let path = if replace_mount { &f.mount } else { &f.image };
        let old = path.with_extension("old");
        std::fs::rename(path, &old).unwrap();
        if replace_mount {
            private_dir(path);
        } else {
            std::fs::write(path, &original).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mut child = spawn(&f.image, &f.mount, input, root, "10000");
        child.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
        assert_eq!(wait(&mut child), 114);
        if replace_mount {
            std::fs::remove_dir(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::rename(old, path).unwrap();
    }
    let mut child = f.spawn("30000");
    child.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    let code = wait(&mut child);
    let checked = f.mount.join("fixture.txt").exists()
        && std::fs::read(f.mount.join("fixture.txt")).unwrap() == b"original synthetic fixture"
        && std::fs::write(f.mount.join("not-writable.txt"), b"reject").is_err();
    // Only this test created the mount; never force detach on a failure.
    let detached = Command::new("/usr/bin/hdiutil")
        .arg("detach")
        .arg(&f.mount)
        .output()
        .unwrap();
    assert!(
        detached.status.success(),
        "retain fixture: normal detach failed"
    );
    assert_eq!(code, 113);
    assert!(checked);
    assert_eq!(std::fs::read(&f.image).unwrap(), original);
    std::fs::remove_dir_all(f.base).unwrap();
}

#[path = "support/macos_release.rs"]
mod release;
