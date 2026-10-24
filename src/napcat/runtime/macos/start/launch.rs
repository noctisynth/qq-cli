//! Once-only sandboxed creation and publication of the exact child identity.
use super::super::run;
use super::*;
use crate::{native::process::Pinned, storage::state::Transaction};
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::Duration,
};

pub(super) struct Launch {
    prepared: prepare::Prepared,
    profile: String,
    key: super::super::compatibility::Key,
    scope: run::Scope,
    pub target: start_intent::Target,
}
impl Launch {
    pub(super) fn executable(&self) -> std::path::PathBuf {
        self.prepared.adapter.executable()
    }
    pub(super) fn verify_service(
        &self,
        service: &run::ServiceRun,
        deadline: Instant,
    ) -> Result<()> {
        if !service.verifiable_inputs()
            || service.scope() != Some(&self.scope)
            || service.target() != &self.target
            || service.compatibility() != Some(&self.key)
            || service.owner().identity().image() != self.executable()
        {
            return Err(conflict());
        }
        service.owner().verify(deadline)
    }
    pub(super) fn new(prepared: prepare::Prepared) -> Result<Self> {
        let framework = prepared.adapter.framework_root();
        let profile = sandbox::profile(prepared.material.directory(), framework)?;
        let runtime = &prepared.runtime;
        let mut files = Vec::new();
        for file in prepared.material.bundle().files() {
            let mut bytes = Vec::new();
            file.write_to(&mut bytes)?;
            files.push((file.name(), format!("{:x}", Sha256::digest(bytes))));
        }
        let key = super::super::compatibility::Key::current()?;
        let scope = run::Scope::declare(prepared.material.directory().join("QQ.app"), &profile);
        let input = serde_json::to_vec(&(
            "qq-cli-macos-start-input-v3",
            &key,
            &scope,
            &prepared.deployment,
            runtime,
            &profile,
            files,
            prepared.adapter.arguments(),
            environment(&prepared)?.into_iter().collect::<Vec<_>>(),
            prepared.adapter.current_directory(),
        ))
        .map_err(|_| conflict())?;
        let mut target = prepared.deployment.clone();
        target.input_hash = format!("{:x}", Sha256::digest(input));
        Ok(Self {
            prepared,
            profile,
            key,
            scope,
            target,
        })
    }
    pub(super) fn dispatch(
        &self,
        tx: &Transaction<'_>,
        context: &Context<'_, impl Credentials>,
        request: &str,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<Pinned> {
        self.dispatch_checked(tx, context.references, request, deadline, guard, &|| {
            guard()?;
            tx.verify_lock()?;
            self.prepared.verify_in(tx, context, deadline)
        })
    }
    fn dispatch_checked(
        &self,
        tx: &Transaction<'_>,
        refs: &References,
        request: &str,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
        preflight: &dyn Fn() -> Result<()>,
    ) -> Result<Pinned> {
        preflight()?;
        guard()?;
        tx.verify_lock()?;
        let mut child = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", &self.profile])
            .arg(self.prepared.adapter.executable())
            .args(self.prepared.adapter.arguments())
            .env_clear()
            .envs(environment(&self.prepared)?)
            .current_dir(self.prepared.adapter.current_directory())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| unresolved())?;
        // sandbox-exec replaces itself. Observe only this Child's PID until
        // the expected executable is present; never search/adopt another PID.
        loop {
            crate::core::deadline::check(deadline)?;
            guard()?;
            tx.verify_lock()?;
            if child.try_wait().map_err(|_| unresolved())?.is_some() {
                return Err(unresolved());
            }
            if let Ok(owner) = Pinned::observe(child.id(), deadline)
                && owner.identity().image() == self.prepared.adapter.executable()
            {
                owner.verify(deadline)?;
                let control = format!("http://127.0.0.1:{}/", self.prepared.runtime.ports.control);
                run::publish(
                    tx,
                    refs,
                    request,
                    &owner,
                    run::Binding {
                        target: &self.target,
                        compatibility: &self.key,
                        scope: &self.scope,
                        control_endpoint: &control,
                    },
                    deadline,
                )?;
                guard()?;
                return Ok(owner);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn environment(
    prepared: &prepare::Prepared,
) -> Result<std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>> {
    let mut environment = prepared.adapter.environment();
    prepared
        .material
        .bundle()
        .visit_environment(|name, value| {
            environment.insert(name.into(), value.into());
            Ok(())
        })?;
    Ok(environment)
}

#[cfg(test)]
mod tests;
