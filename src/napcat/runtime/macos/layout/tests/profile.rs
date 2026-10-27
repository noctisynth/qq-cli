use super::*;
use crate::napcat::runtime::{generation, lifecycle_lock, macos::stop};

fn next(store: &Store, refs: &References) -> String {
    let selected = store.read().unwrap().instances[0].clone();
    stop::execute(store, refs, &selected, deadline()).unwrap();
    let lifecycle = lifecycle_lock(store, deadline()).unwrap();
    let mut tx = store.transaction(deadline()).unwrap();
    let source = tx.registry.instances[0].clone();
    let installed = tx.registry.installations[0].clone();
    let next =
        generation::reserve_macos(&mut tx, refs, &source, &installed, &lifecycle, deadline())
            .unwrap();
    next.request_id
}

fn first(store: &Store, refs: &References, legacy: bool) -> Layout {
    let Preparation::Ready(layout) = prepare(store, refs, "instance", "first", deadline()).unwrap()
    else {
        panic!("first layout missing");
    };
    if legacy {
        let lifecycle = lifecycle_lock(store, deadline()).unwrap();
        let tx = store.transaction(deadline()).unwrap();
        let (target, _, _) = scope(&tx, "instance").unwrap();
        let (path, _) = paths(store, &target).unwrap();
        let mut record = load(&path, refs).unwrap().unwrap();
        record.schema_version = 1;
        record.layout_version = 2;
        assert!(record.profile.is_none());
        // Old serialization must omit the newly introduced optional member.
        assert!(
            serde_json::to_value(&record)
                .unwrap()
                .get("profile")
                .is_none()
        );
        save(&tx, &lifecycle, refs, &path, &record).unwrap();
    }
    layout
}

#[test]
fn successors_reuse_one_owned_profile_without_copying_or_chaining_it() {
    for legacy in [false, true] {
        let (_temp, store, refs) = setup();
        let first = first(&store, &refs, legacy);
        let marker = first.profile_home().join("synthetic-session");
        std::fs::write(&marker, b"owned synthetic data").unwrap();
        let original_target = scope(&store.transaction(deadline()).unwrap(), "instance")
            .unwrap()
            .0;
        let original_reference = load_bound(&paths(&store, &original_target).unwrap().0, &refs)
            .unwrap()
            .unwrap()
            .1;
        for _ in 0..2 {
            let request = next(&store, &refs);
            let Preparation::Ready(layout) =
                prepare(&store, &refs, "instance", &request, deadline()).unwrap()
            else {
                panic!("successor missing");
            };
            assert_ne!(layout.directory(), first.directory());
            assert_eq!(layout.profile_home(), first.profile_home());
            assert_eq!(std::fs::read(&marker).unwrap(), b"owned synthetic data");
            assert!(
                !layout
                    .directory()
                    .join("profile-home/synthetic-session")
                    .exists()
            );
            let tx = store.transaction(deadline()).unwrap();
            let target = scope(&tx, "instance").unwrap().0;
            let record = load(&paths(&store, &target).unwrap().0, &refs)
                .unwrap()
                .unwrap();
            assert_eq!(record.profile.as_deref(), Some(original_reference.as_str()));
            assert_eq!((record.schema_version, record.layout_version), (2, 3));
        }
    }
}

#[test]
fn legacy_record_fingerprint_is_unchanged_by_optional_profile_support() {
    let (_temp, store, refs) = setup();
    first(&store, &refs, true);
    let tx = store.transaction(deadline()).unwrap();
    let target = scope(&tx, "instance").unwrap().0;
    let record = load(&paths(&store, &target).unwrap().0, &refs)
        .unwrap()
        .unwrap();
    let original_bytes = format!(
        "{{\"schema_version\":1,\"layout_version\":2,\"request\":{},\"target\":{},\"outcome\":{}}}",
        serde_json::to_string(&record.request).unwrap(),
        serde_json::to_string(&record.target).unwrap(),
        serde_json::to_string(&record.outcome).unwrap(),
    );
    assert_eq!(
        serde_json::to_vec(&record).unwrap(),
        original_bytes.as_bytes()
    );
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-layout-v1\0");
    hash.update(original_bytes);
    let Some(Preparation::Ready(recovered)) =
        recover_in(&tx, &refs, "instance", deadline()).unwrap()
    else {
        panic!("legacy layout missing");
    };
    assert_eq!(recovered.fingerprint(), format!("{:x}", hash.finalize()));
}

#[test]
fn replaced_or_missing_owner_is_rejected_before_creating_the_successor_tree() {
    use std::os::unix::fs::PermissionsExt;
    for missing in [false, true] {
        let (_temp, store, refs) = setup();
        let original = first(&store, &refs, false);
        let marker = original.profile_home().join("synthetic-session");
        std::fs::write(&marker, b"preserve").unwrap();
        let request = next(&store, &refs);
        let preserved = original.directory().join("preserved-profile");
        std::fs::rename(original.profile_home(), &preserved).unwrap();
        if !missing {
            std::fs::create_dir(original.profile_home()).unwrap();
            std::fs::set_permissions(
                original.profile_home(),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        assert!(prepare(&store, &refs, "instance", &request, deadline()).is_err());
        let tx = store.transaction(deadline()).unwrap();
        let target = scope(&tx, "instance").unwrap().0;
        let (index, root) = paths(&store, &target).unwrap();
        assert!(!index.exists());
        assert!(!root.exists());
        assert_eq!(
            std::fs::read(preserved.join("synthetic-session")).unwrap(),
            b"preserve"
        );
    }
}

#[test]
fn authenticated_but_foreign_or_recursive_profile_declarations_are_rejected() {
    for foreign in [false, true] {
        let (_temp, store, refs) = setup();
        first(&store, &refs, false);
        let request = next(&store, &refs);
        prepare(&store, &refs, "instance", &request, deadline()).unwrap();
        if foreign {
            let mut tx = store.transaction(deadline()).unwrap();
            let mut other = tx.registry.instances[0].clone();
            other.id = "foreign-instance".into();
            other.run_id = "foreign-run".into();
            tx.registry.instances.push(other);
            tx.commit().unwrap();
            prepare(
                &store,
                &refs,
                "foreign-instance",
                "foreign-request",
                deadline(),
            )
            .unwrap();
        }
        let tx = store.transaction(deadline()).unwrap();
        let target = scope(&tx, "instance").unwrap().0;
        let (index, root) = paths(&store, &target).unwrap();
        let mut record = load(&index, &refs).unwrap().unwrap();
        let reference = if foreign {
            let other = scope(&tx, "foreign-instance").unwrap().0;
            load_bound(&paths(&store, &other).unwrap().0, &refs)
                .unwrap()
                .unwrap()
                .1
        } else {
            let mut owner = authenticate(&refs, record.profile.as_ref().unwrap()).unwrap();
            owner.profile = record.profile.clone();
            refs.issue("l2", &owner).unwrap()
        };
        record.profile = Some(reference);
        assert!(profile::restore(&root, &record, &refs, deadline()).is_err());
    }
}

#[test]
fn missing_original_stop_proof_never_opens_or_replaces_the_old_profile() {
    let (_temp, store, refs) = setup();
    let original = first(&store, &refs, false);
    let request = next(&store, &refs);
    let stops = store.root().join("macos-stops");
    let proof = std::fs::read_dir(stops)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::rename(&proof, proof.with_extension("preserved")).unwrap();
    assert!(prepare(&store, &refs, "instance", &request, deadline()).is_err());
    original.verify(deadline()).unwrap();
}
