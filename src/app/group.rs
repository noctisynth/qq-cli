//! One typed group operation per caller key; upstream owns QQ role enforcement.
mod content;
mod read;
mod requests;
use crate::{
    cli::{args::ApiCallArgs, group::GroupCommand},
    core::{
        contracts::account::NumberOrString,
        contracts::{
            group::*,
            scalar::{LocalId, No, Yes},
        },
        error::{AppError, Result},
        input::DecimalId,
        output::Envelope,
    },
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::time::Instant;
struct Prepared {
    action: &'static str,
    payload: Value,
    operation: Operation,
    group_id: DecimalId,
}
fn prepare(command: &GroupCommand) -> Result<Option<Prepared>> {
    let (action, payload, operation, group_id) = match command {
        GroupCommand::Notice(args) => {
            let crate::cli::group::NoticeArgs {
                group_id,
                content,
                image,
                pinned,
                confirm_required,
                notice_type,
                show_edit_card,
                tip_window_type,
                ..
            } = args.as_ref();
            let image = image
                .as_ref()
                .map(|path| {
                    super::profile_image::read(path).map(|v| v.0).map_err(|_| {
                        AppError::new(
                            "NOTICE_IMAGE_INPUT",
                            "Notice image must be a nonempty regular file of at most 8 MiB.",
                            2,
                        )
                    })
                })
                .transpose()?;
            (
                "_send_group_notice",
                serde_json::to_value(
                    crate::core::contracts::group_content::PublishNoticePayload {
                        group_id: group_id.to_string(),
                        content: content
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                        image,
                        pinned: NumberOrString::Number(if *pinned { 1.0 } else { 0.0 }),
                        notice_type: NumberOrString::Number(f64::from(*notice_type)),
                        confirm_required: NumberOrString::Number(if *confirm_required {
                            1.0
                        } else {
                            0.0
                        }),
                        is_show_edit_card: NumberOrString::Number(if *show_edit_card {
                            1.0
                        } else {
                            0.0
                        }),
                        tip_window_type: NumberOrString::Number(f64::from(*tip_window_type)),
                    },
                ),
                Operation::Notice,
                group_id.clone(),
            )
        }
        GroupCommand::NoticeDelete {
            group_id,
            notice_id,
            ..
        } => (
            "_del_group_notice",
            serde_json::to_value(crate::core::contracts::group_content::DeleteNoticePayload {
                group_id: group_id.to_string(),
                notice_id: notice_id.to_string(),
            }),
            Operation::NoticeDelete,
            group_id.clone(),
        ),
        GroupCommand::EssenceRemove {
            group_id,
            sequence,
            random,
            ..
        } => {
            let integer = |value: &str| {
                crate::core::domain::parse_js_integer(value)
                    .map(|v| v.to_string())
                    .ok_or_else(|| {
                        AppError::new(
                            "MESSAGE_INTEGER_RANGE",
                            "Message sequence and random must be exact native integers.",
                            2,
                        )
                    })
            };
            (
                "delete_essence_msg",
                serde_json::to_value(
                    crate::core::contracts::group_content::DeleteEssencePayload {
                        group_id: Some(group_id.to_string()),
                        message_id: None,
                        msg_seq: Some(integer(sequence)?),
                        msg_random: Some(integer(random)?),
                    },
                ),
                Operation::EssenceRemove,
                group_id.clone(),
            )
        }
        GroupCommand::Sign { group_id, .. } => {
            if !crate::core::domain::is_exact_js_id(group_id) {
                return Err(AppError::new(
                    "PACKET_ID_RANGE",
                    "The group ID must be exactly representable by the upstream numeric route.",
                    2,
                ));
            }
            (
                "set_group_sign",
                serde_json::to_value(crate::core::contracts::group_content::GroupPayload {
                    group_id: group_id.to_string(),
                }),
                Operation::Sign,
                group_id.clone(),
            )
        }
        GroupCommand::Poke {
            group_id, user_id, ..
        } => (
            "group_poke",
            serde_json::to_value(
                crate::core::contracts::interaction::PokePayload::for_target(
                    Some(group_id.to_string()),
                    user_id.to_string(),
                )?,
            ),
            Operation::Poke,
            group_id.clone(),
        ),
        GroupCommand::Rename { group_id, name, .. } => (
            "set_group_name",
            serde_json::to_value(RenamePayload {
                group_id: group_id.to_string(),
                group_name: name.to_string(),
            }),
            Operation::Rename,
            group_id.clone(),
        ),
        GroupCommand::Card {
            group_id,
            user_id,
            card,
            ..
        } => (
            "set_group_card",
            serde_json::to_value(CardPayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                card: Some(card.to_string()),
            }),
            Operation::Card,
            group_id.clone(),
        ),
        GroupCommand::Admin {
            group_id,
            user_id,
            enabled,
            ..
        } => (
            "set_group_admin",
            serde_json::to_value(AdminPayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                enable: Some(BoolOrString::Bool(*enabled)),
            }),
            Operation::Admin,
            group_id.clone(),
        ),
        GroupCommand::Ban {
            group_id,
            user_id,
            duration,
            ..
        } => (
            "set_group_ban",
            serde_json::to_value(BanPayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                duration: NumberOrString::Number(f64::from(*duration)),
            }),
            Operation::Ban,
            group_id.clone(),
        ),
        GroupCommand::BanAll {
            group_id, enabled, ..
        } => (
            "set_group_whole_ban",
            serde_json::to_value(BanAllPayload {
                group_id: group_id.to_string(),
                enable: Some(BoolOrString::Bool(*enabled)),
            }),
            Operation::BanAll,
            group_id.clone(),
        ),
        GroupCommand::Kick {
            group_id,
            user_id,
            reject_add,
            ..
        } => (
            "set_group_kick",
            serde_json::to_value(KickPayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                reject_add_request: Some(BoolOrString::Bool(*reject_add)),
            }),
            Operation::Kick,
            group_id.clone(),
        ),
        GroupCommand::KickMany {
            group_id,
            user_id,
            reject_add,
            ..
        } => (
            "set_group_kick_members",
            serde_json::to_value(KickManyPayload {
                group_id: group_id.to_string(),
                user_id: user_id.iter().map(ToString::to_string).collect(),
                reject_add_request: Some(BoolOrString::Bool(*reject_add)),
            }),
            Operation::KickMany,
            group_id.clone(),
        ),
        GroupCommand::Leave { group_id, .. } => (
            "set_group_leave",
            serde_json::to_value(LeavePayload {
                group_id: group_id.to_string(),
                is_dismiss: Some(BoolOrString::Bool(false)),
            }),
            Operation::Leave,
            group_id.clone(),
        ),
        GroupCommand::Dismiss { group_id, .. } => (
            "set_group_leave",
            serde_json::to_value(LeavePayload {
                group_id: group_id.to_string(),
                is_dismiss: Some(BoolOrString::Bool(true)),
            }),
            Operation::Dismiss,
            group_id.clone(),
        ),
        GroupCommand::Title {
            group_id,
            user_id,
            title,
            ..
        } => (
            "set_group_special_title",
            serde_json::to_value(TitlePayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                special_title: title.to_string(),
            }),
            Operation::Title,
            group_id.clone(),
        ),
        GroupCommand::Avatar { group_id, file, .. } => (
            "set_group_portrait",
            serde_json::to_value(AvatarPayload {
                group_id: group_id.to_string(),
                file: super::profile_image::read(file)?.0,
            }),
            Operation::Avatar,
            group_id.clone(),
        ),
        GroupCommand::Remark {
            group_id, remark, ..
        } => (
            "set_group_remark",
            serde_json::to_value(RemarkPayload {
                group_id: group_id.to_string(),
                remark: remark.to_string(),
            }),
            Operation::Remark,
            group_id.clone(),
        ),
        GroupCommand::RobotPolicy {
            group_id,
            member_switch,
            member_examine,
            ..
        } => (
            "set_group_robot_add_option",
            serde_json::to_value(RobotPolicyPayload {
                group_id: group_id.to_string(),
                robot_member_switch: member_switch.map(f64::from),
                robot_member_examine: member_examine.map(f64::from),
            }),
            Operation::RobotPolicy,
            group_id.clone(),
        ),
        GroupCommand::JoinPolicy {
            group_id,
            add_type,
            question,
            answer,
            ..
        } => (
            "set_group_add_option",
            serde_json::to_value(JoinPolicyPayload {
                group_id: group_id.to_string(),
                add_type: f64::from(*add_type),
                group_question: question.as_ref().map(ToString::to_string),
                group_answer: answer.as_ref().map(ToString::to_string),
            }),
            Operation::JoinPolicy,
            group_id.clone(),
        ),
        GroupCommand::SearchPolicy {
            group_id,
            no_code_finger_open,
            no_finger_open,
            ..
        } => (
            "set_group_search",
            serde_json::to_value(SearchPolicyPayload {
                group_id: group_id.to_string(),
                no_code_finger_open: no_code_finger_open.map(f64::from),
                no_finger_open: no_finger_open.map(f64::from),
            }),
            Operation::SearchPolicy,
            group_id.clone(),
        ),
        GroupCommand::InvitePolicy {
            group_id, policy, ..
        } => (
            "set_group_member_invite_policy",
            serde_json::to_value(InvitePolicyPayload {
                group_id: group_id.to_string(),
                policy: *policy,
            }),
            Operation::InvitePolicy,
            group_id.clone(),
        ),
        GroupCommand::MemberPermissions {
            group_id,
            upload_album,
            temporary_session,
            create_group,
            ..
        } => (
            "set_group_member_permissions",
            serde_json::to_value(MemberPermissionsPayload {
                group_id: group_id.to_string(),
                allow_member_upload_album: *upload_album,
                allow_member_temporary_session: *temporary_session,
                allow_member_create_group: *create_group,
            }),
            Operation::MemberPermissions,
            group_id.clone(),
        ),
        GroupCommand::HistoryVisibility {
            group_id, visible, ..
        } => (
            "set_group_new_member_history_visibility",
            serde_json::to_value(HistoryVisibilityPayload {
                group_id: group_id.to_string(),
                visible: *visible,
            }),
            Operation::HistoryVisibility,
            group_id.clone(),
        ),
        _ => return Ok(None),
    };
    Ok(Some(Prepared {
        action,
        payload: payload.map_err(|_| AppError::invalid())?,
        operation,
        group_id,
    }))
}
pub(super) fn offline(command: &GroupCommand) -> Result<Option<Envelope>> {
    if let Some(result) = requests::offline(command)? {
        return Ok(Some(result));
    }
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let p = prepare(command)?.ok_or_else(AppError::invalid)?;
    Ok(Some(Envelope::typed(Draft {
        operation: p.operation,
        group_id: p.group_id,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &GroupCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if let Some(result) = requests::execute(command, session, store, references, deadline)? {
        return Ok(Some(result));
    }
    if let Some(result) = content::execute(command, session, deadline)? {
        return Ok(Some(result));
    }
    let Some(p) = prepare(command)? else {
        return read::execute(command, session, deadline);
    };
    let w = command.write().ok_or_else(AppError::invalid)?;
    let args = ApiCallArgs {
        action: p.action.parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: w.request_id.clone(),
        dry_run: false,
        deadline: crate::cli::args::DeadlineArgs {
            timeout: w.deadline.timeout,
        },
    };
    let (result, scope) =
        super::api::execute_prepared(&args, p.payload, session, store, references, deadline)?
            .into_api_result()?;
    Ok(Some(
        Envelope::typed(Receipt {
            operation: p.operation,
            group_id: p.group_id,
            request_id: LocalId::new(
                result
                    .request_id
                    .as_deref()
                    .ok_or_else(AppError::protocol)?,
            )?,
            outcome: result.outcome.ok_or_else(AppError::protocol)?,
            deduplicated: result.deduplicated,
        })
        .scoped(scope),
    ))
}
