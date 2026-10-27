//! Authenticated fixed-input assembly under an at-most-once deployment intent.
use super::{
    adapter::{AdapterPlan, PackageEdit, Prepared},
    deploy::{self, Status, Target},
};
use crate::core::error::{AppError, Result};
#[cfg(test)]
use crate::napcat::install::catalog;
use crate::napcat::install::{installation, macos::source::Source};
use crate::storage::{
    references::References,
    state::{Installation, Instance, Store, Transaction, reject_links},
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

mod inputs;
mod kernel;
pub use kernel::Assembly;

/// Not serializable: a digest cannot restore these authenticated observations.
pub struct Plan<'a> {
    source: &'a Source,
    layout: super::layout::Layout,
    store_root: PathBuf,
    state_lock: (u64, u64, u64, u64),
    instance: Instance,
    installation: Installation,
    prepared: Prepared,
    adapter: AdapterPlan,
    package: PackageEdit,
    target: Target,
}
impl<'a> Plan<'a> {
    /// Read-only planning in an already prepared runtime layout. No adapter/app
    /// publication, source copying, signing, process or credential creation.
    pub fn inspect(
        store: &Store,
        references: &References,
        source: &'a Source,
        instance_id: &str,
        deadline: Instant,
    ) -> Result<Self> {
        let lifecycle = crate::napcat::runtime::lifecycle_lock(store, deadline)?;
        let store_root = store_path(store)?;
        let transaction = store.transaction(deadline)?;
        let state_lock = transaction.lock_identity()?;
        let (instance, installation) = scope(&transaction, instance_id)?;
        transaction.verify_lock()?;
        drop(transaction);
        let release = super::release::lookup(&installation.version)?;
        if installation.version != release.version || installation.digest != release.archive.sha256
        {
            return Err(invalid());
        }
        lifecycle.verify()?;
        installation::verify(store, &installation, references, deadline)?;
        source.revalidate(deadline)?;
        let root =
            store_root
                .join("runtimes")
                .join(crate::napcat::runtime::runtime_store::identity(
                    &instance.id,
                    &instance.run_id,
                )?);
        let Some(super::layout::Preparation::Ready(layout)) =
            super::layout::recover(store, references, instance_id, deadline)?
        else {
            return Err(invalid());
        };
        if layout.directory() != root {
            return Err(invalid());
        }
        let prepared =
            Prepared::inspect_with_profile(&root, &installation.directory, layout.profile_home())?;
        let adapter = prepared.adapter_plan()?;
        let package = prepared.plan_source_package(source, deadline)?;
        let input_hash = inputs::digest(
            source,
            &instance,
            &installation,
            &prepared,
            &adapter,
            &package,
            &layout,
        )?;
        let target = Target {
            instance_id: instance.id.clone(),
            run_id: instance.run_id.clone(),
            installation_id: installation.id.clone(),
            installation_digest: installation.digest.clone(),
            input_hash,
        };
        let plan = Self {
            source,
            layout,
            store_root,
            state_lock,
            instance,
            installation,
            prepared,
            adapter,
            package,
            target,
        };
        plan.check_scope(store, deadline)?;
        lifecycle.verify()?;
        Ok(plan)
    }

    pub fn target(&self) -> &Target {
        &self.target
    }

    /// This is deployment only, never a start permit. Existing Pending/Unknown
    /// intents cannot re-enter the callback, including with another request key.
    pub fn deploy(
        &self,
        store: &Store,
        references: &References,
        request_id: &str,
        deadline: Instant,
    ) -> Result<Status> {
        self.check_scope(store, deadline)?;
        let mut failure = None;
        let status = deploy::deploy(
            store,
            references,
            request_id,
            &self.target,
            deadline,
            |root, guard| {
                let result: Result<_> = (|| {
                    guard()?;
                    self.revalidate(store, references, deadline)?;
                    guard()?;
                    let checked_guard = || {
                        guard()?;
                        self.layout.verify(deadline)
                    };
                    let roles = self.source.signing_roles()?;
                    let copy = Assembly {
                        source: self.source.inventory(),
                        roles: &roles,
                        prepared: &self.prepared,
                        adapter: &self.adapter,
                        package: &self.package,
                    }
                    .execute(root, deadline, checked_guard)?;
                    self.revalidate(store, references, deadline)?;
                    guard()?;
                    Ok(copy)
                })();
                if let Err(error) = &result {
                    failure = Some(error.clone());
                }
                result
            },
        )?;
        if let Some(mut error) = failure {
            error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
            return Err(error);
        }
        // The d2 artifact covers QQ.app. Its external entry and Framework are
        // also required when returning an already completed deployment, whose
        // original callback is deliberately never run again.
        if status == Status::Complete {
            self.layout.verify(deadline)?;
            self.check_scope(store, deadline)?;
            verify_published(
                store,
                references,
                &self.installation,
                &self.adapter,
                deadline,
            )?;
            self.check_scope(store, deadline)?;
        }
        Ok(status)
    }

    fn check_scope(&self, store: &Store, deadline: Instant) -> Result<()> {
        if store_path(store)? != self.store_root {
            return Err(invalid());
        }
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != self.state_lock
            || scope(&transaction, &self.instance.id)?
                != (self.instance.clone(), self.installation.clone())
        {
            return Err(invalid());
        }
        transaction.verify_lock()
    }
    /// Read-only revalidation; never restores a deployment or process permit.
    pub fn revalidate(
        &self,
        store: &Store,
        references: &References,
        deadline: Instant,
    ) -> Result<()> {
        self.check_scope(store, deadline)?;
        installation::verify(store, &self.installation, references, deadline)?;
        self.source.revalidate(deadline)?;
        self.layout.verify(deadline)?;
        Prepared::inspect_with_profile(
            self.prepared.current_directory(),
            &self.installation.directory,
            self.layout.profile_home(),
        )?;
        self.check_scope(store, deadline)
    }
}

fn store_path(store: &Store) -> Result<PathBuf> {
    reject_links(store.root())?;
    let path = store.root().canonicalize().map_err(|_| invalid())?;
    if path != store.root() {
        return Err(invalid());
    }
    Ok(path)
}
fn scope(transaction: &Transaction<'_>, id: &str) -> Result<(Instance, Installation)> {
    let mut matches = transaction.registry.instances.iter().filter(|v| v.id == id);
    let instance = matches.next().ok_or_else(invalid)?;
    if matches.next().is_some()
        || instance.state != "prepared"
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
    {
        return Err(invalid());
    }
    let mut matches = transaction
        .registry
        .installations
        .iter()
        .filter(|v| v.id == instance.installation_id);
    let installation = matches.next().ok_or_else(invalid)?;
    if matches.next().is_some() {
        return Err(invalid());
    }
    Ok((instance.clone(), installation.clone()))
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_ASSEMBLY_CONFLICT",
        "Fixed deployment inputs or prepared instance changed; no successful assembly was accepted.",
        4,
    )
}

fn verify_published(
    store: &Store,
    references: &References,
    installed: &Installation,
    adapter: &AdapterPlan,
    deadline: Instant,
) -> Result<()> {
    installation::verify(store, installed, references, deadline)?;
    adapter.verify()?;
    Ok(())
}

#[cfg(test)]
mod tests;
