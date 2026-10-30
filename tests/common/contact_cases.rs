use serde_json::{Value, json};
pub struct Case {
    pub path: &'static str,
    pub args: Vec<String>,
    pub action: &'static str,
    pub payload: Value,
    #[cfg(windows)]
    pub data: Value,
    pub write: bool,
    #[cfg(windows)]
    pub rejected: bool,
}
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            path: "contact.friend-requests",
            args: vec!["contact".into(), "friend-requests".into()],
            action: "qq_cli_get_friend_requests",
            payload: json!({}),
            #[cfg(windows)]
            data: json!({"requests":[{"flag":"100","uid":"u_synthetic","nickname":"Synthetic","message":"synthetic message"}],"complete":false}),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.profile",
            args: vec![
                "contact".into(),
                "profile".into(),
                "10002".into(),
                "--no-cache".into(),
            ],
            action: "get_stranger_info",
            payload: json!({"user_id":"10002","no_cache":true}),
            #[cfg(windows)]
            data: json!({"user_id":"10002","uid":"u_synthetic","nickname":"Synthetic","long_nick":"note","remark":"remark","sex":"unknown","qid":"synthetic","age":20,"qqLevel":3,"status":10,"reg_time":100,"is_vip":false,"is_years_vip":false,"vip_level":0,"login_days":0,"arbitrary":{"secret":"synthetic-hidden-field"}}),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.categories",
            args: vec!["contact".into(), "categories".into()],
            action: "get_friends_with_category",
            payload: json!({}),
            #[cfg(windows)]
            data: json!([{"categoryId":1,"categoryName":"Synthetic","categoryMbCount":2,"buddyList":[{"user_id":"10002","nickname":"Allowed","remark":"A"},{"user_id":"10003","nickname":"Filtered"}]}]),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.unilateral",
            args: vec!["contact".into(), "unilateral".into()],
            action: "get_unidirectional_friend_list",
            payload: json!({}),
            #[cfg(windows)]
            data: json!([{"uin":10002,"uid":"u_synthetic","nick_name":"Allowed","age":20,"source":"Synthetic"},{"uin":10003,"uid":"u_filtered","nick_name":"Filtered","age":20,"source":"Synthetic"}]),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.requests",
            args: vec![
                "contact".into(),
                "requests".into(),
                "--limit".into(),
                "2".into(),
            ],
            action: "get_doubt_friends_add_request",
            payload: json!({"count":2.0}),
            #[cfg(windows)]
            data: json!([{"flag":"u_request","uin":10002,"nick":"Allowed","reason":"synthetic reason","msg":"synthetic message","group_code":20001,"time":100},{"flag":"u_unknown","uin":0,"nick":"Filtered"}]),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.remark",
            args: vec!["contact".into(), "remark".into(), "10002".into(), "".into()],
            action: "set_friend_remark",
            payload: json!({"user_id":"10002","remark":""}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.remove",
            args: vec![
                "contact".into(),
                "remove".into(),
                "10002".into(),
                "--block".into(),
                "--both".into(),
            ],
            action: "delete_friend",
            payload: json!({"user_id":"10002","temp_block":true,"temp_both_del":true}),
            #[cfg(windows)]
            data: json!({"result":0}),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.respond",
            args: vec![
                "contact".into(),
                "respond".into(),
                "req_exact_flag".into(),
                "--approve".into(),
                "false".into(),
            ],
            action: "set_friend_add_request",
            payload: json!({"flag":"req_exact_flag","approve":false}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.respond",
            args: vec![
                "contact".into(),
                "respond".into(),
                "100".into(),
                "--approve".into(),
                "true".into(),
                "--remark".into(),
                "Synthetic".into(),
            ],
            action: "set_friend_add_request",
            payload: json!({"flag":"100","approve":true,"remark":"Synthetic"}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.accept-doubt",
            args: vec![
                "contact".into(),
                "accept-doubt".into(),
                "u_exact_flag".into(),
            ],
            action: "set_doubt_friends_add_request",
            payload: json!({"flag":"u_exact_flag","approve":true}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.remove",
            args: vec!["contact".into(), "remove".into(), "10002".into()],
            action: "delete_friend",
            payload: json!({"user_id":"10002","temp_block":false,"temp_both_del":false}),
            #[cfg(windows)]
            data: json!({"valid":false,"message":"synthetic-private-error"}),
            write: true,
            #[cfg(windows)]
            rejected: true,
        },
        Case {
            path: "contact.remove",
            args: vec!["contact".into(), "remove".into(), "10002".into()],
            action: "delete_friend",
            payload: json!({"user_id":"10002","temp_block":false,"temp_both_del":false}),
            #[cfg(windows)]
            data: json!({"result":37,"errMsg":"synthetic-private-error"}),
            write: true,
            #[cfg(windows)]
            rejected: true,
        },
        Case {
            path: "contact.status",
            args: vec!["contact".into(), "status".into(), "10002".into()],
            action: "nc_get_user_status",
            payload: json!({"user_id":"10002"}),
            #[cfg(windows)]
            data: json!({"status":10,"ext_status":0}),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.like",
            args: vec![
                "contact".into(),
                "like".into(),
                "10002".into(),
                "--times".into(),
                "2".into(),
            ],
            action: "send_like",
            payload: json!({"user_id":"10002","times":2.0}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.likes",
            args: vec![
                "contact".into(),
                "likes".into(),
                "10002".into(),
                "--expected-uid".into(),
                "u_synthetic".into(),
            ],
            action: "get_profile_like",
            payload: json!({"user_id":"10002","start":0.0,"count":10.0}),
            #[cfg(windows)]
            data: json!({"uid":"u_synthetic","time":"100","favoriteInfo":{"userInfos":[{"uid":"u_visitor","uin":0,"unreviewed":"synthetic-hidden-field"}],"total_count":10,"last_time":100,"today_count":1},"voteInfo":{"userInfos":[{"uid":"u_visitor","uin":10003}],"total_count":20,"new_count":1,"new_nearby_count":0,"last_visit_time":100}}),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.likes",
            args: vec!["contact".into(), "likes".into()],
            action: "get_profile_like",
            payload: json!({"start":0.0,"count":10.0}),
            #[cfg(windows)]
            data: json!({"uid":"u_self","time":"100","favoriteInfo":{"userInfos":[{"uid":"u_visitor","uin":0,"unreviewed":"synthetic-hidden-field"}],"total_count":10,"last_time":100,"today_count":1},"voteInfo":{"userInfos":[{"uid":"u_visitor","uin":10003}],"total_count":20,"new_count":1,"new_nearby_count":0,"last_visit_time":100}}),
            write: false,
            #[cfg(windows)]
            rejected: false,
        },
        Case {
            path: "contact.likes",
            args: vec![
                "contact".into(),
                "likes".into(),
                "10002".into(),
                "--expected-uid".into(),
                "u_synthetic".into(),
            ],
            action: "get_profile_like",
            payload: json!({"user_id":"10002","start":0.0,"count":10.0}),
            #[cfg(windows)]
            data: json!({"uid":"u_self","time":"100","favoriteInfo":{"userInfos":[{"uid":"u_visitor","uin":0,"unreviewed":"synthetic-hidden-field"}],"total_count":10,"last_time":100,"today_count":1},"voteInfo":{"userInfos":[{"uid":"u_visitor","uin":10003}],"total_count":20,"new_count":1,"new_nearby_count":0,"last_visit_time":100}}),
            write: false,
            #[cfg(windows)]
            rejected: true,
        },
    ]
}
