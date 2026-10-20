//! Body-free, scoped replay boundaries. A checkpoint never proves continuous delivery.
use crate::core::{
    domain::{Conversation, Scope},
    error::{AppError, Result},
};
use crate::messaging::message::Locator;
use crate::storage::references::References;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Checkpoint {
    schema_version: u8,
    scope: Scope,
    run: String,
    target: Option<Conversation>,
    pub since: i64,
    seen: Vec<String>,
}
impl Checkpoint {
    pub fn new(scope: Scope, run: String, target: Option<Conversation>) -> Self {
        Self {
            schema_version: 1,
            scope,
            run,
            target,
            since: now() - 1,
            seen: vec![],
        }
    }
    pub fn restore(&mut self, refs: &References, token: &str) -> Result<()> {
        let old: Self = refs.get("w1", token)?;
        let now = now();
        if old.schema_version != 1
            || old.scope != self.scope
            || old.run != self.run
            || old.target.is_none()
            || old.target != self.target
            || old.seen.len() > 256
            || old
                .seen
                .iter()
                .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            || old.since > now
            || now.saturating_sub(old.since) > 86_400
        {
            return Err(AppError::new(
                "EVENT_CHECKPOINT_STALE",
                "Event checkpoint is expired or belongs to a different account, instance, run or conversation.",
                4,
            ));
        }
        *self = old;
        Ok(())
    }
    pub fn contains(&self, id: &str) -> bool {
        self.seen.iter().any(|s| s == id)
    }
    pub fn remember(&mut self, id: String) {
        if self.seen.len() == 256 {
            self.seen.remove(0);
        }
        self.seen.push(id);
    }
    pub fn token(&self, refs: &References) -> Result<Option<String>> {
        self.target
            .as_ref()
            .map(|_| refs.issue("w1", self))
            .transpose()
    }
}
fn now() -> i64 {
    chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now()).timestamp()
}
pub(super) fn message_id(locator: &Locator) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(locator).map_err(|_| AppError::protocol())?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> Checkpoint {
        Checkpoint::new(
            Scope {
                instance_id: "instance-example".into(),
                account: "10001".into(),
            },
            "run-example".into(),
            Some("private:10002".parse().unwrap()),
        )
    }
    #[test]
    fn checkpoint_binds_identity_run_target_age_and_bounded_body_free_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let refs = References::with_key(tmp.path().join("refs"), [77; 32]);
        let mut original = state();
        for n in 0..300 {
            original.remember(format!("{n:064x}"));
        }
        assert_eq!(original.seen.len(), 256);
        let token = original.token(&refs).unwrap().unwrap();
        state().restore(&refs, &token).unwrap();
        let payload: serde_json::Value = refs.get("w1", &token).unwrap();
        assert!(payload.get("body").is_none());
        for variant in 0..6 {
            let mut wrong = state();
            match variant {
                0 => wrong.scope.account = "10003".into(),
                1 => wrong.scope.instance_id = "instance-other".into(),
                2 => wrong.run = "run-other".into(),
                3 => wrong.target = Some("private:10003".parse().unwrap()),
                4 => wrong.since = now() - 86_401,
                _ => wrong.since = now() + 3600,
            }
            let token = wrong.token(&refs).unwrap().unwrap();
            assert_eq!(
                state().restore(&refs, &token).unwrap_err().code,
                "EVENT_CHECKPOINT_STALE"
            );
        }
        let mut bytes = token.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = if bytes[last] == b'A' { b'B' } else { b'A' };
        assert!(
            state()
                .restore(&refs, std::str::from_utf8(&bytes).unwrap())
                .is_err()
        );
    }
}
