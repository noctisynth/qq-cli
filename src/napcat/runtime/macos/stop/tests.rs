use super::{
    members::{self, Checkpoint, Member, Phase},
    *,
};
use crate::{
    core::error::Result,
    native::{
        macos_terminate::Target,
        process::{self, ExecutableSet, Pinned},
    },
};
use std::{
    io::{BufRead, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
mod session;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

struct Program {
    _temp: tempfile::TempDir,
    root: PathBuf,
    executable: PathBuf,
}
impl Program {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("Synthetic.app");
        std::fs::create_dir_all(root.join("Contents/MacOS")).unwrap();
        let source = base.join("synthetic.c");
        std::fs::write(
            &source,
            r#"#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
int main(int argc, char **argv) {
 pid_t child=0;
 if(argc>1 && argv[1][0]=='t') {
  child=fork();
  if(child<0) return 2;
  if(child==0) { if(setsid()<0) _exit(3); for(;;) pause(); }
 }
 printf("%d\n",child); fflush(stdout);
 char command;
 while(read(0,&command,1)==1) {
  if(command=='x') _exit(0);
  if(command=='e') { execl(argv[0],argv[0],"leaf",NULL); _exit(4); }
 }
 if(child) { kill(child,SIGKILL); waitpid(child,NULL,0); }
 return 0;
}
"#,
        )
        .unwrap();
        let executable = root.join("Contents/MacOS/QQ");
        assert!(
            Command::new("/usr/bin/clang")
                .args(["-O0", "-o"])
                .arg(&executable)
                .arg(source)
                .status()
                .unwrap()
                .success()
        );
        std::fs::write(root.join("Contents/Info.plist"), br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.qq-cli.stop-fixture</string><key>CFBundleExecutable</key><string>QQ</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
        assert!(
            Command::new("/usr/bin/codesign")
                .args(["--force", "--sign", "-", "--timestamp=none"])
                .arg(&root)
                .output()
                .unwrap()
                .status
                .success()
        );
        Self {
            _temp: temp,
            root,
            executable,
        }
    }
    fn files(&self) -> ExecutableSet {
        ExecutableSet::open(
            &self.root,
            std::slice::from_ref(&self.executable),
            deadline(),
        )
        .unwrap()
    }
    fn spawn(&self, tree: bool) -> Owned {
        let mut command = Command::new(&self.executable);
        command.arg(if tree { "tree" } else { "leaf" });
        Self::spawn_command(command)
    }
    fn spawn_command(mut command: Command) -> Owned {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let stdout = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let pid: u32 = receive
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .parse()
            .unwrap();
        let owner = Pinned::observe(child.id(), deadline()).unwrap();
        let target = Target::capture(&owner, deadline()).unwrap();
        let descendant = (pid != 0).then(|| {
            let descendant = Pinned::observe(pid, deadline()).unwrap();
            assert!(descendant.is_direct_child_of(&owner, deadline()).unwrap());
            Target::capture(&descendant, deadline()).unwrap()
        });
        Owned {
            child,
            target,
            descendant,
            output: receive,
        }
    }
}
struct Owned {
    child: Child,
    target: Target,
    descendant: Option<Target>,
    output: std::sync::mpsc::Receiver<String>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        // Only fixture children whose original parent relation was checked above.
        if let Some(target) = &self.descendant
            && let Ok(owner) = target.reopen(deadline())
        {
            let _ = target.terminate_once(&owner, deadline());
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
struct Memory {
    saved: Vec<Member>,
    interrupt: Option<Phase>,
    calls: usize,
}
impl Memory {
    fn new(target: &Target) -> Self {
        Self {
            saved: vec![Member::root(target.clone())],
            interrupt: None,
            calls: 0,
        }
    }
}
impl Checkpoint for Memory {
    fn guard(&self) -> Result<()> {
        Ok(())
    }
    fn save(&mut self, members: &[Member]) -> Result<()> {
        self.calls += 1;
        self.saved = members.to_vec();
        if self.interrupt == Some(Phase::Intent)
            && members.iter().any(|m| m.suspend == Phase::Intent)
        {
            return Err(unknown());
        }
        if self.interrupt == Some(Phase::Observed)
            && members.iter().any(|m| m.terminate == Phase::Intent)
        {
            return Err(unknown());
        }
        Ok(())
    }
}

#[test]
fn detached_process_group_is_stopped_only_after_parent_link_and_all_generations_exit() {
    let program = Program::new();
    let mut owned = program.spawn(true);
    let mut memory = Memory::new(&owned.target);
    let mut members = memory.saved.clone();
    members::execute(&program.files(), &mut members, &mut memory, deadline()).unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[1].parent, Some(0));
    assert!(
        members
            .iter()
            .all(|m| process::exited(m.target.identity(), deadline()).unwrap())
    );
    assert!(members.iter().all(|m| m.terminate == Phase::Observed));
    assert!(owned.child.wait().unwrap().code().is_none());
}
#[test]
fn matching_foreign_image_is_preserved_without_adoption_or_termination() {
    let program = Program::new();
    let owned = program.spawn(false);
    let mut foreign = program.spawn(false);
    let mut memory = Memory::new(&owned.target);
    let mut members = memory.saved.clone();
    assert_eq!(
        members::execute(&program.files(), &mut members, &mut memory, deadline())
            .unwrap_err()
            .code,
        "STOP_UNVERIFIABLE"
    );
    assert_eq!(members.len(), 1);
    assert!(foreign.child.try_wait().unwrap().is_none());
    assert!(
        !foreign
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
    assert!(
        owned
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
}
#[test]
fn exited_parent_does_not_grant_ownership_of_an_unrecorded_orphan() {
    let program = Program::new();
    let mut owned = program.spawn(true);
    owned.child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    assert!(owned.child.wait().unwrap().success());
    let mut memory = Memory::new(&owned.target);
    let mut members = memory.saved.clone();
    assert_eq!(
        members::execute(&program.files(), &mut members, &mut memory, deadline())
            .unwrap_err()
            .code,
        "STOP_UNVERIFIABLE"
    );
    assert_eq!(members.len(), 1);
    assert!(
        !owned
            .descendant
            .as_ref()
            .unwrap()
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
}
#[test]
fn original_root_exit_and_empty_confined_inventory_complete_without_signals() {
    let program = Program::new();
    let mut owned = program.spawn(false);
    owned.child.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    assert!(owned.child.wait().unwrap().success());
    let mut memory = Memory::new(&owned.target);
    let mut members = memory.saved.clone();
    members::execute(&program.files(), &mut members, &mut memory, deadline()).unwrap();
    assert_eq!(memory.calls, 0);
}
#[test]
fn interrupted_suspension_intent_is_never_resent() {
    let program = Program::new();
    let mut owned = program.spawn(false);
    let mut memory = Memory::new(&owned.target);
    memory.interrupt = Some(Phase::Intent);
    let mut members = memory.saved.clone();
    assert!(members::execute(&program.files(), &mut members, &mut memory, deadline()).is_err());
    assert!(memory.saved[0].suspend == Phase::Intent);
    memory.interrupt = None;
    let mut recovered = memory.saved.clone();
    assert!(
        members::execute(
            &program.files(),
            &mut recovered,
            &mut memory,
            Instant::now() + Duration::from_millis(100)
        )
        .is_err()
    );
    assert!(
        !owned
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
    assert!(owned.child.try_wait().unwrap().is_none());
}
#[test]
fn interrupted_termination_waits_for_original_exit_and_does_not_repeat_effect() {
    let program = Program::new();
    let mut owned = program.spawn(false);
    let mut memory = Memory::new(&owned.target);
    memory.interrupt = Some(Phase::Observed);
    let mut members = memory.saved.clone();
    assert!(members::execute(&program.files(), &mut members, &mut memory, deadline()).is_err());
    assert!(memory.saved[0].terminate == Phase::Intent);
    memory.interrupt = None;
    let mut recovered = memory.saved.clone();
    assert!(
        members::execute(
            &program.files(),
            &mut recovered,
            &mut memory,
            Instant::now() + Duration::from_millis(100)
        )
        .is_err()
    );
    assert!(owned.child.try_wait().unwrap().is_none());
    let owner = owned.target.reopen(deadline()).unwrap();
    owned.target.terminate_once(&owner, deadline()).unwrap();
    owned.child.wait().unwrap();
    let mut recovered = memory.saved.clone();
    members::execute(&program.files(), &mut recovered, &mut memory, deadline()).unwrap();
}
#[test]
fn same_image_exec_cannot_refresh_saved_audit_generation() {
    let program = Program::new();
    let mut owned = program.spawn(false);
    let mut memory = Memory::new(&owned.target);
    owned.child.stdin.as_mut().unwrap().write_all(b"e").unwrap();
    assert_eq!(
        owned.output.recv_timeout(Duration::from_secs(5)).unwrap(),
        "0"
    );
    let mut members = memory.saved.clone();
    assert!(members::execute(&program.files(), &mut members, &mut memory, deadline()).is_err());
    assert_eq!(memory.calls, 0);
    assert!(owned.child.try_wait().unwrap().is_none());
    assert!(
        !Pinned::observe(owned.child.id(), deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
}
