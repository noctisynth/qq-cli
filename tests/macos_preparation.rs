#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use qq_cli::napcat::runtime::macos::adapter::Prepared;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn fixture(base: &Path) -> (PathBuf, PathBuf) {
    // Quotes, backslashes and JS-looking text must remain literal path data.
    let root = base.join(r#"instance \"';throw Error(1); 雪"#);
    let framework = base.join("framework ' with spaces");
    std::fs::create_dir_all(&framework).unwrap();
    let home = root.join("profile-home/Library/Containers/com.tencent.qq/Data");
    for leaf in [
        "Desktop",
        "Documents",
        "Downloads",
        "Music",
        "Pictures",
        "Movies",
        "Library/Application Support/QQ",
    ] {
        std::fs::create_dir_all(home.join(leaf)).unwrap();
    }
    for leaf in [
        "temp",
        "logs",
        "config",
        "cache",
        "napcat-data/config",
        "napcat-data/logs",
        "napcat-data/cache",
        "napcat-data/plugins",
    ] {
        std::fs::create_dir_all(root.join(leaf)).unwrap();
    }
    (root, framework)
}

#[test]
fn environment_and_arguments_are_literal_and_do_not_inherit_process_overrides() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let (root, framework) = fixture(&base);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let env = plan.environment();
    assert_eq!(env.len(), 8);
    assert_eq!(env[&OsString::from("HOME")], root.join("profile-home"));
    assert_eq!(
        env[&OsString::from("CFFIXED_USER_HOME")],
        root.join("profile-home")
    );
    assert_eq!(
        env[&OsString::from("NAPCAT_WORKDIR")],
        root.join("napcat-data")
    );
    assert_eq!(
        env[&OsString::from("PATH")],
        "/usr/bin:/bin:/usr/sbin:/sbin"
    );
    for name in [
        "NODE_OPTIONS",
        "DYLD_INSERT_LIBRARIES",
        "ELECTRON_RUN_AS_NODE",
        "HTTP_PROXY",
        "NAPCAT_QQ_VERSION_CONFIG_PATH",
    ] {
        assert!(!env.contains_key(&OsString::from(name)));
    }
    let mut expected = OsString::from("--user-data-dir=");
    expected.push(plan.electron_home().join("Library/Application Support/QQ"));
    assert_eq!(plan.arguments(), [expected]);
    assert_eq!(plan.current_directory(), root);
    assert_eq!(plan.executable(), root.join("QQ.app/Contents/MacOS/QQ"));
    // Inspection cannot create the app, entry or other deployment artifacts.
    assert!(!root.join("QQ.app").exists());
}

#[test]
fn inherited_profile_keeps_data_arguments_and_home_but_not_per_run_paths() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let (original, framework) = fixture(&base);
    let (staged, _) = fixture(&base.join("staged"));
    let next = base.join("next run");
    std::fs::rename(staged, &next).unwrap();
    let profile = original.join("profile-home");
    let marker = profile.join("synthetic-session");
    std::fs::write(&marker, b"preserve").unwrap();
    let first = Prepared::inspect(&original, &framework).unwrap();
    let inherited = Prepared::inspect_with_profile(&next, &framework, &profile).unwrap();
    assert_eq!(first.arguments(), inherited.arguments());
    assert_eq!(first.electron_home(), inherited.electron_home());
    assert_eq!(inherited.profile_home(), profile);
    let environment = inherited.environment();
    assert_eq!(environment[&OsString::from("HOME")], profile);
    assert_eq!(environment[&OsString::from("CFFIXED_USER_HOME")], profile);
    assert_eq!(environment[&OsString::from("TMPDIR")], next.join("temp"));
    assert_eq!(
        environment[&OsString::from("NAPCAT_WORKDIR")],
        next.join("napcat-data")
    );
    assert_eq!(std::fs::read(marker).unwrap(), b"preserve");
    assert!(!next.join("profile-home/synthetic-session").exists());
    assert_ne!(
        first.main_source().unwrap(),
        inherited.main_source().unwrap()
    );
    std::fs::remove_dir(profile.join("Library/Containers/com.tencent.qq/Data/Movies")).unwrap();
    assert!(Prepared::inspect_with_profile(&next, &framework, &profile).is_err());
}

#[test]
fn generated_main_executes_only_fixed_imports_and_passes_paths_as_data() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let (root, framework) = fixture(&base);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let (staged, _) = fixture(&base.join("staged"));
    let next = base.join("next run");
    std::fs::rename(staged, &next).unwrap();
    let inherited = Prepared::inspect_with_profile(&next, &framework, plan.profile_home()).unwrap();
    let input = serde_json::json!([
        {"source":plan.main_source().unwrap(),"root":root,"framework":framework,"profile":plan.profile_home()},
        {"source":inherited.main_source().unwrap(),"root":next,"framework":framework,"profile":inherited.profile_home()},
    ]);
    // VM has a stubbed require/process, never Electron or QQ. Verify the actual
    // generated program, including synchronous/asynchronous failure callbacks.
    let script = r#"
const vm=require('node:vm'),assert=require('node:assert/strict');
for(const input of JSON.parse(require('node:fs').readFileSync(0,'utf8'))) {
for(const mode of ['ok','throw','async']) {
 const imports=[],writes=[];let called=0;
 const process={stderr:{write:value=>writes.push(value)}};
 // Electron's regular fs presents ASAR archives as virtual directories.
 // Adapter layout validation must receive the physical filesystem instead.
 const app={}; const fs={physical:true}; const electronFs={physical:false};
 const filename=input.root+'/adapter/synthetic/main.cjs',launcher={},defer=()=>{};
 const load=name=>{
   imports.push(name);
   if(name==='node:fs') return electronFs;
   if(name==='original-fs') return fs;
   if(name==='electron') return {app};
   assert.equal(name,'./entry.cjs');
   return {start(config,host){
     called++;assert.equal(config.instanceRoot,input.root);assert.equal(config.frameworkRoot,input.framework);
     assert.equal(config.profileHome,input.profile);
     assert.equal(host.app,app);assert.equal(host.fs,fs);assert.equal(host.process,process);assert.equal(host.load,load);
     assert.equal(host.entryPath,filename);assert.equal(host.defer,defer);assert.equal(host.getLauncher(),launcher);
     if(mode==='throw') throw Error('private synthetic failure');
     if(mode==='async') host.onFailure('private synthetic failure');
   }};
 };
 vm.runInNewContext(input.source,{require:load,process,__filename:filename,setImmediate:defer,launcher});
 assert.equal(called,1);assert.deepEqual(imports,['./entry.cjs','original-fs','electron']);
 assert.deepEqual(writes,mode==='ok'?[]:['QQ_CLI_ADAPTER_START_FAILED\n']);
 assert.equal(process.exitCode,mode==='ok'?undefined:1);
}
}
"#;
    let mut child = Command::new("node")
        .args(["-e", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn preparation_rejects_aliases_missing_paths_and_overlapping_roots() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let (root, framework) = fixture(&base);
    for (a, b) in [(&root, &root), (&base, &framework), (&root, &base)] {
        assert!(Prepared::inspect(a, b).is_err());
    }
    assert!(Prepared::inspect(Path::new("relative"), &framework).is_err());
    let alias = base.join("alias");
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    assert!(Prepared::inspect(&alias, &framework).is_err());
    std::fs::remove_dir(root.join("temp")).unwrap();
    assert!(Prepared::inspect(&root, &framework).is_err());
    std::os::unix::fs::symlink(root.join("logs"), root.join("temp")).unwrap();
    assert!(Prepared::inspect(&root, &framework).is_err());
}

fn adapter_parent(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let parent = root.join("adapter");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    parent
}

#[test]
fn adapter_bundle_publishes_as_one_complete_generation_without_overwriting() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
    let parent = adapter_parent(&root);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let calls = std::cell::Cell::new(0);
    let main = plan
        .publish_adapter(|| {
            calls.set(calls.get() + 1);
            Ok(())
        })
        .unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(main, plan.verify_adapter().unwrap());
    let dir = main.parent().unwrap();
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 4);
    assert_eq!(
        std::fs::read(dir.join("friend_requests.cjs")).unwrap(),
        include_bytes!("../src/napcat/runtime/friend_requests.cjs").to_vec()
    );
    assert_eq!(
        std::fs::metadata(dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for file in std::fs::read_dir(dir).unwrap() {
        assert_eq!(
            file.unwrap().metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(plan.publish_adapter(|| Ok(())).is_err());
    assert_eq!(main, plan.verify_adapter().unwrap());
    assert_eq!(
        std::fs::read_to_string(&main).unwrap(),
        plan.main_source().unwrap()
    );
    // The failed duplicate can leave only a clearly nonfinal staging directory.
    assert!(std::fs::read_dir(parent).unwrap().all(|entry| {
        let entry = entry.unwrap();
        entry.path() == dir
            || entry
                .file_name()
                .to_string_lossy()
                .starts_with(".qq-cli-bundle-")
    }));
}

#[test]
fn bundle_verification_rejects_modified_extra_linked_or_public_files() {
    use std::os::unix::fs::PermissionsExt;
    for variant in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
        adapter_parent(&root);
        let plan = Prepared::inspect(&root, &framework).unwrap();
        let main = plan.publish_adapter(|| Ok(())).unwrap();
        let dir = main.parent().unwrap();
        match variant {
            0 => {
                let mut bytes = std::fs::read(&main).unwrap();
                bytes[0] ^= 1;
                std::fs::write(&main, bytes).unwrap();
            }
            1 => std::fs::write(dir.join("package.json"), "{}").unwrap(),
            2 => {
                std::fs::rename(&main, root.join("saved")).unwrap();
                std::os::unix::fs::symlink(root.join("saved"), &main).unwrap();
            }
            3 => std::fs::hard_link(&main, root.join("linked")).unwrap(),
            _ => std::fs::set_permissions(&main, std::fs::Permissions::from_mode(0o644)).unwrap(),
        }
        assert!(plan.verify_adapter().is_err());
    }
}

#[test]
fn lost_authority_never_returns_a_successful_publication() {
    use qq_cli::core::error::AppError;
    for fail_at in 1..=3 {
        let temp = tempfile::tempdir().unwrap();
        let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
        let parent = adapter_parent(&root);
        let plan = Prepared::inspect(&root, &framework).unwrap();
        let calls = std::cell::Cell::new(0);
        assert!(
            plan.publish_adapter(|| {
                calls.set(calls.get() + 1);
                if calls.get() == fail_at {
                    Err(AppError::storage())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        if fail_at < 3 {
            assert!(std::fs::read_dir(parent).unwrap().all(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".qq-cli-bundle-")
            }));
            assert!(plan.verify_adapter().is_err());
        } else {
            // Failure after rename is not rolled back or reported as safe to replay.
            assert!(plan.verify_adapter().is_ok());
        }
    }
}

#[test]
fn parent_replacement_during_publication_is_rejected_before_rename() {
    let temp = tempfile::tempdir().unwrap();
    let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
    let parent = adapter_parent(&root);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let calls = std::cell::Cell::new(0);
    assert!(
        plan.publish_adapter(|| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                std::fs::rename(&parent, root.join("original-adapter")).unwrap();
                adapter_parent(&root);
            }
            Ok(())
        })
        .is_err()
    );
    assert_eq!(std::fs::read_dir(parent).unwrap().count(), 0);
    assert!(plan.verify_adapter().is_err());
}

fn synthetic_package() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "name":"qq-chat", "platform":"darwin", "version":"synthetic-version",
        "buildVersion":"synthetic-build", "main":"./application.asar/app_launcher/index.js",
        "eleArch":"x64", "unknown":{"keep":[1,true,"original"]}
    }))
    .unwrap()
}
fn package_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn deployment_bytes_are_planned_before_publication_and_rechecked_afterwards() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let (root, framework) = fixture(&base);
    let prepared = Prepared::inspect(&root, &framework).unwrap();
    let adapter = prepared.adapter_plan().unwrap();
    let source = synthetic_package();
    let edit = prepared
        .plan_package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build",
        )
        .unwrap();
    let expected: serde_json::Value = serde_json::from_slice(edit.bytes()).unwrap();
    assert_eq!(
        expected["main"],
        serde_json::json!(
            Path::new("../../../..").join(adapter.entry().strip_prefix(&root).unwrap())
        )
    );
    assert_eq!(adapter.digest(), prepared.adapter_plan().unwrap().digest());
    assert!(!root.join("adapter").exists());
    assert!(!root.join("QQ.app").exists());
    assert!(adapter.verify().is_err());
    assert!(
        prepared
            .package_edit(
                &source,
                &package_digest(&source),
                "synthetic-version",
                "synthetic-build",
            )
            .is_err()
    );

    let other_framework = base.join("other-framework");
    std::fs::create_dir(&other_framework).unwrap();
    let other = Prepared::inspect(&root, &other_framework).unwrap();
    assert_ne!(adapter.digest(), other.adapter_plan().unwrap().digest());
    let other_edit = other
        .plan_package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build",
        )
        .unwrap();
    assert_ne!(edit.result_digest(), other_edit.result_digest());

    adapter_parent(&root);
    assert_eq!(adapter.publish(|| Ok(())).unwrap(), adapter.entry());
    assert_eq!(prepared.verify_adapter().unwrap(), adapter.entry());
    let published = prepared
        .package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build",
        )
        .unwrap();
    assert_eq!(published.bytes(), edit.bytes());
    assert!(other.verify_adapter().is_err());
    std::fs::write(adapter.entry(), "changed after planning").unwrap();
    assert!(adapter.verify().is_err());
    assert!(prepared.verify_adapter().is_err());
    // A pure plan never upgrades mutated publication into verified bytes.
    assert_eq!(
        prepared
            .plan_package_edit(
                &source,
                &package_digest(&source),
                "synthetic-version",
                "synthetic-build",
            )
            .unwrap()
            .bytes(),
        edit.bytes()
    );
    assert!(
        prepared
            .package_edit(
                &source,
                &package_digest(&source),
                "synthetic-version",
                "synthetic-build",
            )
            .is_err()
    );
}

#[test]
fn package_entry_resolves_with_electron_join_from_the_app_directory() {
    let temp = tempfile::tempdir().unwrap();
    let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
    adapter_parent(&root);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let entry = plan.publish_adapter(|| Ok(())).unwrap();
    let source = synthetic_package();
    let edit = plan
        .package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build",
        )
        .unwrap();
    let package: serde_json::Value = serde_json::from_slice(edit.bytes()).unwrap();
    let app = edit.destination().parent().unwrap();
    std::fs::create_dir_all(app).unwrap();
    let input = serde_json::json!({"app": app, "main": package["main"], "entry": entry});
    // Electron joins appPath and package.main before asking Node to resolve it.
    // Node's join keeps an absolute-looking second path under appPath, unlike
    // resolve. Exercise that behavior with the actual published entry file.
    let mut child = Command::new("node")
        .args([
            "-e",
            "const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');const x=JSON.parse(fs.readFileSync(0,'utf8'));assert.equal(require.resolve(path.join(x.app,x.main)),x.entry);",
        ])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&input).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn package_edit_changes_only_main_and_binds_source_and_result() {
    let temp = tempfile::tempdir().unwrap();
    let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
    adapter_parent(&root);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    let main = plan.publish_adapter(|| Ok(())).unwrap();
    let source = synthetic_package();
    let edit = plan
        .package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build",
        )
        .unwrap();
    let mut expected: serde_json::Value = serde_json::from_slice(&source).unwrap();
    expected["main"] =
        serde_json::json!(Path::new("../../../..").join(main.strip_prefix(&root).unwrap()));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(edit.bytes()).unwrap(),
        expected
    );
    assert_eq!(edit.source_digest(), package_digest(&source));
    assert_eq!(edit.result_digest(), package_digest(edit.bytes()));
    assert_eq!(
        edit.destination(),
        root.join("QQ.app/Contents/Resources/app/package.json")
    );
    edit.verify_source(&source).unwrap();
    edit.verify_result(edit.bytes()).unwrap();
    assert!(edit.verify_source(edit.bytes()).is_err());
    assert!(edit.verify_result(&source).is_err());
    assert!(!edit.destination().exists());
}

#[test]
fn package_edit_rejects_source_changes_version_changes_and_already_patched_main() {
    let temp = tempfile::tempdir().unwrap();
    let (root, framework) = fixture(&temp.path().canonicalize().unwrap());
    adapter_parent(&root);
    let plan = Prepared::inspect(&root, &framework).unwrap();
    plan.publish_adapter(|| Ok(())).unwrap();
    let source = synthetic_package();
    assert!(
        plan.package_edit(
            &source,
            &"0".repeat(64),
            "synthetic-version",
            "synthetic-build"
        )
        .is_err()
    );
    assert!(
        plan.package_edit(
            &source,
            &package_digest(&source),
            "other",
            "synthetic-build"
        )
        .is_err()
    );
    for (key, value) in [
        ("name", "other"),
        ("platform", "win32"),
        ("main", "/other/main.cjs"),
        ("type", "module"),
        ("buildVersion", "other"),
    ] {
        let mut changed: serde_json::Value = serde_json::from_slice(&source).unwrap();
        changed[key] = serde_json::json!(value);
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(
            plan.package_edit(
                &bytes,
                &package_digest(&bytes),
                "synthetic-version",
                "synthetic-build"
            )
            .is_err()
        );
    }
    let duplicate = String::from_utf8(source.clone())
        .unwrap()
        .replacen("{", "{\"main\":\"untrusted\",", 1)
        .into_bytes();
    assert!(
        plan.package_edit(
            &duplicate,
            &package_digest(&duplicate),
            "synthetic-version",
            "synthetic-build"
        )
        .is_err()
    );
    let main = plan.verify_adapter().unwrap();
    std::fs::write(main, "tampered").unwrap();
    assert!(
        plan.package_edit(
            &source,
            &package_digest(&source),
            "synthetic-version",
            "synthetic-build"
        )
        .is_err()
    );
}

#[test]
fn package_deployment_requires_live_hardened_copy_and_rechecks_effect_boundary() {
    use qq_cli::core::error::AppError;
    use qq_cli::napcat::install::macos::bundle::Inventory;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::time::{Duration, Instant};
    for variant in 0..13 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let (root, framework) = fixture(&base);
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        adapter_parent(&root);
        let plan = Prepared::inspect(&root, &framework).unwrap();
        let main = if variant >= 11 {
            plan.adapter_plan().unwrap().entry().to_owned()
        } else {
            plan.publish_adapter(|| Ok(())).unwrap()
        };
        let mut source = synthetic_package();
        source.extend_from_slice(&[b' '; 4096]);
        let edit = plan
            .plan_package_edit(
                &source,
                &package_digest(&source),
                "synthetic-version",
                "synthetic-build",
            )
            .unwrap();
        if variant == 12 {
            assert_eq!(plan.publish_adapter(|| Ok(())).unwrap(), main);
        }
        let original = base.join("Original.app");
        let package = "Contents/Resources/app/package.json";
        std::fs::create_dir_all(original.join("Contents/Resources/app")).unwrap();
        std::fs::write(original.join(package), &source).unwrap();
        std::fs::write(original.join("untouched"), b"original").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let original = Inventory::inspect(&original, deadline).unwrap();
        let copy = original
            .copy_to_new(&root, "QQ.app", deadline, || Ok(()))
            .unwrap();
        let copy = if variant == 1 {
            copy
        } else {
            copy.harden_copy(deadline, || Ok(())).unwrap()
        };
        let copy = if variant == 2 {
            Inventory::inspect(&root.join("QQ.app"), deadline).unwrap()
        } else {
            copy
        };
        let before = std::fs::metadata(edit.destination()).unwrap();
        if variant == 3 {
            std::fs::write(&main, "changed").unwrap();
        }
        let calls = std::cell::Cell::new(0);
        let result = plan.deploy_package(
            copy,
            &edit,
            if variant == 8 {
                Instant::now()
            } else {
                deadline
            },
            || {
                calls.set(calls.get() + 1);
                if (variant == 4 && calls.get() == 2)
                    || (variant == 5 && calls.get() == 4)
                    || (variant == 10 && calls.get() == 3)
                {
                    return Err(AppError::storage());
                }
                if variant == 6 && calls.get() == 2 {
                    std::fs::rename(edit.destination(), root.join("saved-package")).unwrap();
                    std::fs::write(edit.destination(), &source).unwrap();
                }
                if variant == 9 && calls.get() == 4 {
                    std::fs::write(&main, "changed after write").unwrap();
                }
                if variant == 7 && calls.get() == 4 {
                    std::fs::write(root.join("QQ.app/untouched"), "changed").unwrap();
                }
                Ok(())
            },
        );
        original.revalidate(deadline).unwrap();
        if variant == 0 || variant == 12 {
            let result = result.unwrap();
            result.revalidate(deadline).unwrap();
            edit.verify_result(&std::fs::read(edit.destination()).unwrap())
                .unwrap();
            let after = std::fs::metadata(edit.destination()).unwrap();
            assert_eq!(before.ino(), after.ino());
            assert_eq!(after.permissions().mode() & 0o777, 0o600);
            // A changed package is not replayable using the original plan.
            assert!(
                plan.deploy_package(result, &edit, deadline, || Ok(()))
                    .is_err()
            );
        } else {
            assert!(result.is_err(), "variant {variant}");
            let actual = std::fs::read(edit.destination()).unwrap();
            if variant == 5 || variant == 7 || variant == 9 {
                assert_eq!(actual, edit.bytes());
            } else {
                assert_eq!(actual, source);
            }
        }
    }
}
