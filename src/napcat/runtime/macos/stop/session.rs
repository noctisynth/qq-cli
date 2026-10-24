//! Lifecycle composition keeps original locks and checkpoint ordering explicit.
use super::super::{
    deploy::{self, Completed},
    run,
};
use super::{
    journal::{self, Record},
    members::{self, Checkpoint, Member},
    unknown, unverifiable,
};
use crate::{
    core::error::Result,
    napcat::runtime::{lifecycle_lock, start_intent},
    native::process::{self, ExecutableSet},
    storage::{
        references::References,
        state::{Installation, Instance, Store, Transaction, durable::Lock},
    },
};
use std::time::Instant;

pub struct Stopped {
    pub instance_id: String,
    pub reused: bool,
}

pub fn execute(
    store: &Store,
    refs: &References,
    selected: &Instance,
    deadline: Instant,
) -> Result<Stopped> {
    let lifecycle = lifecycle_lock(store, deadline)?;
    locked(store, refs, selected, &lifecycle, deadline)
}
pub(in crate::napcat::runtime::macos) fn locked(
    store: &Store,
    refs: &References,
    selected: &Instance,
    lifecycle: &Lock,
    deadline: Instant,
) -> Result<Stopped> {
    let tx = store.transaction(deadline)?;
    let state_lock = tx.lock_identity()?;
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == selected.id)
        .ok_or_else(unverifiable)?
        .clone();
    if instance != *selected {
        return Err(unverifiable());
    }
    let mut installed = tx
        .registry
        .installations
        .iter()
        .filter(|i| i.id == instance.installation_id);
    let installation = installed.next().ok_or_else(unverifiable)?.clone();
    if installed.next().is_some() {
        return Err(unverifiable());
    }
    let prior = journal::load(store, refs, &instance)?;
    if let Some(record) = &prior {
        verify_binding(&tx, refs, record, false, deadline)?;
        if record.completed {
            if !matches!(instance.state.as_str(), "stopping" | "stopped") {
                return Err(unverifiable());
            }
            acknowledge(tx, lifecycle, record)?;
            return Ok(Stopped {
                instance_id: instance.id,
                reused: true,
            });
        }
    }
    let mut record = match &prior {
        Some(record) => record.clone(),
        None => {
            let origin = original(&tx, refs, &instance, &installation, deadline)?;
            let members = match &origin {
                Some(origin) => {
                    origin.scope.as_ref().ok_or_else(unverifiable)?;
                    vec![Member::root(
                        origin.signal.clone().ok_or_else(unverifiable)?,
                    )]
                }
                None => vec![],
            };
            Record {
                schema_version: 1,
                instance: instance.clone(),
                installation: installation.clone(),
                origin,
                deployment: None,
                deployment_receipt: None,
                members,
                completed: false,
            }
        }
    };
    let previous_boot = record
        .origin
        .as_ref()
        .map(|o| process::current_boot(&o.owner).map(|current| !current))
        .transpose()?
        .unwrap_or(false);
    tx.verify_lock()?;
    lifecycle.verify()?;
    drop(tx);

    // Reboot is an independent proof: none of the original descendants can
    // survive it. Do not reopen a reused PID or require the old app to exist.
    let deployed = if record.origin.is_some() && !previous_boot {
        let deployed = deploy::completed_inventory(store, refs, &instance, deadline)?
            .ok_or_else(unverifiable)?;
        let receipt = deployed.receipt()?;
        if record
            .deployment
            .as_ref()
            .is_some_and(|target| target != &deployed.target)
            || record
                .deployment_receipt
                .as_ref()
                .is_some_and(|original| original != &receipt)
            || (prior.is_some() && record.deployment.is_none())
        {
            return Err(unverifiable());
        }
        record.deployment = Some(deployed.target.clone());
        record.deployment_receipt = Some(receipt);
        Some(deployed)
    } else {
        None
    };
    let files = match (&deployed, &record.origin) {
        (Some(deployed), Some(origin)) => Some(ExecutableSet::open(
            origin.scope.as_ref().ok_or_else(unverifiable)?.app(),
            &deployed.inventory.execution_paths(deadline)?,
            deadline,
        )?),
        _ => None,
    };
    let mut tx = store.transaction(deadline)?;
    if tx.lock_identity()? != state_lock || journal::load(store, refs, &instance)? != prior {
        return Err(unverifiable());
    }
    verify_binding(&tx, refs, &record, false, deadline)?;
    lifecycle.verify()?;
    if let Some(deployed) = &deployed {
        deployed.verify_in(&tx, refs, deadline)?;
    }
    if prior.is_none() {
        journal::save(store, refs, &record)?;
    }
    // Persist the stop intent before the registry transition and before every signal.
    tx.verify_lock()?;
    lifecycle.verify()?;
    tx.registry
        .instances
        .iter_mut()
        .find(|i| i.id == instance.id)
        .ok_or_else(unverifiable)?
        .state = "stopping".into();
    tx.commit()?;
    let mut operation = Operation {
        store,
        refs,
        lifecycle,
        state_lock,
        record,
        deployed,
        deadline,
    };
    operation.complete(files.as_ref()).map_err(|_| {
        let mut error = unknown();
        error.details = Some(Box::new(
            crate::core::contracts::failure::Details::Instance {
                instance_id: instance.id.clone(),
            },
        ));
        error.next_action = Some(Box::new(
            crate::core::contracts::action::NextAction::Doctor {
                instance: Some(instance.id),
            },
        ));
        error
    })
}

struct Operation<'a> {
    store: &'a Store,
    refs: &'a References,
    lifecycle: &'a Lock,
    state_lock: (u64, u64, u64, u64),
    record: Record,
    deployed: Option<Completed>,
    deadline: Instant,
}
impl Operation<'_> {
    fn complete(&mut self, files: Option<&ExecutableSet>) -> Result<Stopped> {
        let deadline = self.deadline;
        let mut members = self.record.members.clone();
        if let Some(files) = files {
            members::execute(files, &mut members, self, deadline).map_err(|_| unknown())?;
        } else {
            self.guard()?;
            for member in &members {
                if !process::exited(member.target.identity(), deadline)? {
                    return Err(unknown());
                }
            }
        }
        // A fresh full digest, not the metadata cache, closes the stop proof.
        if let Some(deployed) = &self.deployed {
            deployed.inventory.revalidate(deadline)?;
        }
        self.guard()?;
        let mut complete = self.record.clone();
        complete.completed = true;
        self.publish(complete).map_err(|_| unknown())?;
        let tx = self.store.transaction(deadline)?;
        self.verify_in(&tx)?;
        acknowledge(tx, self.lifecycle, &self.record).map_err(|_| unknown())?;
        Ok(Stopped {
            instance_id: self.record.instance.id.clone(),
            reused: false,
        })
    }
    fn verify_in(&self, tx: &Transaction<'_>) -> Result<()> {
        crate::core::deadline::check(self.deadline)?;
        self.lifecycle.verify()?;
        tx.verify_lock()?;
        if tx.lock_identity()? != self.state_lock
            || journal::load(self.store, self.refs, &self.record.instance)?.as_ref()
                != Some(&self.record)
        {
            return Err(unverifiable());
        }
        verify_binding(tx, self.refs, &self.record, true, self.deadline)?;
        if let Some(deployed) = &self.deployed {
            deployed.verify_in(tx, self.refs, self.deadline)?;
        }
        self.lifecycle.verify()?;
        tx.verify_lock()
    }
    fn publish(&mut self, next: Record) -> Result<()> {
        let tx = self.store.transaction(self.deadline)?;
        self.verify_in(&tx)?;
        journal::save(self.store, self.refs, &next)?;
        self.lifecycle.verify()?;
        tx.verify_lock()?;
        self.record = next;
        Ok(())
    }
}
impl Checkpoint for Operation<'_> {
    fn guard(&self) -> Result<()> {
        self.verify_in(&self.store.transaction(self.deadline)?)
    }
    fn save(&mut self, members: &[Member]) -> Result<()> {
        let mut next = self.record.clone();
        next.members = members.to_vec();
        self.publish(next)
    }
}

fn original(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<Option<run::Origin>> {
    if run::has_record(tx.store(), &instance.id, &instance.run_id)? {
        return run::stop_origin(tx, refs, instance, installation, deadline).map(Some);
    }
    if matches!(
        instance.state.as_str(),
        "preparing" | "prepared" | "stopping" | "stopped"
    ) && instance.account.is_none()
        && instance.process_id.is_none()
        && instance.process_started.is_none()
        && !start_intent::has_run_claim(tx.store(), &instance.id, &instance.run_id)?
    {
        return Ok(None);
    }
    Err(unverifiable())
}
fn verify_binding(
    tx: &Transaction<'_>,
    refs: &References,
    record: &Record,
    stopping: bool,
    deadline: Instant,
) -> Result<()> {
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == record.instance.id)
        .ok_or_else(unverifiable)?;
    let mut expected = record.instance.clone();
    if stopping {
        expected.state = "stopping".into();
    } else if matches!(instance.state.as_str(), "stopping" | "stopped") {
        expected.state = instance.state.clone();
    }
    if *instance != expected
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == record.installation.id)
            .count()
            != 1
        || !tx.registry.installations.contains(&record.installation)
        || original(tx, refs, instance, &record.installation, deadline)? != record.origin
    {
        return Err(unverifiable());
    }
    tx.verify_lock()
}
fn acknowledge(mut tx: Transaction<'_>, lifecycle: &Lock, record: &Record) -> Result<()> {
    if !record.completed {
        return Err(unverifiable());
    }
    tx.verify_lock()?;
    lifecycle.verify()?;
    tx.registry
        .instances
        .iter_mut()
        .find(|i| i.id == record.instance.id)
        .ok_or_else(unverifiable)?
        .state = "stopped".into();
    tx.commit()?;
    lifecycle.verify()
}
