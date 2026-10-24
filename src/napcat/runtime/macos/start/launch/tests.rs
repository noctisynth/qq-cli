use super::*;
use crate::{
    napcat::{
        install::catalog,
        runtime::{
            macos::{adapter, layout, material},
            runtime_config::Ports,
            runtime_store::Plan,
        },
    },
    storage::state::{Installation, private_dir},
};
use std::{cell::RefCell, collections::BTreeMap, path::Path};
#[derive(Default)]
struct Memory(RefCell<BTreeMap<String, String>>);
impl Credentials for Memory {
    fn get(&self, key: &str) -> Result<String> {
        self.0
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::new("CREDENTIAL_NOT_FOUND", "Synthetic key absent", 4))
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.0.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no deletion")
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
#[test]
fn actual_sandbox_child_is_recorded_once_and_cannot_write_or_execute_outside_its_app() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let store = Store::new(base.join("state"));
    let refs = References::with_key(store.root().join("references"), [59; 32]);
    let lease = crate::napcat::runtime::ports::Lease::dynamic(deadline()).unwrap();
    let ports: Ports = lease.ports();
    let release = catalog::lookup("4.18.41").unwrap();
    let runtime = Plan {
        instance_id: "instance".into(),
        run_id: "run".into(),
        version: release.version.into(),
        ports,
    };
    let framework = base.join("framework");
    private_dir(&framework).unwrap();
    let installed = Installation {
        id: "install".into(),
        version: release.version.into(),
        digest: release.archive.sha256.into(),
        directory: framework.clone(),
    };
    let instance = Instance {
        id: "instance".into(),
        run_id: "run".into(),
        installation_id: installed.id.clone(),
        endpoint: format!("http://127.0.0.1:{}/", ports.http),
        ws_endpoint: Some(format!("ws://127.0.0.1:{}/event", ports.events)),
        credential_key: crate::napcat::runtime::runtime_store::api_credential_key(
            "instance", "run",
        )
        .unwrap(),
        state: "preparing".into(),
        account: None,
        process_id: None,
        process_started: None,
    };
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances.push(instance);
    tx.registry.installations.push(installed.clone());
    tx.commit().unwrap();
    let layout::Preparation::Ready(layout) =
        layout::prepare(&store, &refs, "instance", "layout", deadline()).unwrap()
    else {
        panic!("layout");
    };
    let memory = Memory::default();
    let material::Preparation::Ready(material) =
        material::prepare(&store, &refs, &memory, &runtime, "material", deadline()).unwrap()
    else {
        panic!("material");
    };
    let adapter = adapter::Prepared::inspect(layout.directory(), &framework).unwrap();
    let app = layout.directory().join("QQ.app/Contents/MacOS");
    std::fs::create_dir_all(&app).unwrap();
    let marker = layout.directory().join("temp/entered");
    let outside = base.join("outside");
    // This is an existing project file, never a private account or credential.
    let denied_read = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let c_string = |path: &Path| serde_json::to_string(path.to_str().unwrap()).unwrap();
    let code = format!(
        r#"#include <stdio.h>
#include <unistd.h>
#include <sys/wait.h>
#include <stdlib.h>
#include <string.h>
int main(void) {{
 FILE *f=fopen({outside},"w"); int write_denied=!f; if(f) fclose(f);
 f=fopen({read},"r"); int read_denied=!f; if(f) fclose(f);
 pid_t p=fork(); if(p==0){{execl("/usr/bin/true","true",NULL);_exit(99);}}
 int status=0; waitpid(p,&status,0);
 int exec_denied=WIFEXITED(status)&&WEXITSTATUS(status)==99;
 f=fopen({marker},"w"); if(!f) return 2;
 const char *jwt=getenv("NAPCAT_WEBUI_JWT_SECRET_KEY");
 fprintf(f,"%d %d %d %d",write_denied,read_denied,exec_denied,jwt&&strlen(jwt)==64); fclose(f);
 for(int i=0;i<60;i++) sleep(1); return 0;
}}
"#,
        outside = c_string(&outside),
        read = c_string(&denied_read),
        marker = c_string(&marker)
    );
    let c = base.join("fixture.c");
    std::fs::write(&c, code).unwrap();
    let output = Command::new("/usr/bin/clang")
        .arg(&c)
        .args(["-o"])
        .arg(app.join("QQ"))
        .output()
        .unwrap();
    assert!(output.status.success(), "synthetic compiler failed");
    let deployment = start_intent::Target {
        instance_id: "instance".into(),
        run_id: "run".into(),
        installation_id: "install".into(),
        installation_digest: release.archive.sha256.into(),
        input_hash: "a".repeat(64),
    };
    let mut launch = Launch::new(prepare::Prepared {
        runtime,
        deployment,
        material,
        adapter,
    })
    .unwrap();
    let prepared_environment = environment(&launch.prepared).unwrap();
    assert!(
        prepared_environment
            .get(std::ffi::OsStr::new("NAPCAT_WEBUI_JWT_SECRET_KEY"))
            .is_some_and(|value| value == launch.prepared.material.bundle().jwt_secret())
    );
    drop(lease);
    let tx = store.transaction(deadline()).unwrap();
    assert!(
        launch
            .dispatch(
                &tx,
                &Context {
                    store: &store,
                    references: &refs,
                    credentials: &memory
                },
                "start",
                deadline(),
                &|| Ok(())
            )
            .is_err(),
        "missing authenticated deployment must reject production dispatch"
    );
    assert!(!marker.exists());
    assert!(
        launch
            .dispatch_checked(
                &tx,
                &refs,
                "start",
                deadline(),
                &|| Err(conflict()),
                &|| Ok(())
            )
            .is_err()
    );
    assert!(!marker.exists());
    drop(tx);
    let start_intent::Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "start", &launch.target, deadline()).unwrap()
    else {
        panic!("fresh");
    };
    let mut owner = None;
    let receipt = intent
        .dispatch(|tx| {
            // Synthetic setup deliberately has no upstream i1/d2 assets. Production
            // dispatch supplies their real preflight; the process/journal effects
            // below are the same implementation, with no PID adoption path.
            owner = Some(launch.dispatch_checked(
                tx,
                &refs,
                "start",
                deadline(),
                &|| Ok(()),
                &|| Ok(()),
            )?);
            Ok(())
        })
        .unwrap();
    assert_eq!(receipt.outcome, start_intent::Outcome::Dispatched);
    let owner = owner.unwrap();
    let end = deadline();
    while !marker.exists() {
        assert!(Instant::now() < end);
        owner.verify(deadline()).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
    let observed = std::fs::read_to_string(&marker).unwrap();
    let tx = store.transaction(deadline()).unwrap();
    let current = tx.registry.instances[0].clone();
    let service = run::reopen_service(&tx, &refs, &current, &installed, deadline()).unwrap();
    assert_eq!(
        service.owner().identity().process_id(),
        owner.identity().process_id()
    );
    assert_eq!(
        service.control_endpoint(),
        Some(format!("http://127.0.0.1:{}/", ports.control).as_str())
    );
    assert!(service.compatibility() == Some(&launch.key));
    launch.verify_service(&service, deadline()).unwrap();
    let original = launch.target.input_hash.clone();
    launch.target.input_hash = "0".repeat(64);
    assert!(
        launch.verify_service(&service, deadline()).is_err(),
        "a legacy or different software/launch target cannot inherit this authenticated run"
    );
    launch.target.input_hash = original;
    drop(service);
    drop(tx);
    assert!(matches!(
        start_intent::begin(&store, &refs, "start", &launch.target, deadline()).unwrap(),
        start_intent::Begin::Existing(_)
    ));
    assert!(start_intent::begin(&store, &refs, "other", &launch.target, deadline()).is_err());
    // An authentic legacy record remains readable. Its absent software key
    // cannot trigger template reconstruction, API reads or a new k2 witness.
    let path = std::fs::read_dir(store.root().join("macos-runs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut pointer: serde_json::Value = crate::storage::config::read_json(&path, 4096).unwrap();
    let mut body: serde_json::Value = refs
        .get("j2", pointer["reference"].as_str().unwrap())
        .unwrap();
    body.as_object_mut().unwrap().remove("compatibility");
    let mut tx = store.transaction(deadline()).unwrap();
    pointer["reference"] = refs.issue("j2", &body).unwrap().into();
    crate::storage::state::atomic_json(&path, &pointer).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.registry.instances[0].account = Some("10001".into());
    let ready = tx.registry.instances[0].clone();
    tx.commit().unwrap();
    let tx = store.transaction(deadline()).unwrap();
    let legacy = run::reopen_service(&tx, &refs, &ready, &installed, deadline()).unwrap();
    assert!(legacy.compatibility().is_none());
    assert!(legacy.owner().identity() == owner.identity());
    drop(legacy);
    drop(tx);
    crate::napcat::runtime::macos::compatibility::record(
        &Context {
            store: &store,
            references: &refs,
            credentials: &memory,
        },
        &crate::storage::config::Config::default(),
        &ready,
        deadline(),
    )
    .unwrap();
    assert!(!crate::napcat::runtime::macos::compatibility::has_evidence(&store).unwrap());
    let target = crate::native::macos_terminate::Target::capture(&owner, deadline()).unwrap();
    target.terminate_once(&owner, deadline()).unwrap();
    let pid = rustix::process::Pid::from_raw(owner.identity().process_id() as i32).unwrap();
    rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::empty()).unwrap();
    assert_eq!(observed, "1 1 1 1");
    assert!(!outside.exists());
}
