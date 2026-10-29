//! Original action inventory from the fixed public registration, not a call whitelist.
use crate::core::{
    contracts::api::{Description, Effect, Summary},
    input::ActionName,
};
pub const VERSION: &str = "v4.18.38";
pub const COMMIT: &str = "bbd0d2aca3d470c20b33b1d4f95123335633bf26";
pub const ACTIONS: &[&str] = &[
    "_del_group_notice",
    "_get_group_notice",
    "_get_model_show",
    "_mark_all_as_read",
    "_send_group_notice",
    "_set_model_show",
    ".handle_quick_operation",
    ".ocr_image",
    "add_custom_face",
    "ArkShareGroup",
    "ArkSharePeer",
    "bot_exit",
    "can_send_image",
    "can_send_record",
    "cancel_group_album_media_like",
    "cancel_group_todo",
    "cancel_online_file",
    "check_url_safely",
    "clean_cache",
    "clean_stream_temp_file",
    "click_inline_keyboard_button",
    "complete_group_todo",
    "create_collection",
    "create_flash_task",
    "create_group_file_folder",
    "del_group_album_media",
    "delete_custom_face",
    "delete_essence_msg",
    "delete_friend",
    "delete_group_file",
    "delete_group_folder",
    "delete_msg",
    "delete_qzone_msg",
    "do_group_album_comment",
    "download_file",
    "download_file_image_stream",
    "download_file_record_stream",
    "download_file_stream",
    "download_fileset",
    "fetch_custom_face",
    "fetch_custom_face_detail",
    "fetch_emoji_like",
    "fetch_ptt_text",
    "forward_friend_single_msg",
    "forward_group_single_msg",
    "friend_poke",
    "get_ai_characters",
    "get_ai_record",
    "get_clientkey",
    "get_collection_list",
    "get_cookies",
    "get_credentials",
    "get_csrf_token",
    "get_doubt_friends_add_request",
    "get_emoji_likes",
    "get_essence_msg_list",
    "get_file",
    "get_fileset_id",
    "get_fileset_info",
    "get_flash_file_list",
    "get_flash_file_url",
    "get_forward_msg",
    "get_friend_list",
    "get_friend_msg_history",
    "get_friends_with_category",
    "get_group_album_media_list",
    "get_group_at_all_remain",
    "get_group_detail_info",
    "get_group_file_system_info",
    "get_group_file_url",
    "get_group_files_by_folder",
    "get_group_honor_info",
    "get_group_ignore_add_request",
    "get_group_ignored_notifies",
    "get_group_info",
    "get_group_info_ex",
    "get_group_list",
    "get_group_member_info",
    "get_group_member_list",
    "get_group_msg_history",
    "get_group_root_files",
    "get_group_share_link",
    "get_group_shut_list",
    "get_group_signed_list",
    "get_group_system_msg",
    "get_guild_list",
    "get_guild_service_profile",
    "get_image",
    "get_login_info",
    "get_mini_app_ark",
    "get_msg",
    "get_online_clients",
    "get_online_file_msg",
    "get_private_file_url",
    "get_profile_like",
    "get_qun_album_list",
    "get_recent_contact",
    "get_record",
    "get_rkey",
    "get_rkey_server",
    "get_robot_uin_range",
    "get_share_link",
    "get_status",
    "get_stranger_info",
    "get_unidirectional_friend_list",
    "get_version_info",
    "group_poke",
    "mark_group_msg_as_read",
    "mark_msg_as_read",
    "mark_private_msg_as_read",
    "move_group_file",
    "nc_get_packet_status",
    "nc_get_rkey",
    "nc_get_user_status",
    "ocr_image",
    "receive_online_file",
    "refuse_online_file",
    "rename_group_file",
    "send_ark_share",
    "send_flash_msg",
    "send_forward_msg",
    "send_group_ai_record",
    "send_group_ark_share",
    "send_group_forward_msg",
    "send_group_msg",
    "send_group_sign",
    "send_like",
    "send_msg",
    "send_online_file",
    "send_online_folder",
    "send_packet",
    "send_poke",
    "send_private_forward_msg",
    "send_private_msg",
    "send_qzone_msg",
    "set_custom_face_desc",
    "set_diy_online_status",
    "set_doubt_friends_add_request",
    "set_essence_msg",
    "set_friend_add_request",
    "set_friend_remark",
    "set_group_add_option",
    "set_group_add_request",
    "set_group_admin",
    "set_group_album_media_like",
    "set_group_ban",
    "set_group_card",
    "set_group_kick",
    "set_group_kick_members",
    "set_group_leave",
    "set_group_member_invite_policy",
    "set_group_member_permissions",
    "set_group_name",
    "set_group_new_member_history_visibility",
    "set_group_portrait",
    "set_group_remark",
    "set_group_robot_add_option",
    "set_group_search",
    "set_group_sign",
    "set_group_special_title",
    "set_group_todo",
    "set_group_whole_ban",
    "set_input_status",
    "set_msg_emoji_like",
    "set_online_status",
    "set_qq_avatar",
    "set_qq_profile",
    "set_restart",
    "set_self_longnick",
    "test_download_stream",
    "trans_group_file",
    "translate_en2zh",
    "upload_file_stream",
    "upload_group_file",
    "upload_image_to_qun_album",
    "upload_images_to_qun_album",
    "upload_private_file",
    "upload_video_to_qun_album",
];

pub fn base(action: &str) -> &str {
    action
        .strip_suffix("_async")
        .or_else(|| action.strip_suffix("_rate_limited"))
        .unwrap_or(action)
}
pub fn effect(action: &str) -> Effect {
    match base(action) {
        "ArkSharePeer"
        | "ArkShareGroup"
        | "send_ark_share"
        | "send_group_ark_share"
        | "get_mini_app_ark" => Effect::Read,
        "click_inline_keyboard_button" | ".handle_quick_operation" => Effect::Write,
        "delete_msg"
        | "mark_msg_as_read"
        | "mark_private_msg_as_read"
        | "mark_group_msg_as_read"
        | "_mark_all_as_read"
        | "forward_friend_single_msg"
        | "forward_group_single_msg"
        | "send_msg"
        | "send_forward_msg"
        | "send_group_forward_msg"
        | "send_private_forward_msg" => Effect::Write,
        "set_group_add_request" => Effect::Write,
        "get_group_system_msg" | "get_group_ignored_notifies" | "get_group_ignore_add_request" => {
            Effect::Read
        }
        "get_group_member_info"
        | "get_group_detail_info"
        | "get_group_info_ex"
        | "get_group_share_link"
        | "get_group_honor_info"
        | "get_group_shut_list"
        | "get_group_at_all_remain" => Effect::Read,
        "get_friends_with_category"
        | "get_unidirectional_friend_list"
        | "get_stranger_info"
        | "get_doubt_friends_add_request" => Effect::Read,
        "delete_friend"
        | "set_friend_remark"
        | "set_doubt_friends_add_request"
        | "set_friend_add_request" => Effect::Write,
        "set_msg_emoji_like" | "set_input_status" | "friend_poke" | "group_poke" | "send_poke" => {
            Effect::Write
        }
        "fetch_emoji_like" | "get_emoji_likes" | "get_record" => Effect::Read,
        "fetch_ptt_text" => Effect::Write,
        "get_status"
        | "get_login_info"
        | "get_version_info"
        | "get_friend_list"
        | "get_group_list"
        | "get_group_info"
        | "get_group_member_list"
        | "get_recent_contact"
        | "get_msg"
        | "get_friend_msg_history"
        | "get_group_msg_history"
        | "get_forward_msg"
        | "get_image"
        | "get_file" => Effect::Read,
        "set_group_name"
        | "set_group_card"
        | "set_group_admin"
        | "set_group_ban"
        | "set_group_whole_ban"
        | "set_group_kick"
        | "set_group_kick_members"
        | "set_group_leave"
        | "set_group_special_title"
        | "set_group_portrait"
        | "set_group_remark"
        | "set_group_robot_add_option"
        | "set_group_add_option"
        | "set_group_search"
        | "set_group_member_invite_policy"
        | "set_group_member_permissions"
        | "set_group_new_member_history_visibility"
        | "send_private_msg"
        | "send_group_msg"
        | "set_qq_profile"
        | "set_qq_avatar"
        | "set_self_longnick"
        | "set_online_status"
        | "set_diy_online_status"
        | "send_like"
        | "_set_model_show" => Effect::Write,
        "get_online_clients" | "get_profile_like" | "nc_get_user_status" | "_get_model_show" => {
            Effect::Read
        }
        _ => Effect::Unknown,
    }
}
pub fn summary(action: &str) -> Summary {
    let effect = effect(action);
    Summary {
        action: action.into(),
        effect,
        effect_verified: effect != Effect::Unknown,
    }
}
pub fn compatibility_only(action: &str) -> bool {
    matches!(
        base(action),
        "get_online_clients" | "_get_model_show" | "_set_model_show"
    )
}
pub fn describe(action: &ActionName) -> Description {
    let base_action = base(action);
    let known = ACTIONS.contains(&base_action);
    let effect = effect(action);
    use crate::core::contracts::{account::*, discovery::SchemaDocument};
    let (parameter_schema, source_file) = match base_action {
        "ArkSharePeer" | "send_ark_share" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::card::PeerPayload,
            >()),
            Some("extends/ShareContact.ts"),
        ),
        "ArkShareGroup" | "send_group_ark_share" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::card::GroupPayload,
            >()),
            Some("extends/ShareContact.ts"),
        ),
        "get_mini_app_ark" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::card::MiniPayload,
            >()),
            Some("extends/GetMiniAppArk.ts"),
        ),
        "click_inline_keyboard_button" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::card::ButtonPayload,
            >()),
            Some("extends/ClickInlineKeyboardButton.ts"),
        ),
        ".handle_quick_operation" => (None, Some("go-cqhttp/QuickAction.ts")),
        "set_msg_emoji_like" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::interaction::ReactPayload,
            >()),
            Some("msg/SetMsgEmojiLike.ts"),
        ),
        "fetch_emoji_like" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::interaction::FetchPayload,
            >()),
            Some("extends/FetchEmojiLike.ts"),
        ),
        "get_emoji_likes" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::interaction::ListPayload,
            >()),
            Some("extends/GetEmojiLikes.ts"),
        ),
        "set_input_status" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::interaction::InputPayload,
            >()),
            Some("extends/SetInputStatus.ts"),
        ),
        "friend_poke" | "group_poke" | "send_poke" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::interaction::PokePayload,
            >()),
            Some("packet/SendPoke.ts"),
        ),
        "get_record" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::voice::RecordPayload,
            >()),
            Some("file/GetRecord.ts"),
        ),
        "fetch_ptt_text" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::voice::TextPayload,
            >()),
            Some("msg/FetchPttText.ts"),
        ),
        "delete_msg" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::MessagePayload,
            >()),
            Some("msg/DeleteMsg.ts"),
        ),
        "mark_msg_as_read" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::MarkReadPayload,
            >()),
            Some("msg/MarkMsgAsRead.ts"),
        ),
        "mark_private_msg_as_read" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::MarkReadPayload,
            >()),
            Some("msg/MarkMsgAsRead.ts"),
        ),
        "mark_group_msg_as_read" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::MarkReadPayload,
            >()),
            Some("msg/MarkMsgAsRead.ts"),
        ),
        "_mark_all_as_read" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::EmptyPayload,
            >()),
            Some("msg/MarkMsgAsRead.ts"),
        ),
        "forward_friend_single_msg" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::ForwardPayload,
            >()),
            Some("msg/ForwardSingleMsg.ts"),
        ),
        "forward_group_single_msg" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::message_ops::ForwardPayload,
            >()),
            Some("msg/ForwardSingleMsg.ts"),
        ),
        "send_msg" => (None, Some("msg/SendMsg.ts")),
        "send_forward_msg" | "send_group_forward_msg" | "send_private_forward_msg" => {
            (None, Some("go-cqhttp/SendForwardMsg.ts"))
        }
        "get_group_system_msg" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_requests::BatchPayload,
            >()),
            Some("system/GetSystemMsg.ts"),
        ),
        "get_group_ignored_notifies" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_requests::EmptyPayload,
            >()),
            Some("group/GetGroupIgnoredNotifies.ts"),
        ),
        "get_group_ignore_add_request" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_requests::EmptyPayload,
            >()),
            Some("extends/GetGroupAddRequest.ts"),
        ),
        "set_group_add_request" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_requests::RespondPayload,
            >()),
            Some("group/SetGroupAddRequest.ts"),
        ),
        "get_group_member_info" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::MemberPayload,
            >()),
            Some("group/GetGroupMemberInfo.ts"),
        ),
        "get_group_detail_info" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::GroupPayload,
            >()),
            Some("group/GetGroupDetailInfo.ts"),
        ),
        "get_group_info_ex" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::GroupPayload,
            >()),
            Some("extends/GetGroupInfoEx.ts"),
        ),
        "get_group_share_link" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::SharePayload,
            >()),
            Some("extends/GetGroupShareLink.ts"),
        ),
        "get_group_honor_info" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::HonorsPayload,
            >()),
            Some("go-cqhttp/GetGroupHonorInfo.ts"),
        ),
        "get_group_shut_list" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::GroupPayload,
            >()),
            Some("group/GetGroupShutList.ts"),
        ),
        "get_group_at_all_remain" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group_query::GroupPayload,
            >()),
            Some("go-cqhttp/GetGroupAtAllRemain.ts"),
        ),
        "send_like" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::LikePayload,
            >()),
            Some("user/SendLike.ts"),
        ),
        "nc_get_user_status" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::UserPayload,
            >()),
            Some("extends/GetUserStatus.ts"),
        ),
        "get_profile_like" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::LikesPayload,
            >()),
            Some("extends/GetProfileLike.ts"),
        ),
        "get_online_clients" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::account::EmptyPayload,
            >()),
            Some("go-cqhttp/GetOnlineClient.ts"),
        ),
        "_get_model_show" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::account::ModelPayload,
            >()),
            Some("go-cqhttp/GoCQHTTPGetModelShow.ts"),
        ),
        "_set_model_show" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::account::EmptyPayload,
            >()),
            Some("go-cqhttp/GoCQHTTPSetModelShow.ts"),
        ),
        "get_friends_with_category" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::EmptyPayload,
            >()),
            Some("extends/GetFriendWithCategory.ts"),
        ),
        "get_unidirectional_friend_list" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::EmptyPayload,
            >()),
            Some("extends/GetUnidirectionalFriendList.ts"),
        ),
        "get_stranger_info" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::ProfilePayload,
            >()),
            Some("go-cqhttp/GetStrangerInfo.ts"),
        ),
        "get_doubt_friends_add_request" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::RequestsPayload,
            >()),
            Some("new/GetDoubtFriendsAddRequest.ts"),
        ),
        "delete_friend" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::RemovePayload,
            >()),
            Some("go-cqhttp/GoCQHTTPDeleteFriend.ts"),
        ),
        "set_friend_remark" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::RemarkPayload,
            >()),
            Some("user/SetFriendRemark.ts"),
        ),
        "set_doubt_friends_add_request" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::DoubtPayload,
            >()),
            Some("new/SetDoubtFriendsAddRequest.ts"),
        ),
        "set_friend_add_request" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::contact::RespondPayload,
            >()),
            Some("user/SetFriendAddRequest.ts"),
        ),
        "set_group_name" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::RenamePayload,
            >()),
            Some("group/SetGroupName.ts"),
        ),
        "set_group_card" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::CardPayload,
            >()),
            Some("group/SetGroupCard.ts"),
        ),
        "set_group_admin" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::AdminPayload,
            >()),
            Some("group/SetGroupAdmin.ts"),
        ),
        "set_group_ban" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::BanPayload,
            >()),
            Some("group/SetGroupBan.ts"),
        ),
        "set_group_whole_ban" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::BanAllPayload,
            >()),
            Some("group/SetGroupWholeBan.ts"),
        ),
        "set_group_kick" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::KickPayload,
            >()),
            Some("group/SetGroupKick.ts"),
        ),
        "set_group_kick_members" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::KickManyPayload,
            >()),
            Some("extends/SetGroupKickMembers.ts"),
        ),
        "set_group_leave" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::LeavePayload,
            >()),
            Some("group/SetGroupLeave.ts"),
        ),
        "set_group_special_title" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::TitlePayload,
            >()),
            Some("extends/SetSpecialTitle.ts"),
        ),
        "set_group_portrait" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::AvatarPayload,
            >()),
            Some("go-cqhttp/SetGroupPortrait.ts"),
        ),
        "set_group_remark" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::RemarkPayload,
            >()),
            Some("extends/SetGroupRemark.ts"),
        ),
        "set_group_robot_add_option" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::RobotPolicyPayload,
            >()),
            Some("extends/SetGroupRobotAddOption.ts"),
        ),
        "set_group_add_option" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::JoinPolicyPayload,
            >()),
            Some("extends/SetGroupAddOption.ts"),
        ),
        "set_group_search" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::SearchPolicyPayload,
            >()),
            Some("extends/SetGroupSearch.ts"),
        ),
        "set_group_member_invite_policy" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::InvitePolicyPayload,
            >()),
            Some("group/SetGroupMemberInvitePolicy.ts"),
        ),
        "set_group_member_permissions" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::MemberPermissionsPayload,
            >()),
            Some("group/SetGroupMemberPermissions.ts"),
        ),
        "set_group_new_member_history_visibility" => (
            Some(SchemaDocument::for_type::<
                crate::core::contracts::group::HistoryVisibilityPayload,
            >()),
            Some("group/SetGroupNewMemberHistoryVisibility.ts"),
        ),
        "set_qq_profile" => (
            Some(SchemaDocument::for_type::<ProfilePayload>()),
            Some("go-cqhttp/SetQQProfile.ts"),
        ),
        "set_qq_avatar" => (
            Some(SchemaDocument::for_type::<AvatarPayload>()),
            Some("extends/SetQQAvatar.ts"),
        ),
        "set_self_longnick" => (
            Some(SchemaDocument::for_type::<SignaturePayload>()),
            Some("extends/SetLongNick.ts"),
        ),
        "set_online_status" => (
            Some(SchemaDocument::for_type::<PresencePayload>()),
            Some("extends/SetOnlineStatus.ts"),
        ),
        "set_diy_online_status" => (
            Some(SchemaDocument::for_type::<StatusTextPayload>()),
            Some("extends/SetDiyOnlineStatus.ts"),
        ),
        _ => (None, None),
    };
    Description {
        compatibility_only: compatibility_only(action),
        parameters_known: parameter_schema.is_some(), parameter_schema,
        action: action.to_string(), base_action: base_action.into(), known, effect,
        effect_verified: effect != Effect::Unknown,
        request_id_required: effect != Effect::Read,
        parameter_shape: "object",
        source: known.then(|| format!("https://github.com/NapNeko/NapCatQQ/blob/{COMMIT}/packages/napcat-onebot/action/{}", source_file.unwrap_or("index.ts"))),
    }
}
