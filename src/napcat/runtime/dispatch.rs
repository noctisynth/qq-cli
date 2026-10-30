//! Bind all reviewed startup inputs to the once-only native dispatch journal.
#![cfg(windows)]
use crate::{
    core::error::{AppError, Result},
    napcat::compatibility::Witness,
    napcat::doctor::Probe,
    napcat::install::catalog::Release,
    napcat::runtime::launch,
    napcat::runtime::run_job::{self, Running},
    napcat::runtime::runtime_store::{self, Plan},
    napcat::runtime::start_intent::Target,
    napcat::runtime::startup::Inputs,
    native::job::SpawnSpec,
    storage::credentials::Credentials,
    storage::references::References,
    storage::state::Transaction,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::Instant;

/// Private, non-serializable inputs for one transaction, not an executable permit.
pub struct Request<'a, C: Credentials, P: Probe> {
    pub inputs: &'a Inputs,
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
    pub probe: &'a P,
    pub references: &'a References,
    pub credentials: &'a C,
    pub runtime: &'a Plan,
    pub inherited_environment: &'a BTreeMap<OsString, OsString>,
}

/// Owns all data borrowed by SpawnSpec, with no Debug/Serialize implementation.
pub struct Checked {
    bootstrap: launch::Bootstrap,
    environment: BTreeMap<OsString, OsString>,
    target: Target,
}
impl Checked {
    pub fn target(&self) -> &Target {
        &self.target
    }
    pub fn spec(&self) -> SpawnSpec<'_> {
        SpawnSpec {
            program: &self.bootstrap.program,
            arguments: &self.bootstrap.arguments,
            environment: &self.environment,
            directory: &self.bootstrap.current_directory,
        }
    }
}

/// Called both before reserving an intent and again within its dispatch transaction.
pub fn check_in<C: Credentials, P: Probe>(
    tx: &Transaction<'_>,
    request: &Request<'_, C, P>,
    deadline: Instant,
) -> Result<Checked> {
    request.inputs.with_revalidated(
        tx,
        request.release,
        request.witnesses,
        request.probe,
        deadline,
        |tx, inputs| {
            let material = runtime_store::verify_prepared(
                tx,
                request.references,
                request.credentials,
                request.runtime,
                deadline,
            )?;
            let instance = tx
                .registry
                .instances
                .iter()
                .find(|i| i.id == request.runtime.instance_id)
                .ok_or_else(conflict)?;
            if instance.installation_id != inputs.installation().id {
                return Err(conflict());
            }
            let mut bootstrap = launch::bootstrap(
                &inputs.application().executable,
                &inputs.installation().directory,
                material.directory(),
            )?;
            let mut environment = launch::environment(
                request.inherited_environment.clone(),
                inputs
                    .application()
                    .executable
                    .parent()
                    .ok_or_else(conflict)?,
                material.directory(),
            )?;
            if material.bundle().friend_requests_enabled() {
                launch::friend_requests_entry(&mut bootstrap, material.directory())?;
                environment.insert(
                    "QQ_CLI_FRAMEWORK_ROOT".into(),
                    inputs.installation().directory.as_os_str().into(),
                );
            }
            let input_hash = input_hash(
                inputs.compatibility(),
                request.runtime,
                &bootstrap,
                &environment,
                material.bundle(),
            )?;
            let target = Target {
                instance_id: request.runtime.instance_id.clone(),
                run_id: request.runtime.run_id.clone(),
                installation_id: inputs.installation().id.clone(),
                installation_digest: inputs.installation().digest.clone(),
                input_hash,
            };
            crate::core::deadline::check(deadline)?;
            let checked = Checked {
                bootstrap,
                environment,
                target,
            };
            checked.spec().validate()?;
            Ok(checked)
        },
    )
}

/// Final checked path into the native once-only primitive. A saved hash never
/// replaces fresh checks, and a mismatch cannot allocate a Job or create a process.
pub fn launch_in<C: Credentials, P: Probe>(
    tx: &Transaction<'_>,
    request: &Request<'_, C, P>,
    request_id: &str,
    target: &Target,
    deadline: Instant,
) -> Result<Running> {
    let checked = check_in(tx, request, deadline)?;
    if checked.target() != target {
        return Err(conflict());
    }
    let control_endpoint = format!("http://127.0.0.1:{}/", request.runtime.ports.control);
    run_job::launch_for_service_once(
        tx,
        request.references,
        request_id,
        target,
        &checked.spec(),
        &run_job::ServiceExpectation {
            image: &request.inputs.application().executable,
            control_endpoint: &control_endpoint,
            ports: request.runtime.ports,
        },
        deadline,
    )
}
fn conflict() -> AppError {
    AppError::new(
        "START_INPUTS_CHANGED",
        "Complete launch inputs do not match the reserved startup intent; no native dispatch was attempted.",
        4,
    )
}

/// Stable v1 encoding shared by dispatch and verification of an observed run.
pub(crate) fn input_hash(
    key: &crate::napcat::compatibility::Key,
    runtime: &Plan,
    bootstrap: &launch::Bootstrap,
    environment: &BTreeMap<OsString, OsString>,
    bundle: &crate::napcat::runtime::runtime_config::Bundle,
) -> Result<String> {
    let mut config = Vec::new();
    for file in bundle.files() {
        let mut bytes = Vec::new();
        file.write_to(&mut bytes)?;
        config.push((file.name(), format!("{:x}", Sha256::digest(&bytes))));
    }
    let arguments = bootstrap
        .arguments
        .iter()
        .map(|a| a.to_str().ok_or_else(conflict))
        .collect::<Result<Vec<_>>>()?;
    let env = environment
        .iter()
        .map(|(k, v)| {
            Ok((
                k.to_str().ok_or_else(conflict)?,
                v.to_str().ok_or_else(conflict)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let encoded = serde_json::to_vec(&(
        1u8,
        key,
        runtime,
        bootstrap.program.to_str().ok_or_else(conflict)?,
        arguments,
        bootstrap.current_directory.to_str().ok_or_else(conflict)?,
        env,
        config,
    ))
    .map_err(|_| conflict())?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}
