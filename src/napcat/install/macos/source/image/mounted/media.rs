//! IORegistry object identity within one boot, not filesystem mount ownership.
use super::*;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Generation {
    boot: String,
    whole: u64,
    volume: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Selection {
    pub generation: Generation,
    pub whole: (u32, u32),
    pub volume: (u32, u32),
}
#[derive(Deserialize)]
struct Node {
    #[serde(rename = "IORegistryEntryID")]
    id: u64,
    #[serde(rename = "IOObjectClass")]
    class: String,
    #[serde(rename = "BSD Name")]
    bsd: Option<String>,
    #[serde(rename = "BSD Major")]
    major: Option<u32>,
    #[serde(rename = "BSD Minor")]
    minor: Option<u32>,
    #[serde(rename = "Whole")]
    whole: Option<bool>,
    #[serde(rename = "Writable")]
    writable: Option<bool>,
    #[serde(rename = "IORegistryEntryChildren", default)]
    children: Vec<Node>,
}

pub(super) fn observe(
    selected: &report::Selected,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Generation> {
    Ok(inspect(selected, deadline, guard)?.generation)
}
pub(super) fn inspect(
    selected: &report::Selected,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Selection> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let boot = crate::native::macos_process::boot_session()?;
    let bytes = command::media(deadline, guard)?;
    let generation = select(&bytes, selected, boot.clone())?;
    if crate::native::macos_process::boot_session()? != boot {
        return Err(invalid());
    }
    guard()?;
    crate::core::deadline::check(deadline)?;
    Ok(generation)
}

fn select(bytes: &[u8], selected: &report::Selected, boot: String) -> Result<Selection> {
    if bytes.len() > 4 * 1024 * 1024 || boot.is_empty() {
        return Err(invalid());
    }
    let roots: Vec<Node> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let whole_name = selected.whole.strip_prefix("/dev/").ok_or_else(invalid)?;
    let volume_name = selected.device.strip_prefix("/dev/").ok_or_else(invalid)?;
    let mut whole = None;
    let mut volume = None;
    let mut whole_device = None;
    let mut volume_device = None;
    let mut budget = 4096usize;
    let mut pending: Vec<_> = roots.iter().map(|node| (node, None, 0usize)).collect();
    while let Some((node, parent_whole, depth)) = pending.pop() {
        budget = budget.checked_sub(1).ok_or_else(invalid)?;
        if depth > 64 || node.id == 0 {
            return Err(invalid());
        }
        let mut ancestor = parent_whole;
        if node.bsd.as_deref() == Some(whole_name) {
            if whole.is_some()
                || node.class != "IOMedia"
                || node.whole != Some(true)
                || node.writable != Some(false)
            {
                return Err(invalid());
            }
            whole_device = Some((
                node.major.ok_or_else(invalid)?,
                node.minor.ok_or_else(invalid)?,
            ));
            whole = Some(node.id);
            ancestor = Some(node.id);
        }
        if node.bsd.as_deref() == Some(volume_name) {
            if volume.is_some()
                || node.class != "IOMedia"
                || node.writable != Some(false)
                || node.whole != Some(whole_name == volume_name)
            {
                return Err(invalid());
            }
            volume_device = Some((
                node.major.ok_or_else(invalid)?,
                node.minor.ok_or_else(invalid)?,
            ));
            volume = Some((node.id, ancestor.ok_or_else(invalid)?));
        }
        pending.extend(node.children.iter().map(|node| (node, ancestor, depth + 1)));
    }
    let whole = whole.ok_or_else(invalid)?;
    let (volume, parent) = volume.ok_or_else(invalid)?;
    if parent != whole || (whole_name != volume_name && volume == whole) {
        return Err(invalid());
    }
    let whole_device = whole_device.ok_or_else(invalid)?;
    let volume_device = volume_device.ok_or_else(invalid)?;
    if whole_name != volume_name && whole_device == volume_device {
        return Err(invalid());
    }
    Ok(Selection {
        generation: Generation {
            boot,
            whole,
            volume,
        },
        whole: whole_device,
        volume: volume_device,
    })
}

pub(super) fn require_gone(
    generation: &Generation,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let boot = crate::native::macos_process::boot_session()?;
    if boot == generation.boot {
        let bytes = command::media(deadline, guard)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(invalid());
        }
        let roots: Vec<Node> = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        let mut pending: Vec<_> = roots.iter().map(|node| (node, 0usize)).collect();
        let mut budget = 4096usize;
        while let Some((node, depth)) = pending.pop() {
            budget = budget.checked_sub(1).ok_or_else(invalid)?;
            if depth > 64
                || node.id == 0
                || node.id == generation.whole
                || node.id == generation.volume
            {
                return Err(invalid());
            }
            pending.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
    }
    if crate::native::macos_process::boot_session()? != boot {
        return Err(invalid());
    }
    guard()?;
    crate::core::deadline::check(deadline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn generation_requires_unique_read_only_media_ancestry_and_boot_binding() {
        let selected = report::select(
            &serde_json::to_vec(&json!({"images":[{
                "image-path":"/synthetic.dmg", "owner-uid":rustix::process::geteuid().as_raw(),
                "writeable":false,"image-encrypted":false,"system-entities":[
                    {"dev-entry":"/dev/disk42"},
                    {"dev-entry":"/dev/disk42s1","mount-point":"/synthetic-mount"}
                ]
            }]}))
            .unwrap(),
            Path::new("/synthetic.dmg"),
            Path::new("/synthetic-mount"),
        )
        .unwrap();
        let valid = json!([{"IORegistryEntryID":100,"IOObjectClass":"IOMedia","BSD Name":"disk42","BSD Major":1,"BSD Minor":42,
        "Whole":true,"Writable":false,"IORegistryEntryChildren":[{
            "IORegistryEntryID":101,"IOObjectClass":"IOPartitionScheme","IORegistryEntryChildren":[{
                "IORegistryEntryID":102,"IOObjectClass":"IOMedia","BSD Name":"disk42s1","BSD Major":1,"BSD Minor":43,"Whole":false,"Writable":false
            }]
        }]}]);
        let check = |v: &serde_json::Value, boot: &str| {
            select(&serde_json::to_vec(v).unwrap(), &selected, boot.into())
        };
        let original = check(&valid, "synthetic-boot-a").unwrap();
        assert_ne!(original, check(&valid, "synthetic-boot-b").unwrap());
        for change in 0..10 {
            let mut changed = valid.clone();
            match change {
                0 => changed[0]["Writable"] = json!(true),
                1 => changed[0]["Whole"] = json!(false),
                2 => changed[0]["IOObjectClass"] = json!("unrelated"),
                3 => changed[0]["IORegistryEntryID"] = json!(0),
                4 => changed.as_array_mut().unwrap().push(valid[0].clone()),
                5 => {
                    let child = changed[0]["IORegistryEntryChildren"].take();
                    changed[0]["IORegistryEntryChildren"] = json!([]);
                    changed
                        .as_array_mut()
                        .unwrap()
                        .extend(child.as_array().unwrap().clone());
                }
                6 => {
                    changed[0]["IORegistryEntryChildren"][0]["IORegistryEntryChildren"][0]["Writable"] =
                        json!(true)
                }
                7 => {
                    changed[0]["IORegistryEntryChildren"][0]["IORegistryEntryChildren"][0]["IORegistryEntryID"] =
                        json!(100)
                }
                8 => {
                    changed[0].as_object_mut().unwrap().remove("BSD Major");
                }
                _ => {
                    changed[0]["IORegistryEntryChildren"][0]["IORegistryEntryChildren"][0]["BSD Minor"] =
                        json!(42);
                }
            }
            assert!(
                check(&changed, "synthetic-boot-a").is_err(),
                "change {change}"
            );
        }
        let mut changed = valid.clone();
        changed[0]["IORegistryEntryChildren"][0]["IORegistryEntryChildren"][0]["IORegistryEntryID"] =
            json!(200);
        assert_ne!(original, check(&changed, "synthetic-boot-a").unwrap());
        assert!(check(&valid, "").is_err());
        let mut oversized = valid.clone();
        oversized
            .as_array_mut()
            .unwrap()
            .extend(std::iter::repeat_n(
                json!({"IORegistryEntryID":300,"IOObjectClass":"unrelated"}),
                4096,
            ));
        assert!(check(&oversized, "synthetic-boot-a").is_err());
        let mut bytes = serde_json::to_vec(&valid).unwrap();
        bytes.resize(4 * 1024 * 1024 + 1, b' ');
        assert!(select(&bytes, &selected, "synthetic-boot-a".into()).is_err());
    }
}
