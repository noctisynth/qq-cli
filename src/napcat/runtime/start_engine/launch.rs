//! Shared runtime preparation, once-only dispatch and prelogin retention.
use super::*;
pub(super) fn creation<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    inputs: &crate::napcat::runtime::startup::Inputs,
    creation: Creation,
    request_id: &str,
    deadline: Instant,
) -> Result<Started> {
    let (plan, reused) = match creation {
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
    let request = crate::napcat::runtime::dispatch::Request {
        inputs,
        release: context.release,
        witnesses: context.witnesses,
        probe: context.probe,
        references: context.references,
        credentials: context.credentials,
        runtime: &plan,
        inherited_environment: context.environment,
    };
    let tx = context.store.transaction(deadline)?;
    let target = crate::napcat::runtime::dispatch::check_in(&tx, &request, deadline)?
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
            running = Some(crate::napcat::runtime::dispatch::launch_in(
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
        crate::core::deadline::check(deadline)
            .map_err(|_| unknown(request_id, &plan.instance_id))?;
        let tx = context
            .store
            .transaction(deadline)
            .map_err(|_| unknown(request_id, &plan.instance_id))?;
        let attempt = crate::napcat::runtime::run_job::retain_prelogin(
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
