use super::*;
use crate::napcat::runtime::{instance_creation::Creation, lifecycle_lock, macos::layout};
use crate::storage::state::Installation;
use std::time::Duration;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn only_authenticated_stopped_ancestors_are_exposed_and_missing_proofs_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [71; 32]);
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations.push(Installation {
        id: "install".into(),
        version: "synthetic".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("framework"),
    });
    tx.registry.instances.push(Instance {
        id: "instance".into(),
        run_id: "first".into(),
        installation_id: "install".into(),
        endpoint: "http://127.0.0.1:24001".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        state: "prepared".into(),
        account: None,
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let mut expected = BTreeSet::new();
    let mut request = "first".to_owned();
    for step in 0..2 {
        let selected = store.read().unwrap().instances[0].clone();
        layout::prepare(&store, &refs, &selected.id, &request, deadline()).unwrap();
        stop::execute(&store, &refs, &selected, deadline()).unwrap();
        expected.insert(runtime_store::identity(&selected.id, &selected.run_id).unwrap());
        let lease = lifecycle_lock(&store, deadline()).unwrap();
        let mut tx = store.transaction(deadline()).unwrap();
        let source = tx.registry.instances[0].clone();
        let installation = tx.registry.installations[0].clone();
        let next =
            generation::reserve_macos(&mut tx, &refs, &source, &installation, &lease, deadline())
                .unwrap();
        let Creation::Fresh(reservation) = next.creation else {
            panic!("fresh successor required")
        };
        let plan = reservation.plan().clone();
        drop(tx);
        drop(lease);
        let before = std::fs::read(store.root().join("registry.json")).unwrap();
        assert_eq!(
            predecessors(&store, &refs, &plan, deadline()).unwrap(),
            expected.iter().cloned().collect::<Vec<_>>()
        );
        assert_eq!(
            std::fs::read(store.root().join("registry.json")).unwrap(),
            before
        );
        let wrong = References::with_key(store.root().join("references"), [72; 32]);
        assert!(predecessors(&store, &wrong, &plan, deadline()).is_err());
        let mut stale = plan.clone();
        stale.run_id = "unrelated".into();
        assert!(predecessors(&store, &refs, &stale, deadline()).is_err());
        request = next.request_id;
        if step == 1 {
            let proof = std::fs::read_dir(store.root().join("macos-stops"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let preserved = proof.with_extension("preserved");
            std::fs::rename(&proof, &preserved).unwrap();
            assert!(predecessors(&store, &refs, &plan, deadline()).is_err());
            std::fs::rename(preserved, proof).unwrap();
        }
    }
}
