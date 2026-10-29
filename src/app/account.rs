//! Account user tasks; protocol execution and idempotency stay shared with api.call.
use crate::{
    cli::{account::AccountCommand, args::ApiCallArgs},
    core::{
        contracts::{
            account::*,
            scalar::{LocalId, No, Yes},
        },
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use serde_json::Value;
use std::{fs::File, io::Read, time::Instant};
struct Prepared {
    action: &'static str,
    payload: Value,
    fields: Vec<Field>,
    operation: Operation,
    bytes: Option<u64>,
}
fn payload<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| AppError::invalid())
}
fn prepare(command: &AccountCommand) -> Result<Prepared> {
    let (action, payload, fields, operation, bytes) = match command {
        AccountCommand::Set {
            nickname,
            signature,
            sex,
            ..
        } => {
            let mut fields = Vec::new();
            let (action, body) = if let Some(nickname) = nickname {
                fields.push(Field::Nickname);
                if signature.is_some() {
                    fields.push(Field::Signature);
                }
                if sex.is_some() {
                    fields.push(Field::Sex);
                }
                let sex = sex.as_deref().map(|sex| {
                    NumberOrString::Number(match sex {
                        "male" => 1.0,
                        "female" => 2.0,
                        _ => 0.0,
                    })
                });
                (
                    "set_qq_profile",
                    payload(ProfilePayload {
                        nickname: nickname.to_string(),
                        personal_note: signature.as_ref().map(ToString::to_string),
                        sex,
                    })?,
                )
            } else {
                let signature = signature.as_ref().ok_or_else(AppError::invalid)?;
                fields.push(Field::Signature);
                (
                    "set_self_longnick",
                    payload(SignaturePayload {
                        signature: signature.to_string(),
                    })?,
                )
            };
            (action, body, fields, Operation::Set, None)
        }
        AccountCommand::Avatar { file, .. } => {
            let source = File::open(file).map_err(|_| avatar_input())?;
            if !source.metadata().map_err(|_| avatar_input())?.is_file() {
                return Err(avatar_input());
            }
            let mut bytes = Vec::new();
            source
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| avatar_input())?;
            if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
                return Err(avatar_input());
            }
            let size = bytes.len() as u64;
            (
                "set_qq_avatar",
                payload(AvatarPayload {
                    file: format!("base64://{}", STANDARD.encode(bytes)),
                })?,
                vec![Field::Avatar],
                Operation::Avatar,
                Some(size),
            )
        }
        AccountCommand::Presence {
            status,
            ext_status,
            battery_status,
            ..
        } => (
            "set_online_status",
            payload(PresencePayload {
                status: NumberOrString::Number(f64::from(*status)),
                ext_status: NumberOrString::Number(f64::from(*ext_status)),
                battery_status: NumberOrString::Number(f64::from(*battery_status)),
            })?,
            vec![Field::Status, Field::ExtStatus, Field::BatteryStatus],
            Operation::Presence,
            None,
        ),
        AccountCommand::StatusText {
            text,
            face_id,
            face_type,
            ..
        } => (
            "set_diy_online_status",
            payload(StatusTextPayload {
                face_id: NumberOrString::Text(face_id.to_string()),
                face_type: NumberOrString::Text(face_type.to_string()),
                wording: text.to_string(),
            })?,
            vec![Field::StatusText, Field::FaceId, Field::FaceType],
            Operation::StatusText,
            None,
        ),
    };
    Ok(Prepared {
        action,
        payload,
        fields,
        operation,
        bytes,
    })
}
fn avatar_input() -> AppError {
    AppError::new(
        "AVATAR_INPUT",
        "Avatar must be a readable nonempty regular image file of at most 8 MiB.",
        2,
    )
}
pub(super) fn offline(command: &AccountCommand) -> Result<Option<Envelope>> {
    if !command.write().dry_run {
        return Ok(None);
    }
    let prepared = prepare(command)?;
    Ok(Some(Envelope::typed(Draft {
        operation: prepared.operation,
        dry_run: Yes,
        identity_verified: No,
        requested_fields: prepared.fields,
        input_bytes: prepared.bytes,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &AccountCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    let prepared = prepare(command)?;
    let write = command.write();
    let args = ApiCallArgs {
        action: prepared.action.parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: write.request_id.clone(),
        dry_run: false,
        deadline: crate::cli::args::DeadlineArgs {
            timeout: write.deadline.timeout,
        },
    };
    let (result, scope) = super::api::execute_prepared(
        &args,
        prepared.payload,
        session,
        store,
        references,
        deadline,
    )?
    .into_api_result()?;
    Ok(Envelope::typed(Receipt {
        operation: prepared.operation,
        request_id: LocalId::new(
            result
                .request_id
                .as_deref()
                .ok_or_else(AppError::protocol)?,
        )?,
        outcome: result.outcome.ok_or_else(AppError::protocol)?,
        deduplicated: result.deduplicated,
        requested_fields: prepared.fields,
    })
    .scoped(scope))
}
