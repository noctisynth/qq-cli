use super::*;
use std::process::Command;

#[test]
fn inherited_profile_access_does_not_expose_other_private_state_even_outside_users() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let state = base.join("state.+[qa]");
    let current = state.join("runtimes/current");
    let old = state.join("runtimes/previous");
    let home = old.join("profile-home");
    let foreign = state.join("runtimes/foreign/profile-home");
    let framework = state.join("installations/framework");
    let app = current.join("QQ.app/Contents/MacOS");
    let uploads = state.join("attachments/prepare-current-test");
    let retained = state.join("attachments/prepare-previous-test");
    let other_uploads = state.join("attachments/prepare-foreign-test");
    let similar = state.join("attachments/prepare-currentish-test");
    for path in [
        &home,
        &foreign,
        &framework,
        &app,
        &current.join("temp"),
        &uploads,
        &retained,
        &other_uploads,
        &similar,
    ] {
        std::fs::create_dir_all(path).unwrap();
    }
    for path in [
        home.join("session"),
        foreign.join("session"),
        old.join("private-config"),
        state.join("registry"),
        framework.join("module"),
        uploads.join("image"),
        retained.join("file"),
        other_uploads.join("image"),
        similar.join("image"),
    ] {
        std::fs::write(path, b"synthetic private data").unwrap();
    }
    let escaped = home.join("foreign-link");
    std::os::unix::fs::symlink(foreign.join("session"), &escaped).unwrap();
    let upload_escape = uploads.join("foreign-link");
    std::os::unix::fs::symlink(other_uploads.join("image"), &upload_escape).unwrap();
    let text = |path: &Path| serde_json::to_string(path.to_str().unwrap()).unwrap();
    let code = format!(
        r#"#include <stdio.h>
static int readable(const char *p){{ FILE *f=fopen(p,"r"); if(!f)return 0; fclose(f); return 1; }}
static int writable(const char *p){{ FILE *f=fopen(p,"a"); if(!f)return 0; fclose(f); return 1; }}
int main(void){{
 int ok=readable({session}) && writable({session}) && readable({module}) && writable({output})
   && !readable({other}) && !readable({escaped}) && !readable({config}) && !readable({registry}) && !writable({config})
   && readable({upload}) && !writable({upload}) && readable({retained}) && !writable({retained})
   && !readable({other_upload}) && !readable({similar}) && !readable({upload_escape});
 printf("%d",ok); return ok?0:1;
}}
"#,
        session = text(&home.join("session")),
        module = text(&framework.join("module")),
        output = text(&current.join("temp/output")),
        other = text(&foreign.join("session")),
        escaped = text(&escaped),
        config = text(&old.join("private-config")),
        registry = text(&state.join("registry")),
        upload = text(&uploads.join("image")),
        retained = text(&retained.join("file")),
        other_upload = text(&other_uploads.join("image")),
        similar = text(&similar.join("image")),
        upload_escape = text(&upload_escape),
    );
    let c = base.join("scope.c");
    std::fs::write(&c, code).unwrap();
    let executable = app.join("scope");
    assert!(
        Command::new("/usr/bin/clang")
            .arg(c)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success()
    );
    let legacy = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &profile(&current, &framework, &home).unwrap()])
        .arg(&executable)
        .env_clear()
        .output()
        .unwrap();
    assert!(
        !legacy.status.success(),
        "current-only uploads must reproduce the historical download failure"
    );
    let output = Command::new("/usr/bin/sandbox-exec")
        .args([
            "-p",
            &profile_with_history(&current, &framework, &home, &["previous".into()]).unwrap(),
        ])
        .arg(executable)
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "profile isolation failed: {:?}",
        output.stdout
    );
    assert_eq!(output.stdout, b"1");
}
