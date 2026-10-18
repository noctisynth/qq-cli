//! Bind all reviewed startup inputs to the once-only native dispatch journal.
#![cfg(windows)]
use crate::{
    catalog::Release,
    compatibility::Witness,
    credentials::Credentials,
    doctor::Probe,
    error::{AppError, Result},
    launch,
    native_job::SpawnSpec,
    references::References,
    run_job::{self, Running},
    runtime_store::{self, Plan},
    start_intent::Target,
    startup::Inputs,
    state::Transaction,
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
            let bootstrap = launch::bootstrap(
                &inputs.application().executable,
                &inputs.installation().directory,
            )?;
            let environment = launch::environment(
                request.inherited_environment.clone(),
                inputs
                    .application()
                    .executable
                    .parent()
                    .ok_or_else(conflict)?,
                material.directory(),
            )?;
            let mut config = Vec::new();
            for file in material.bundle().files() {
                let mut bytes = Vec::new();
                file.write_to(&mut bytes)?;
                config.push((file.name(), format!("{:x}", Sha256::digest(&bytes))));
            }
            // Unicode validation here matches the native process boundary. Persist
            // only the digest, never this private command/environment/config material.
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
                inputs.compatibility(),
                &request.runtime,
                bootstrap.program.to_str().ok_or_else(conflict)?,
                arguments,
                bootstrap.current_directory.to_str().ok_or_else(conflict)?,
                env,
                config,
            ))
            .map_err(|_| conflict())?;
            let target = Target {
                instance_id: request.runtime.instance_id.clone(),
                run_id: request.runtime.run_id.clone(),
                installation_id: inputs.installation().id.clone(),
                installation_digest: inputs.installation().digest.clone(),
                input_hash: format!("{:x}", Sha256::digest(encoded)),
            };
            crate::deadline::check(deadline)?;
            Ok(Checked {
                bootstrap,
                environment,
                target,
            })
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
