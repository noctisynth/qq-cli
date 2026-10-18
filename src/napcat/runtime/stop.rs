//! Explicit owned-instance stop. Intent precedes effects; a completion proof precedes stopped state.
mod journal;
#[cfg(test)]
mod tests;
use crate::{
    core::error::{AppError, Result},
    native::job::StopJob,
    storage::{
        references::References,
        state::{Instance, Store},
    },
};
use journal::{Outcome, Record, Target, load, save};
use std::time::{Duration, Instant};

pub struct Stopped {
    pub instance_id: String,
    pub reused: bool,
}
/// Called only under the lifecycle and state locks; registry text alone is insufficient.
pub(crate) fn require_completed(
    store: &Store,
    refs: &References,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
) -> Result<()> {
    let target = Target {
        instance_id: instance.id.clone(),
        run_id: instance.run_id.clone(),
        installation_id: installation.id.clone(),
        installation_digest: installation.digest.clone(),
    };
    if instance.state != "stopped"
        || instance.installation_id != installation.id
        || !load(store, refs, &target)?.is_some_and(|record| record.outcome == Outcome::Completed)
    {
        return Err(AppError::new(
            "STOP_PROOF_REQUIRED",
            "Unregistering requires the original completed stop proof; no instance was removed.",
            4,
        ));
    }
    Ok(())
}
pub fn execute(
    store: &Store,
    refs: &References,
    selected: &Instance,
    deadline: Instant,
) -> Result<Stopped> {
    let _operation = super::operation_lock::acquire(store, deadline)?;
    let (target, job, cached) = {
        let mut tx = store.transaction(deadline)?;
        let instance = tx
            .registry
            .instances
            .iter()
            .find(|i| i.id == selected.id)
            .ok_or_else(changed)?;
        let mut comparable = instance.clone();
        if matches!(instance.state.as_str(), "stopping" | "stopped") {
            comparable.state = selected.state.clone();
        }
        if comparable != *selected {
            return Err(changed());
        }
        let installation = tx
            .registry
            .installations
            .iter()
            .find(|i| i.id == instance.installation_id)
            .ok_or_else(changed)?;
        if tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == installation.id)
            .count()
            != 1
        {
            return Err(changed());
        }
        let target = Target {
            instance_id: instance.id.clone(),
            run_id: instance.run_id.clone(),
            installation_id: installation.id.clone(),
            installation_digest: installation.digest.clone(),
        };
        let prior = load(store, refs, &target)?;
        if let Some(record) = &prior {
            if record.target != target {
                return Err(changed());
            }
            if record.outcome == Outcome::Completed {
                if !matches!(instance.state.as_str(), "stopped" | "stopping") {
                    return Err(changed());
                }
                let instance_id = instance.id.clone();
                tx.registry
                    .instances
                    .iter_mut()
                    .find(|i| i.id == instance_id)
                    .ok_or_else(changed)?
                    .state = "stopped".into();
                tx.commit()?;
                return Ok(Stopped {
                    instance_id,
                    reused: true,
                });
            }
        }
        let recorded = super::run_job::has_record(store, &instance.id, &instance.run_id)?;
        let job_id = if recorded {
            Some(super::run_job::owned_job_id(
                store,
                refs,
                instance,
                installation,
            )?)
        } else if (matches!(instance.state.as_str(), "preparing" | "prepared")
            || (instance.state == "stopping"
                && prior.as_ref().is_some_and(|record| record.job.is_none())))
            && instance.account.is_none()
            && instance.process_id.is_none()
            && instance.process_started.is_none()
            && !super::start_intent::has_run_claim(store, &instance.id, &instance.run_id)?
        {
            None
        } else {
            return Err(AppError::new(
                "STOP_UNVERIFIABLE",
                "This instance has no provable dispatch or completion; no process was stopped.",
                4,
            ));
        };
        if prior.as_ref().is_some_and(|r| r.job != job_id) {
            return Err(changed());
        }
        let job = job_id
            .as_ref()
            .map(|id| StopJob::reopen(id, deadline))
            .transpose()
            .map_err(|error| {
                if prior.is_some() {
                    unknown(&target.instance_id)
                } else {
                    error
                }
            })?;
        let record = Record {
            schema_version: 1,
            target: target.clone(),
            job: job_id,
            outcome: Outcome::Pending,
        };
        save(store, refs, &record)?;
        tx.registry
            .instances
            .iter_mut()
            .find(|i| i.id == target.instance_id)
            .ok_or_else(changed)?
            .state = "stopping".into();
        tx.commit()?;
        (target, job, record)
    };
    let completion = (|| -> Result<()> {
        if let Some(job) = &job {
            if !job.empty(deadline)? {
                job.request(deadline)?;
            }
            for _ in 0..1200 {
                if job.empty(deadline)? {
                    return Ok(());
                }
                crate::core::deadline::check(deadline)?;
                std::thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(25)),
                );
            }
            return Err(unknown(&target.instance_id));
        }
        Ok(())
    })();
    if completion.is_err() {
        let record = Record {
            outcome: Outcome::Unknown,
            ..cached
        };
        let _ = save(store, refs, &record);
        return Err(unknown(&target.instance_id));
    }
    // Completion is durable before registry acknowledgement, while the original job handle is held.
    let complete = Record {
        outcome: Outcome::Completed,
        ..cached
    };
    save(store, refs, &complete).map_err(|_| unknown(&target.instance_id))?;
    let mut tx = store
        .transaction(deadline)
        .map_err(|_| unknown(&target.instance_id))?;
    if tx
        .registry
        .installations
        .iter()
        .filter(|installation| {
            installation.id == target.installation_id
                && installation.digest == target.installation_digest
        })
        .count()
        != 1
    {
        return Err(changed());
    }
    let instance = tx
        .registry
        .instances
        .iter_mut()
        .find(|i| i.id == target.instance_id)
        .ok_or_else(|| unknown(&target.instance_id))?;
    if instance.run_id != target.run_id
        || instance.installation_id != target.installation_id
        || instance.state != "stopping"
    {
        return Err(changed());
    }
    instance.state = "stopped".into();
    tx.commit().map_err(|_| unknown(&target.instance_id))?;
    Ok(Stopped {
        instance_id: target.instance_id,
        reused: false,
    })
}
fn changed() -> AppError {
    AppError::new(
        "STOP_INPUT_CHANGED",
        "The original instance, run, installation or stop proof changed; no replacement target was selected.",
        4,
    )
}
fn unknown(instance_id: &str) -> AppError {
    let mut error = AppError::new(
        "STOP_OUTCOME_UNKNOWN",
        "Stop may have been requested; inspect this same instance instead of choosing another process.",
        5,
    );
    error.outcome = Some("unknown");
    error.details = Some(Box::new(
        serde_json::json!({"instanceId":instance_id,"state":"stopping"}),
    ));
    error.next_action = Some(Box::new(
        serde_json::json!({"command":"doctor","args":{"instance":instance_id}}),
    ));
    error
}
