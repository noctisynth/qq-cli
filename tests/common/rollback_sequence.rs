//! Original 37 -> 38 -> 37 stand-ins; no QQ, upstream DLL or JavaScript is executed.
use super::{directory, release_original_runtime, until};
use qq_cli::{
    napcat::{
        compatibility::{EvidenceKind, Key, Verdict, Witness},
        doctor::Probe,
        install::catalog,
        runtime::{
            generation,
            instance_creation::Evidence,
            start_engine::{self, Context, Started},
            startup, stop,
            version_intent::{self, Operation},
        },
    },
    storage::credentials::Credentials,
};

pub(super) fn run<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    source: &Started,
    resume: bool,
) {
    let store = context.store;
    let refs = context.references;
    let old_release = catalog::lookup("v4.18.37").unwrap();
    let paths = context.probe.qq_paths().unwrap();
    let application = context.probe.inspect_qq(&paths[0], until()).unwrap();
    let witnesses = [Witness {
        key: Key::from_installation(context.probe.host(), &old_release, &application).unwrap(),
        verdict: Verdict::Supported,
        reference: "original rollback fixture, not real compatibility".into(),
        kind: EvidenceKind::FrameworkStartup,
    }];
    let old_context = Context {
        release: &old_release,
        witnesses: &witnesses,
        ..*context
    };
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    assert!(start_engine::rollback(context, &source.instance, until()).is_err());
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    let tx = store.transaction(until()).unwrap();
    let installation = tx
        .registry
        .installations
        .iter()
        .find(|i| i.id == source.instance.installation_id)
        .unwrap();
    let original_control = qq_cli::napcat::runtime::run_job::reopen_control(
        &tx,
        refs,
        &source.instance,
        installation,
        until(),
    )
    .unwrap();
    drop(tx);
    let selected = if resume {
        let target = start_engine::rollback_target(store, refs, &source.instance, until()).unwrap();
        let inputs = startup::inspect(
            store,
            &store.read().unwrap(),
            &old_release,
            &witnesses,
            context.probe,
            until(),
        )
        .unwrap();
        version_intent::begin(
            store,
            refs,
            &source.instance,
            &target,
            Operation::Rollback,
            until(),
        )
        .unwrap();
        stop::execute(store, refs, &source.instance, until()).unwrap();
        let stopped = store.read().unwrap().instances.remove(0);
        let next = inputs
            .reserve_version_run(
                store,
                refs,
                &stopped,
                &Evidence {
                    release: &old_release,
                    witnesses: &witnesses,
                    probe: context.probe,
                },
                until(),
            )
            .unwrap();
        drop(next.creation); // The first caller exits after target publication, before launch.
        store.read().unwrap().instances.remove(0)
    } else {
        source.instance.clone()
    };
    let rolled = start_engine::rollback(&old_context, &selected, until()).unwrap();
    assert_eq!(rolled.reused, resume);
    assert_eq!(rolled.instance.id, source.instance.id);
    assert_ne!(rolled.instance.run_id, source.instance.run_id);
    assert_ne!(
        rolled.instance.installation_id,
        source.instance.installation_id
    );
    assert!(rolled.instance.account.is_none());
    assert!(
        original_control
            .service()
            .job()
            .members(until())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        generation::rollback_installation(store, refs, &rolled.instance)
            .unwrap()
            .unwrap()
            .version,
        "v4.18.38"
    );
    assert_eq!(
        generation::maintenance_operation(store, refs, &rolled.instance).unwrap(),
        Some(Operation::Rollback)
    );
    assert!(store.read().unwrap().maintenance_intents.is_empty());
    let secret = context
        .credentials
        .get(&rolled.instance.credential_key)
        .unwrap();
    let repeated = start_engine::rollback(&old_context, &selected, until()).unwrap();
    assert!(repeated.reused);
    assert_eq!(repeated.instance, rolled.instance);
    assert_eq!(
        context
            .credentials
            .get(&rolled.instance.credential_key)
            .unwrap(),
        secret
    );
    let error = qq_cli::app::runtime::response(
        store,
        context.credentials,
        &qq_cli::storage::config::Config::default(),
        Started {
            instance: rolled.instance.clone(),
            reused: rolled.reused,
            request_id: rolled.request_id.clone(),
        },
        until(),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "AUTH_REQUIRED");
    let value = serde_json::to_value(qq_cli::core::output::Envelope::failure(error)).unwrap();
    let schema = qq_cli::cli::registry::get("napcat.rollback").unwrap();
    assert!(
        jsonschema::validator_for(&schema["responseSchema"])
            .unwrap()
            .is_valid(&value)
    );
    let (plan, _) = generation::active(store, refs, &rolled.instance, until())
        .unwrap()
        .unwrap();
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.instances[0].state = "starting".into();
    tx.commit().unwrap();
    let unknown = start_engine::rollback(&old_context, &selected, until())
        .err()
        .unwrap();
    assert_eq!(unknown.code, "START_OUTCOME_UNKNOWN");
    assert!(!unknown.retry_safe);
    assert_eq!(
        std::fs::read_dir(store.root().join("run-jobs"))
            .unwrap()
            .count(),
        3
    );
    if resume {
        let live = store.read().unwrap().instances.remove(0);
        stop::execute(store, refs, &live, until()).unwrap();
        assert!(start_engine::rollback(&old_context, &selected, until()).is_err());
        assert_eq!(
            std::fs::read_dir(store.root().join("run-jobs"))
                .unwrap()
                .count(),
            3
        );
    } else {
        release_original_runtime(&directory(store, &plan));
    }
}
