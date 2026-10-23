#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::io::{IoSlice, Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn actual_cli_worker_copies_descriptors_and_never_emits_cli_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("original");
    let destination = temp.path().join("copy");
    std::fs::write(&source, b"original private fixture").unwrap();
    let source = std::fs::File::open(source).unwrap();
    let target = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&destination)
        .unwrap();
    let (mut parent, child) = UnixStream::pair().unwrap();
    parent
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let child: OwnedFd = child.into();
    let mut worker = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(["--internal-macos-copy-v1", "10000"])
        .env_clear()
        .stdin(Stdio::from(child))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let descriptors = [source.as_fd(), target.as_fd()];
    let mut bytes = [std::mem::MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(2))];
    let mut control = rustix::net::SendAncillaryBuffer::new(&mut bytes);
    assert!(control.push(rustix::net::SendAncillaryMessage::ScmRights(&descriptors)));
    assert_eq!(
        rustix::net::sendmsg(
            &parent,
            &[IoSlice::new(&[2])],
            &mut control,
            rustix::net::SendFlags::empty()
        )
        .unwrap(),
        1
    );
    let mut reply = [0_u8];
    parent.read_exact(&mut reply).unwrap();
    assert_eq!(reply, [101]);
    parent.write_all(&[0]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = worker.try_wait().unwrap() {
            assert_eq!(status.code(), Some(101));
            break;
        }
        if Instant::now() >= deadline {
            let _ = worker.kill();
            let _ = worker.wait();
            panic!("worker did not finish");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = worker.wait_with_output().unwrap();
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert_eq!(
        std::fs::read(destination).unwrap(),
        b"original private fixture"
    );
}

#[test]
fn actual_worker_rejects_malformed_private_arguments_silently() {
    for args in [
        vec!["--internal-macos-copy-v1"],
        vec!["--internal-macos-copy-v1", "0"],
        vec!["--internal-macos-copy-v1", "1800001"],
        vec!["--internal-macos-copy-v1", "100", "extra"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(102));
        assert!(result.stdout.is_empty() && result.stderr.is_empty());
    }
}
