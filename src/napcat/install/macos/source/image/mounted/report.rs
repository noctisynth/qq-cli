//! Typed interpretation of system machine output, never a detach capability.
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct Report {
    images: Vec<Attached>,
}
#[derive(Deserialize)]
struct Attached {
    #[serde(rename = "image-path")]
    image: PathBuf,
    #[serde(rename = "owner-uid")]
    owner: u32,
    writeable: bool,
    #[serde(rename = "image-encrypted")]
    encrypted: Option<bool>,
    #[serde(rename = "system-entities")]
    entities: Vec<Entity>,
}
#[derive(Deserialize)]
struct Entity {
    #[serde(rename = "dev-entry")]
    device: String,
    #[serde(rename = "mount-point")]
    mount: Option<PathBuf>,
}
#[derive(Clone, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selected {
    pub device: String,
    pub whole: String,
    pub mount: PathBuf,
    entities: BTreeSet<String>,
}

pub(super) fn absent(bytes: &[u8], image: &Path, mount: &Path) -> Result<()> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(invalid());
    }
    let report: Report = plist::from_bytes(bytes).map_err(|_| invalid())?;
    if report.images.len() > 256
        || report.images.iter().any(|i| {
            i.image == image || i.entities.iter().any(|e| e.mount.as_deref() == Some(mount))
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn whole(device: &str) -> Result<&str> {
    let suffix = device.strip_prefix("/dev/disk").ok_or_else(invalid)?;
    if suffix.is_empty() || suffix.len() > 40 {
        return Err(invalid());
    }
    let mut pieces = suffix.split('s');
    let disk = pieces.next().ok_or_else(invalid)?;
    if std::iter::once(disk)
        .chain(pieces)
        .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }
    Ok(&device[.."/dev/disk".len() + disk.len()])
}

pub(super) fn select(bytes: &[u8], image: &Path, mount: &Path) -> Result<Selected> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(invalid());
    }
    let report: Report = plist::from_bytes(bytes).map_err(|_| invalid())?;
    if report.images.len() > 256 {
        return Err(invalid());
    }
    let mut matches = report.images.into_iter().filter(|i| i.image == image);
    let found = matches.next().ok_or_else(invalid)?;
    if matches.next().is_some()
        || found.owner != rustix::process::geteuid().as_raw()
        || found.writeable
        || found.encrypted != Some(false)
        || found.entities.is_empty()
        || found.entities.len() > 32
    {
        return Err(invalid());
    }
    let mut entities = BTreeSet::new();
    let mut mounted = None;
    let mut disk = None;
    for entity in found.entities {
        let root = whole(&entity.device)?.to_owned();
        if disk.as_ref().is_some_and(|d| d != &root) || !entities.insert(entity.device.clone()) {
            return Err(invalid());
        }
        disk = Some(root);
        if let Some(point) = entity.mount {
            if point != mount || mounted.is_some() {
                return Err(invalid());
            }
            mounted = Some(entity.device);
        }
    }
    let whole = disk.ok_or_else(invalid)?;
    if !entities.contains(&whole) {
        return Err(invalid());
    }
    Ok(Selected {
        device: mounted.ok_or_else(invalid)?,
        whole,
        mount: mount.into(),
        entities,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::encode;
    use super::*;
    use serde_json::json;
    #[test]
    fn mount_report_requires_unique_owned_read_only_image_and_exact_devices() {
        let valid = json!({"images":[{
            "image-path":"/synthetic/qq.dmg", "owner-uid":rustix::process::geteuid().as_raw(),
            "writeable":false,"image-encrypted":false,"system-entities":[
                {"dev-entry":"/dev/disk42"},
                {"dev-entry":"/dev/disk42s1","mount-point":"/synthetic/mount"}
            ]
        }]});
        let check = |value: &serde_json::Value| {
            select(
                &encode(value),
                Path::new("/synthetic/qq.dmg"),
                Path::new("/synthetic/mount"),
            )
        };
        assert!(check(&valid).is_ok());
        for change in 0..10 {
            let mut value = valid.clone();
            match change {
                0 => value["images"][0]["writeable"] = json!(true),
                1 => value["images"][0]["image-encrypted"] = json!(true),
                2 => value["images"][0]["owner-uid"] = json!(u32::MAX),
                3 => value["images"][0]["image-path"] = json!("/other.dmg"),
                4 => value["images"]
                    .as_array_mut()
                    .unwrap()
                    .push(valid["images"][0].clone()),
                5 => value["images"][0]["system-entities"][1]["dev-entry"] = json!("/dev/disk43s1"),
                6 => value["images"][0]["system-entities"][1]["mount-point"] = json!("/other"),
                7 => {
                    value["images"][0]["system-entities"][0]["mount-point"] =
                        json!("/synthetic/mount")
                }
                8 => value["images"][0]["system-entities"][0]["dev-entry"] = json!("/dev/disk42s1"),
                _ => value["images"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("writeable")
                    .map(|_| ())
                    .unwrap(),
            }
            assert!(check(&value).is_err(), "change {change}");
        }
        for name in [
            "/dev/rdisk4",
            "/dev/disk",
            "/dev/disk4s",
            "/dev/disk4s../2",
            "/dev/disk4;echo",
        ] {
            assert!(whole(name).is_err());
        }
        assert!(select(b"{}", Path::new("/a"), Path::new("/b")).is_err());
    }

    #[test]
    fn opaque_system_metadata_does_not_replace_required_mount_identity() {
        let image = Path::new("/synthetic.dmg");
        let mount = Path::new("/synthetic-mount");
        let original = encode(&json!({"images":[{
            "image-path":image,"owner-uid":rustix::process::geteuid().as_raw(),
            "writeable":false,"image-encrypted":false,"system-entities":[
                {"dev-entry":"/dev/disk42"},
                {"dev-entry":"/dev/disk42s1","mount-point":mount}
            ]
        }]}));
        let expected = select(&original, image, mount).unwrap();
        let mut value = plist::Value::from_reader_xml(original.as_slice()).unwrap();
        let attached = value
            .as_dictionary_mut()
            .unwrap()
            .get_mut("images")
            .unwrap()
            .as_array_mut()
            .unwrap()[0]
            .as_dictionary_mut()
            .unwrap();
        attached.insert(
            "image-alias".into(),
            plist::Value::Data(b"opaque bookmark".to_vec()),
        );
        attached.insert(
            "unrelated-date".into(),
            plist::Value::Date(plist::Date::from_xml_format("2026-10-28T00:00:00Z").unwrap()),
        );
        let mut bytes = Vec::new();
        plist::to_writer_xml(&mut bytes, &value).unwrap();
        assert_eq!(
            select(&bytes, image, mount).unwrap().device,
            expected.device
        );
        assert!(absent(&bytes, Path::new("/other.dmg"), Path::new("/other-mount")).is_ok());
        assert!(absent(&bytes, image, mount).is_err());
        bytes.clear();
        plist::to_writer_binary(&mut bytes, &value).unwrap();
        assert!(select(&bytes, image, mount).unwrap() == expected);
        value
            .as_dictionary_mut()
            .unwrap()
            .get_mut("images")
            .unwrap()
            .as_array_mut()
            .unwrap()[0]
            .as_dictionary_mut()
            .unwrap()
            .insert("owner-uid".into(), plist::Value::Data(vec![0]));
        bytes.clear();
        plist::to_writer_xml(&mut bytes, &value).unwrap();
        assert!(select(&bytes, image, mount).is_err());
    }

    #[test]
    fn unrelated_system_images_may_omit_encryption_without_authorizing_the_target() {
        let image = Path::new("/synthetic.dmg");
        let mount = Path::new("/synthetic-mount");
        let mut report = json!({"images":[{
            "image-path":image,"owner-uid":rustix::process::geteuid().as_raw(),
            "writeable":false,"image-encrypted":false,"system-entities":[
                {"dev-entry":"/dev/disk42"},
                {"dev-entry":"/dev/disk42s1","mount-point":mount}
            ]
        },{
            "image-path":"/synthetic-system.dmg","owner-uid":0,
            "writeable":false,"diskimages2":true,"system-entities":[
                {"dev-entry":"/dev/disk7"}
            ]
        }]});
        assert!(select(&encode(&report), image, mount).is_ok());
        assert!(
            absent(
                &encode(&report),
                Path::new("/other.dmg"),
                Path::new("/other-mount")
            )
            .is_ok()
        );
        report["images"][0]
            .as_object_mut()
            .unwrap()
            .remove("image-encrypted");
        assert!(select(&encode(&report), image, mount).is_err());
        assert!(absent(&encode(&report), image, mount).is_err());
    }
}
