use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Secrets {
    schema_version: u8,
    binding: String,
    api: String,
    control: String,
    jwt: String,
}
pub(super) fn digest(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}
fn binding(target: &Target) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-material-v1\0");
    hash.update(serde_json::to_vec(target).map_err(|_| invalid())?);
    Ok(format!("{:x}", hash.finalize()))
}
fn keys(plan: &Plan) -> Result<(String, String)> {
    let id = runtime_store::identity(&plan.instance_id, &plan.run_id)?;
    Ok((
        format!("runtime-bundle-{id}"),
        runtime_store::api_credential_key(&plan.instance_id, &plan.run_id)?,
    ))
}
fn get(credentials: &impl Credentials, key: &str, deadline: Instant) -> Result<Option<String>> {
    crate::core::deadline::check(deadline)?;
    let value = match credentials.get(key) {
        Ok(v) if v.len() <= 2048 => Some(v),
        Ok(_) => return Err(invalid()),
        Err(e) if e.code == "CREDENTIAL_NOT_FOUND" => None,
        Err(_) => return Err(invalid()),
    };
    crate::core::deadline::check(deadline)?;
    Ok(value)
}
pub(super) fn require_absent(
    credentials: &impl Credentials,
    plan: &Plan,
    deadline: Instant,
) -> Result<()> {
    let (bundle, api) = keys(plan)?;
    if get(credentials, &bundle, deadline)?.is_some() || get(credentials, &api, deadline)?.is_some()
    {
        return Err(invalid());
    }
    Ok(())
}
pub(super) fn generate(target: &Target) -> Result<(Bundle, String)> {
    let bundle = Bundle::generate(&target.plan.version, target.plan.ports)?;
    let raw = serde_json::to_string(&Secrets {
        schema_version: 1,
        binding: binding(target)?,
        api: bundle.api_token().into(),
        control: bundle.control_token().into(),
        jwt: bundle.jwt_secret().into(),
    })
    .map_err(|_| invalid())?;
    Ok((bundle, raw))
}
pub(super) fn recover(
    credentials: &impl Credentials,
    record: &Record,
    deadline: Instant,
) -> Result<Bundle> {
    let (key, api_key) = keys(&record.target.plan)?;
    let raw = get(credentials, &key, deadline)?.ok_or_else(invalid)?;
    if digest(&raw) != record.secret_digest {
        return Err(invalid());
    }
    let secrets: Secrets = serde_json::from_str(&raw).map_err(|_| invalid())?;
    if secrets.schema_version != 1 || secrets.binding != binding(&record.target)? {
        return Err(invalid());
    }
    let bundle = Bundle::restore(
        &record.target.plan.version,
        record.target.plan.ports,
        secrets.api,
        secrets.control,
        secrets.jwt,
    )?;
    if get(credentials, &api_key, deadline)?.as_deref() != Some(bundle.api_token()) {
        return Err(invalid());
    }
    Ok(bundle)
}
pub(super) fn publish(
    credentials: &impl Credentials,
    record: &Record,
    raw: &str,
    bundle: &Bundle,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let (key, api_key) = keys(&record.target.plan)?;
    for (key, value) in [(&key, raw), (&api_key, bundle.api_token())] {
        guard()?;
        if get(credentials, key, deadline)?.is_some() {
            return Err(invalid());
        }
        // The lifecycle lock serializes cooperating clients. Credentials::set is
        // not a CAS primitive and is not hostile same-user isolation.
        credentials.set(key, value).map_err(|_| invalid())?;
        guard()?;
        if get(credentials, key, deadline)?.as_deref() != Some(value) {
            return Err(invalid());
        }
    }
    recover(credentials, record, deadline)?;
    Ok(())
}
