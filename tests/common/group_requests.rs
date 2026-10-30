use serde_json::{Value, json};
pub struct Case {
    #[cfg(windows)]
    pub leaf: &'static str,
    pub args: Vec<String>,
    pub action: &'static str,
    pub payload: Value,
    #[cfg(windows)]
    pub data: Value,
    pub write: bool,
    #[cfg(windows)]
    pub rejected: bool,
    #[cfg(windows)]
    pub scan: usize,
}
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            #[cfg(windows)]
            leaf: "requests",
            args: vec![
                "group".into(),
                "requests".into(),
                "--limit".into(),
                "2".into(),
            ],
            action: "get_group_system_msg",
            payload: json!({"count":2.0}),
            #[cfg(windows)]
            data: json!({"invited_requests":[{"request_id":1001,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}],"InvitedRequest":[{"request_id":1001,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}],"join_requests":[{"request_id":1002,"group_id":20002,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}]}),
            write: false,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "requests",
            args: vec![
                "group".into(),
                "requests".into(),
                "--source".into(),
                "legacy-notices".into(),
                "--limit".into(),
                "1".into(),
            ],
            action: "get_group_ignored_notifies",
            payload: json!({}),
            #[cfg(windows)]
            data: json!({"invited_requests":[{"request_id":1001,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}],"InvitedRequest":[{"request_id":1001,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}],"join_requests":[{"request_id":1003,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}]}),
            write: false,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "requests",
            args: vec![
                "group".into(),
                "requests".into(),
                "--source".into(),
                "doubt".into(),
                "--group-id".into(),
                "20001".into(),
            ],
            action: "get_group_ignore_add_request",
            payload: json!({}),
            #[cfg(windows)]
            data: json!([{"request_id":9007199254740992_u64,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"},{"request_id":1002,"group_id":20002,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}]),
            write: false,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "respond",
            args: vec![
                "group".into(),
                "respond".into(),
                "synthetic-exact-flag".into(),
                "--approve".into(),
                "true".into(),
            ],
            action: "set_group_add_request",
            payload: json!({"flag":"synthetic-exact-flag","approve":true,"count":100.0}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "respond",
            args: vec![
                "group".into(),
                "respond".into(),
                "synthetic-refuse-flag".into(),
                "--approve".into(),
                "false".into(),
                "--reason=".into(),
                "--search-limit".into(),
                "20".into(),
            ],
            action: "set_group_add_request",
            payload: json!({"flag":"synthetic-refuse-flag","approve":false,"reason":"","count":20.0}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "respond",
            args: vec![
                "group".into(),
                "respond".into(),
                "synthetic-missing-flag".into(),
                "--approve".into(),
                "false".into(),
            ],
            action: "set_group_add_request",
            payload: json!({"flag":"synthetic-missing-flag","approve":false,"count":100.0}),
            #[cfg(windows)]
            data: json!(null),
            write: true,
            #[cfg(windows)]
            rejected: true,
            #[cfg(windows)]
            scan: 1000,
        },
        Case {
            #[cfg(windows)]
            leaf: "requests",
            args: vec![
                "group".into(),
                "requests".into(),
                "--source".into(),
                "doubt".into(),
            ],
            action: "get_group_ignore_add_request",
            payload: json!({}),
            #[cfg(windows)]
            data: json!([{"request_id":1001,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"},{"request_id":1002,"group_id":20001,"group_name":"Synthetic","invitor_uin":10002,"requester_nick":"Synthetic","message":"synthetic message","actor":0,"checked":false,"unreviewed":"synthetic-hidden-field"}]),
            write: false,
            #[cfg(windows)]
            rejected: false,
            #[cfg(windows)]
            scan: 1,
        },
    ]
}
