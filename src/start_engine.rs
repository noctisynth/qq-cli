//! Explicit new-instance orchestration. Reviewed trust gates still precede all dispatch.
#![cfg(windows)]
use crate::{
    catalog::Release,
    compatibility::Witness,
    credentials::Credentials,
    doctor::Probe,
    error::{AppError, Result},
    instance_creation::{Creation, Evidence},
    references::References,
    runtime_store,
    start_intent::{self, Begin, Outcome},
    state::{Instance, Store, private_dir, reject_links},
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::time::{Duration, Instant};

pub struct Context<'a, C: Credentials, P: Probe> {
    pub store: &'a Store,
    pub references: &'a References,
    pub credentials: &'a C,
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
    pub probe: &'a P,
    pub environment: &'a BTreeMap<OsString, OsString>,
}
pub struct Started {
    pub instance: Instance,
    pub reused: bool,
    pub request_id: Option<String>,
}

/// A retained prelogin process is a checkpoint, never evidence of an online account.
pub fn response(
    store: &Store,
    credentials: &impl Credentials,
    config: &crate::config::Config,
    started: Started,
    deadline: Instant,
) -> Result<crate::output::Envelope> {
    if started.instance.state == "login_required" {
        let mut error = AppError::new(
            "AUTH_REQUIRED",
            "The instance is waiting for login in its QQ window; inspect status after authentication.",
            4,
        ).with_details(serde_json::json!({"instanceId":started.instance.id,"state":"login_required","account":null,"reused":started.reused}));
        error.request_id = started.request_id;
        error.next_action = Some(Box::new(
            serde_json::json!({"command":"status","args":{"instance":started.instance.id}}),
        ));
        return Err(error);
    }
    let registry = store.read()?;
    let current = crate::state::select(&registry, Some(&started.instance.id))?;
    if current != started.instance {
        return Err(unknown_instance(&started.instance.id, started.request_id));
    }
    let installation = registry
        .installations
        .iter()
        .find(|i| i.id == current.installation_id)
        .ok_or_else(AppError::storage)?;
    let transport =
        crate::transport::Context::load(store, &registry, credentials, &current, deadline)?;
    let token = credentials.get(&current.credential_key)?;
    let backend = transport.client(&current, &token, config)?;
    let info = crate::session::Session {
        backend: &backend,
        config,
        instance: &current,
        expected_version: &installation.version,
    }
    .verify(deadline)?;
    crate::session::remember_identity(store, &current, &info.account, deadline)?;
    let mut data = crate::session::status_data(&info);
    data["instanceId"] = serde_json::json!(current.id);
    data["reused"] = serde_json::json!(started.reused);
    Ok(
        crate::output::Envelope::success(data).scoped(crate::domain::Scope {
            instance_id: current.id,
            account: info.account,
        }),
    )
}

fn operation_lock(store: &Store, deadline: Instant) -> Result<File> {
    crate::deadline::check(deadline)?;
    private_dir(store.root())?;
    let path = store.root().join("startup.lock");
    reject_links(&path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|_| AppError::storage())?;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(TryLockError::WouldBlock) => {
                return Err(AppError::new(
                    "START_BUSY",
                    "Another startup operation is still active; no new preparation or dispatch occurred.",
                    6,
                ).safe_to_retry());
            }
            Err(_) => return Err(AppError::storage()),
        }
    }
}
pub fn explicit<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    request_id: &str,
    deadline: Instant,
) -> Result<Started> {
    context.release.require_authorization()?;
    crate::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
    let _operation = operation_lock(context.store, deadline)?;
    explicit_locked(context, request_id, deadline)
}

/// Recheck selection after acquiring the same operation lock used by explicit creation.
pub fn automatic<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    deadline: Instant,
) -> Result<Started> {
    context.release.require_authorization()?;
    let _operation = operation_lock(context.store, deadline)?;
    crate::startup::inspect(
        context.store,
        &context.store.read()?,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    match crate::default_start::select(
        context.store,
        context.references,
        context.release.version,
        deadline,
    )? {
        crate::default_start::Selection::Create { request_id } => {
            explicit_locked(context, &request_id, deadline)
        }
        crate::default_start::Selection::Existing(instance) => match instance.state.as_str() {
            "ready" | "login_required" => Ok(Started {
                instance,
                reused: true,
                request_id: None,
            }),
            "starting" | "unknown" => Err(unknown_instance(&instance.id, None)),
            _ => Err(crate::session::require_api_ready(&instance)
                .err()
                .unwrap_or_else(AppError::storage)),
        },
    }
}

fn explicit_locked<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    request_id: &str,
    deadline: Instant,
) -> Result<Started> {
    let inputs = crate::startup::inspect(
        context.store,
        &context.store.read()?,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    let evidence = Evidence {
        release: context.release,
        witnesses: context.witnesses,
        probe: context.probe,
    };
    let (plan, reused) = match inputs.reserve_instance(
        context.store,
        context.references,
        request_id,
        &evidence,
        deadline,
    )? {
        Creation::Fresh(reservation) => {
            let plan = reservation.plan().clone();
            drop(reservation);
            (plan, false)
        }
        Creation::Existing { instance, plan } => {
            match instance.state.as_str() {
                "login_required" | "ready" => {
                    return Ok(Started {
                        instance,
                        reused: true,
                        request_id: Some(request_id.into()),
                    });
                }
                "preparing" | "prepared" => (),
                "starting" | "unknown" => return Err(unknown(request_id, &plan.instance_id)),
                _ => {
                    return Err(AppError::new(
                        "INSTANCE_NOT_READY",
                        "This creation key belongs to an inactive instance; no replacement or restart was attempted.",
                        4,
                    ));
                }
            }
            (plan, true)
        }
    };
    match start_intent::get(context.store, context.references, request_id) {
        Ok(_) => return Err(unknown(request_id, &plan.instance_id)),
        Err(error) if error.code == "REQUEST_NOT_FOUND" => (),
        Err(error) => return Err(error),
    }
    runtime_store::prepare(
        context.store,
        context.references,
        context.credentials,
        &plan,
        deadline,
    )?;
    let request = crate::dispatch::Request {
        inputs: &inputs,
        release: context.release,
        witnesses: context.witnesses,
        probe: context.probe,
        references: context.references,
        credentials: context.credentials,
        runtime: &plan,
        inherited_environment: context.environment,
    };
    let tx = context.store.transaction(deadline)?;
    let target = crate::dispatch::check_in(&tx, &request, deadline)?
        .target()
        .clone();
    drop(tx);
    let Begin::Fresh(intent) = start_intent::begin(
        context.store,
        context.references,
        request_id,
        &target,
        deadline,
    )?
    else {
        return Err(unknown(request_id, &plan.instance_id));
    };
    let mut running = None;
    let receipt = intent
        .dispatch(|tx| {
            running = Some(crate::dispatch::launch_in(
                tx, &request, request_id, &target, deadline,
            )?);
            Ok(())
        })
        .map_err(|_| unknown(request_id, &plan.instance_id))?;
    if receipt.outcome != Outcome::Dispatched {
        return Err(unknown(request_id, &plan.instance_id));
    }
    let running = running.ok_or_else(|| unknown(request_id, &plan.instance_id))?;
    let mut retained = false;
    for _ in 0..1200 {
        crate::deadline::check(deadline).map_err(|_| unknown(request_id, &plan.instance_id))?;
        let tx = context
            .store
            .transaction(deadline)
            .map_err(|_| unknown(request_id, &plan.instance_id))?;
        let attempt = crate::run_job::retain_prelogin(
            &tx,
            context.references,
            &target,
            running.job(),
            deadline,
        );
        drop(tx);
        match attempt {
            Ok(_) => {
                retained = true;
                break;
            }
            Err(error) if error.code == "SERVICE_NOT_READY" => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                std::thread::sleep(remaining.min(Duration::from_millis(250)));
            }
            Err(_) => return Err(unknown(request_id, &plan.instance_id)),
        }
    }
    if !retained {
        return Err(unknown(request_id, &plan.instance_id));
    }
    let mut tx = context
        .store
        .transaction(deadline)
        .map_err(|_| unknown(request_id, &plan.instance_id))?;
    let instance = tx
        .registry
        .instances
        .iter_mut()
        .find(|i| {
            i.id == plan.instance_id
                && i.run_id == plan.run_id
                && i.installation_id == target.installation_id
        })
        .ok_or_else(|| unknown(request_id, &plan.instance_id))?;
    if instance.state == "starting" {
        instance.state = "login_required".into();
    } else if !matches!(instance.state.as_str(), "login_required" | "ready") {
        return Err(unknown(request_id, &plan.instance_id));
    }
    let instance = instance.clone();
    tx.commit()
        .map_err(|_| unknown(request_id, &plan.instance_id))?;
    Ok(Started {
        instance,
        reused,
        request_id: Some(request_id.into()),
    })
}
fn unknown(request_id: &str, instance_id: &str) -> AppError {
    unknown_instance(instance_id, Some(request_id.into()))
}
fn unknown_instance(instance_id: &str, request_id: Option<String>) -> AppError {
    let mut error = AppError::new(
        "START_OUTCOME_UNKNOWN",
        "Startup may have been dispatched; inspect this instance and do not repeat creation with another key.",
        5,
    );
    error.outcome = Some("unknown");
    error.request_id = request_id;
    error.next_action = Some(Box::new(
        serde_json::json!({"command":"status","args":{"instance":instance_id}}),
    ));
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_lock_serializes_creation_without_blocking_state_observation() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let end = Instant::now() + Duration::from_secs(3);
        let held = operation_lock(&store, end).unwrap();
        assert!(store.read().unwrap().instances.is_empty());
        let tx = store.transaction(end).unwrap();
        drop(tx);
        let error = operation_lock(&store, Instant::now() + Duration::from_millis(25))
            .err()
            .unwrap();
        assert_eq!(error.code, "START_BUSY");
        assert!(error.retry_safe);
        drop(held);
        operation_lock(&store, end).unwrap();
    }
}
