use super::*;
use serde_json::{Value, json};
const RELEASE: &str = "--internal-macos-release-v1";
fn report(program: &str, args: &[&str]) -> Value {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(out.status.success());
    let mut convert = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    convert
        .stdin
        .take()
        .unwrap()
        .write_all(&out.stdout)
        .unwrap();
    let out = convert.wait_with_output().unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}
fn media<'a>(nodes: &'a Value, name: &str) -> Option<&'a Value> {
    for node in nodes.as_array()? {
        if node["BSD Name"].as_str() == Some(name) {
            return Some(node);
        }
        if let Some(found) = media(&node["IORegistryEntryChildren"], name) {
            return Some(found);
        }
    }
    None
}
fn metadata(f: &Fixture) -> Value {
    let info = report("/usr/bin/hdiutil", &["info", "-plist"]);
    let attached: Vec<_> = info["images"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["image-path"].as_str() == f.image.to_str())
        .collect();
    assert_eq!(attached.len(), 1);
    let entities = attached[0]["system-entities"].as_array().unwrap();
    let volume = entities
        .iter()
        .find(|v| v["mount-point"].as_str() == f.mount.to_str())
        .unwrap()["dev-entry"]
        .as_str()
        .unwrap();
    let whole = entities
        .iter()
        .map(|v| v["dev-entry"].as_str().unwrap())
        .min_by_key(|v| v.len())
        .unwrap();
    let registry = report("/usr/sbin/ioreg", &["-a", "-r", "-c", "IOMedia"]);
    let w = media(&registry, whole.strip_prefix("/dev/").unwrap()).unwrap();
    let v = media(&registry, volume.strip_prefix("/dev/").unwrap()).unwrap();
    let boot = Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.bootsessionuuid"])
        .output()
        .unwrap();
    assert!(boot.status.success());
    json!({"selected":{"device":volume,"whole":whole,"mount":f.mount,
        "entities":entities.iter().map(|v| v["dev-entry"].clone()).collect::<Vec<_>>()},
        "generation":{"boot":String::from_utf8(boot.stdout).unwrap().trim(),"whole":w["IORegistryEntryID"],"volume":v["IORegistryEntryID"]},
        "whole":[w["BSD Major"],w["BSD Minor"]],"volume":[v["BSD Major"],v["BSD Minor"]]})
}
fn device(metadata: &Value, key: &str) -> File {
    let path = metadata["selected"][key]
        .as_str()
        .unwrap()
        .replacen("/dev/disk", "/dev/rdisk", 1);
    File::open(path).unwrap()
}
fn release(metadata: &Value, whole: File, volume: File) -> Child {
    Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args([RELEASE, &metadata.to_string(), "30000"])
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(whole))
        .stderr(Stdio::from(volume))
        .spawn()
        .unwrap()
}
#[test]
fn release_validates_inherited_devices_and_permission_then_ejects_synthetic_mount() {
    let f = Fixture::new();
    let original = std::fs::read(&f.image).unwrap();
    let mut attach = f.spawn("30000");
    attach.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    assert_eq!(wait(&mut attach), 113);
    let valid = metadata(&f);
    for mode in 0..8 {
        let mut value = valid.clone();
        match mode {
            4 => value["generation"]["whole"] = json!(0),
            5 => value["whole"][1] = json!(u32::MAX),
            6 => value["generation"]["boot"] = json!("stale-boot"),
            7 => value["volume"] = Value::Null,
            _ => (),
        }
        let whole = if mode == 2 {
            File::open(&f.image).unwrap()
        } else if mode == 3 {
            device(&valid, "device")
        } else {
            device(&valid, "whole")
        };
        let mut child = release(&value, whole, device(&valid, "device"));
        match mode {
            0 => {
                child.stdin.take();
            }
            1 => child.stdin.as_mut().unwrap().write_all(&[1, 2]).unwrap(),
            _ => child.stdin.as_mut().unwrap().write_all(&[1]).unwrap(),
        }
        assert_eq!(
            wait(&mut child),
            114,
            "case {mode}; retain scene on failure"
        );
        assert!(
            f.mount.join("fixture.txt").exists(),
            "case {mode} changed mount"
        );
    }
    let mut child = release(&valid, device(&valid, "whole"), device(&valid, "device"));
    child.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    assert_eq!(wait(&mut child), 113, "retain fixture: release failed");
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        let info = report("/usr/bin/hdiutil", &["info", "-plist"]);
        let registry = report("/usr/sbin/ioreg", &["-a", "-r", "-c", "IOMedia"]);
        let absent = !info["images"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["image-path"].as_str() == f.image.to_str());
        let whole = valid["selected"]["whole"]
            .as_str()
            .unwrap()
            .strip_prefix("/dev/")
            .unwrap();
        if absent && media(&registry, whole).is_none() {
            break;
        }
        assert!(
            Instant::now() < end,
            "retain fixture: disappearance unconfirmed"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!f.mount.join("fixture.txt").exists());
    assert_eq!(std::fs::read(&f.image).unwrap(), original);
    std::fs::remove_dir_all(f.base).unwrap();
}
#[test]
fn malformed_release_is_silent() {
    for args in [
        vec![RELEASE],
        vec![RELEASE, "{}", "100"],
        vec![RELEASE, "null", "0"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(114));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }
}
