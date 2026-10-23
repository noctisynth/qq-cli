use super::*;
use std::time::Duration;
#[test]
fn concurrent_preparations_cannot_both_consume_one_remaining_slot() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"12345678").unwrap();
    let state = temp.path().join("state");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let state = state.clone();
            let source = source.clone();
            std::thread::spawn(move || {
                barrier.wait();
                prepare_with_quota(
                    &source,
                    Kind::File,
                    &Store::new(state),
                    crate::messaging::attachments::cache::MARKER_RESERVATION + 12,
                    Instant::now() + Duration::from_secs(5),
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "result codes: {:?}",
        results
            .iter()
            .map(|result| result.as_ref().err().map(|error| &error.code))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.code == "CACHE_QUOTA_EXCEEDED"))
            .count(),
        1,
        "result codes: {:?}",
        results
            .iter()
            .map(|result| result.as_ref().err().map(|error| &error.code))
            .collect::<Vec<_>>()
    );
}
#[test]
fn unknown_existing_bytes_count_against_quota_without_being_deleted() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"12345678").unwrap();
    let foreign = store.root().join("attachments/foreign");
    private_dir(&foreign).unwrap();
    let file = foreign.join("unowned.bin");
    std::fs::write(&file, b"12345678").unwrap();
    assert_eq!(
        prepare_with_quota(
            &source,
            Kind::File,
            &store,
            crate::messaging::attachments::cache::MARKER_RESERVATION + 12,
            Instant::now() + Duration::from_secs(5)
        )
        .err()
        .unwrap()
        .code,
        "CACHE_QUOTA_EXCEEDED"
    );
    assert_eq!(std::fs::read(file).unwrap(), b"12345678");
    assert_eq!(
        std::fs::read_dir(store.root().join("attachments"))
            .unwrap()
            .count(),
        1
    );
}
