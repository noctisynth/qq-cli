use serde_json::{Value, json};
pub struct Case {
    pub path: &'static str,
    #[cfg(windows)]
    pub args: Vec<String>,
    pub action: &'static str,
    pub payload: Value,
    #[cfg(windows)]
    pub data: Value,
    #[cfg(windows)]
    pub error: Option<&'static str>,
    #[cfg(windows)]
    pub scan_limit: usize,
}
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            path: "group.member",
            #[cfg(windows)]
            args: vec![
                "group".into(),
                "member".into(),
                "20001".into(),
                "10002".into(),
            ],
            action: "get_group_member_info",
            payload: json!({"group_id":"20001","user_id":"10002","no_cache":true}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"user_id":10002,"nickname":"Synthetic","card":"Card","role":"admin","title":"Title","join_time":10,"last_sent_time":20,"shut_up_timestamp":0,"area":"","card_changeable":true,"unreviewed":"synthetic-hidden-field"}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.member",
            #[cfg(windows)]
            args: vec![
                "group".into(),
                "member".into(),
                "20001".into(),
                "10002".into(),
                "--cached".into(),
            ],
            action: "get_group_member_info",
            payload: json!({"group_id":"20001","user_id":"10002","no_cache":false}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"user_id":10002,"nickname":"Synthetic","card":"Card","role":"admin","title":"Title","join_time":10,"last_sent_time":20,"shut_up_timestamp":0,"area":"","card_changeable":true,"unreviewed":"synthetic-hidden-field"}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.detail",
            #[cfg(windows)]
            args: vec!["group".into(), "detail".into(), "20001".into()],
            action: "get_group_detail_info",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"group_name":"Synthetic","member_count":2,"max_member_count":10,"group_all_shut":-1,"group_remark":"","unreviewed":"synthetic-hidden-field"}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.extended",
            #[cfg(windows)]
            args: vec!["group".into(), "extended".into(), "20001".into()],
            action: "get_group_info_ex",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!({"groupCode":"20001","resultCode":0,"extInfo":{"groupOwnerId":{"memberUin":"10002","memberUid":"u_owner","memberQid":"synthetic"},"inviteRobotMemberSwitch":0,"inviteRobotMemberExamine":2,"luckyWord":"Synthetic","unreviewed":"synthetic-hidden-field"}}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.share",
            #[cfg(windows)]
            args: vec![
                "group".into(),
                "share".into(),
                "20001".into(),
                "--full".into(),
                "--source-id".into(),
                "99".into(),
                "--additional".into(),
                "synthetic".into(),
            ],
            action: "get_group_share_link",
            payload: json!({"group_id":"20001","need_short_url":false,"src_id":99.0,"additional_param":"synthetic"}),
            #[cfg(windows)]
            data: json!({"url":"https://example.invalid/synthetic-group"}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.honors",
            #[cfg(windows)]
            args: vec![
                "group".into(),
                "honors".into(),
                "20001".into(),
                "--kind".into(),
                "emotion".into(),
            ],
            action: "get_group_honor_info",
            payload: json!({"group_id":"20001","type":"emotion"}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"current_talkative":{},"talkative_list":[],"performer_list":[],"legend_list":[],"emotion_list":[{"user_id":"10002","nickname":"Synthetic","avatar":"https://example.invalid/avatar","description":"Synthetic","unreviewed":"synthetic-hidden-field"}],"strong_newbie_list":[]}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.muted",
            #[cfg(windows)]
            args: vec!["group".into(), "muted".into(), "20001".into()],
            action: "get_group_shut_list",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!([{"uid":"u_member","uin":"10002","nick":"Synthetic","cardName":"Card","role":3,"shutUpTime":100,"unreviewed":"synthetic-hidden-field"}]),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.mention-quota",
            #[cfg(windows)]
            args: vec!["group".into(), "mention-quota".into(), "20001".into()],
            action: "get_group_at_all_remain",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!({"can_at_all":true,"remain_at_all_count_for_group":3,"remain_at_all_count_for_uin":2}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.honors",
            #[cfg(windows)]
            args: vec!["group".into(), "honors".into(), "20001".into()],
            action: "get_group_honor_info",
            payload: json!({"group_id":"20001","type":"all"}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"current_talkative":{},"talkative_list":[],"performer_list":[],"legend_list":[],"emotion_list":[],"strong_newbie_list":[]}),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.muted",
            #[cfg(windows)]
            args: vec!["group".into(), "muted".into(), "20001".into()],
            action: "get_group_shut_list",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!([]),
            #[cfg(windows)]
            error: None,
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.member",
            #[cfg(windows)]
            args: vec![
                "group".into(),
                "member".into(),
                "20001".into(),
                "10002".into(),
            ],
            action: "get_group_member_info",
            payload: json!({"group_id":"20001","user_id":"10002","no_cache":true}),
            #[cfg(windows)]
            data: json!({"group_id":20001,"user_id":10003,"nickname":"Synthetic","card":"Card","role":"admin","title":"Title","join_time":10,"last_sent_time":20,"shut_up_timestamp":0,"area":"","card_changeable":true,"unreviewed":"synthetic-hidden-field"}),
            #[cfg(windows)]
            error: Some("BACKEND_PROTOCOL"),
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.detail",
            #[cfg(windows)]
            args: vec!["group".into(), "detail".into(), "20001".into()],
            action: "get_group_detail_info",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!({"group_id":20002,"group_name":"Synthetic","member_count":2,"max_member_count":10,"group_all_shut":0}),
            #[cfg(windows)]
            error: Some("BACKEND_PROTOCOL"),
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.extended",
            #[cfg(windows)]
            args: vec!["group".into(), "extended".into(), "20001".into()],
            action: "get_group_info_ex",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!({"groupCode":"20001","resultCode":37,"extInfo":{}}),
            #[cfg(windows)]
            error: Some("BACKEND_REJECTED"),
            #[cfg(windows)]
            scan_limit: 1000,
        },
        Case {
            path: "group.muted",
            #[cfg(windows)]
            args: vec!["group".into(), "muted".into(), "20001".into()],
            action: "get_group_shut_list",
            payload: json!({"group_id":"20001"}),
            #[cfg(windows)]
            data: json!([{"uid":"u_member","uin":"10002","nick":"Synthetic","cardName":"Card","role":3,"shutUpTime":100,"unreviewed":"synthetic-hidden-field"},{"uid":"u_member","uin":"10002","nick":"Synthetic","cardName":"Card","role":3,"shutUpTime":100,"unreviewed":"synthetic-hidden-field"}]),
            #[cfg(windows)]
            error: Some("SCAN_LIMIT"),
            #[cfg(windows)]
            scan_limit: 1,
        },
    ]
}
