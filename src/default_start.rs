//! Stable internal creation key for automatic startup. Never selects an account.
//! Selection runs under the outer startup operation lock, then the state lock.
use crate::{
    error::{AppError, Result},
    references::References,
    state::{Instance, Store, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    version: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub enum Selection {
    Existing(Instance),
    Create { request_id: String },
}

pub fn select(
    store: &Store,
    refs: &References,
    version: &str,
    deadline: Instant,
) -> Result<Selection> {
    let tx = store.transaction(deadline)?;
    let selected = if tx.registry.instances.is_empty() {
        None
    } else {
        Some(crate::state::select(&tx.registry, None)?)
    };
    if let Some(instance) = &selected {
        let installation = tx
            .registry
            .installations
            .iter()
            .find(|i| i.id == instance.installation_id)
            .ok_or_else(AppError::storage)?;
        if installation.version != version {
            return Err(AppError::new(
                "INSTANCE_VERSION_CONFLICT",
                "Selected instance uses another installation version; no automatic creation occurred.",
                4,
            ));
        }
        if !matches!(instance.state.as_str(), "preparing" | "prepared") {
            return Ok(Selection::Existing(instance.clone()));
        }
    }
    let directory = store.root().join("automatic-start");
    let path = directory.join("active.json");
    reject_links(&path)?;
    let record: Record = match crate::config::read_json::<Pointer>(&path, 4096) {
        Ok(pointer) => {
            if pointer.schema_version != 1 {
                return Err(conflict());
            }
            let record: Record = refs.get("d1", &pointer.reference)?;
            if record.schema_version != 1 || record.version != version {
                return Err(conflict());
            }
            crate::domain::request_key(&record.request_id).map_err(|_| conflict())?;
            record
        }
        Err(error) if error.code == "FILE_NOT_FOUND" && selected.is_none() => {
            let mut random = [0_u8; 16];
            getrandom::fill(&mut random).map_err(|_| AppError::storage())?;
            let request_id = format!(
                "automatic-{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let record = Record {
                schema_version: 1,
                request_id,
                version: version.into(),
            };
            private_dir(&directory)?;
            atomic_json(
                &path,
                &Pointer {
                    schema_version: 1,
                    reference: refs.issue("d1", &record)?,
                },
            )?;
            record
        }
        Err(error) if error.code == "FILE_NOT_FOUND" => return Err(conflict()),
        Err(error) => return Err(error),
    };
    if let Some(instance) = selected {
        let plan = crate::instance_creation::reserved_plan(store, refs, &record.request_id)?
            .ok_or_else(conflict)?;
        if plan.instance_id != instance.id
            || plan.run_id != instance.run_id
            || plan.version != version
        {
            return Err(conflict());
        }
    }
    crate::deadline::check(deadline)?;
    Ok(Selection::Create {
        request_id: record.request_id,
    })
}
fn conflict() -> AppError {
    AppError::new(
        "AUTOMATIC_START_CONFLICT",
        "Automatic startup has no matching signed creation checkpoint; inspect the existing instance or original creation request.",
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn until() -> Instant {
        Instant::now() + Duration::from_secs(3)
    }
    fn key(selection: Selection) -> String {
        match selection {
            Selection::Create { request_id } => request_id,
            _ => panic!("expected creation"),
        }
    }
    #[test]
    fn stable_default_key_survives_empty_registry_and_refuses_changed_or_corrupt_checkpoints() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(store.root().join("references"), [61; 32]);
        let first = key(select(&store, &refs, "v4.18.38", until()).unwrap());
        let path = store.root().join("automatic-start/active.json");
        let original = std::fs::read(&path).unwrap();
        assert_eq!(
            key(select(&store, &refs, "v4.18.38", until()).unwrap()),
            first
        );
        assert_eq!(
            select(&store, &refs, "v4.18.39", until())
                .err()
                .unwrap()
                .code,
            "AUTOMATIC_START_CONFLICT"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!store.root().join("instance-creations").exists());
        assert!(!store.root().join("runtimes").exists());
        std::fs::write(&path, b"original corrupted checkpoint").unwrap();
        assert!(select(&store, &refs, "v4.18.38", until()).is_err());
        assert_eq!(
            std::fs::read(path).unwrap(),
            b"original corrupted checkpoint"
        );
    }
    #[test]
    fn existing_selection_never_guesses_preparation_or_selects_among_multiple_instances() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(store.root().join("references"), [62; 32]);
        let mut tx = store.transaction(until()).unwrap();
        tx.registry.installations.push(crate::state::Installation {
            id: "original-installation".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("original-installation"),
        });
        let instance = Instance {
            id: "original-instance".into(),
            run_id: "original-run".into(),
            installation_id: "original-installation".into(),
            endpoint: "http://127.0.0.1:19001/".into(),
            ws_endpoint: None,
            credential_key: "original-credential".into(),
            account: None,
            state: "preparing".into(),
            process_id: None,
            process_started: None,
        };
        tx.registry.instances.push(instance.clone());
        tx.commit().unwrap();
        assert_eq!(
            select(&store, &refs, "v4.18.38", until())
                .err()
                .unwrap()
                .code,
            "AUTOMATIC_START_CONFLICT"
        );
        assert!(!store.root().join("automatic-start").exists());
        let mut tx = store.transaction(until()).unwrap();
        tx.registry.instances[0].state = "login_required".into();
        tx.commit().unwrap();
        assert!(matches!(
            select(&store, &refs, "v4.18.38", until()).unwrap(),
            Selection::Existing(_)
        ));
        let mut tx = store.transaction(until()).unwrap();
        let mut other = instance;
        other.id = "original-other".into();
        other.run_id = "original-other-run".into();
        tx.registry.instances.push(other);
        tx.commit().unwrap();
        assert_eq!(
            select(&store, &refs, "v4.18.38", until())
                .err()
                .unwrap()
                .code,
            "INSTANCE_AMBIGUOUS"
        );
        assert!(!store.root().join("automatic-start").exists());
    }
    #[test]
    fn concurrent_default_selection_publishes_one_stable_creation_key() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("state");
        let barrier = std::sync::Barrier::new(4);
        let keys = std::thread::scope(|scope| {
            let handles = (0..4)
                .map(|_| {
                    let root = &root;
                    let barrier = &barrier;
                    scope.spawn(move || {
                        let store = Store::new(root.clone());
                        let refs = References::with_key(store.root().join("references"), [63; 32]);
                        barrier.wait();
                        key(select(&store, &refs, "v4.18.38", until()).unwrap())
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(keys.iter().all(|k| k == &keys[0]));
        assert_eq!(
            std::fs::read_dir(root.join("references")).unwrap().count(),
            1
        );
    }
}
