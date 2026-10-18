//! Output contracts shared by command discovery; no state or network access.
use serde_json::{Value, json};

fn object(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","required":required,"properties":properties,"additionalProperties":false})
}
fn id() -> Value {
    json!({"type":"string","pattern":"^[1-9][0-9]{0,31}$"})
}
fn count() -> Value {
    json!({"type":"integer","minimum":0})
}
fn text() -> Value {
    json!({"type":"string"})
}
fn reference(prefix: &str) -> Value {
    json!({"type":"string","pattern":format!("^{prefix}\\.[A-Za-z0-9_-]{{22}}\\.[A-Za-z0-9_-]{{43}}$")})
}
fn conversation() -> Value {
    json!({"type":"string","pattern":"^(private|group):[1-9][0-9]{0,31}$"})
}
fn time() -> Value {
    json!({"type":"integer","minimum":0,"maximum":253402300799_u64})
}

pub fn error_envelope() -> Value {
    object(
        &["schemaVersion", "ok", "error"],
        json!({"schemaVersion":{"const":1},"ok":{"const":false},"error":object(&["code","message","retrySafe"],json!({"code":text(),"message":text(),"retrySafe":{"type":"boolean"},"outcome":{"enum":["pending","accepted","rejected","unknown"]},"requestId":text(),"nextAction":{"type":"object"},"details":{}})),"meta":object(&["scope"],json!({"scope":scope_schema()}))}),
    )
}

pub fn status_response() -> Value {
    let observation = object(
        &[
            "phase",
            "isLogin",
            "isOffline",
            "qrLoginAccepted",
            "coreReady",
        ],
        json!({
            "phase":{"enum":["waiting_qrcode","generating_qrcode","qrcode_scanned","initializing","ready","offline","reconnecting","unknown"]},
            "isLogin":{"type":"boolean"},"isOffline":{"type":"boolean"},"qrLoginAccepted":{"type":"boolean"},"coreReady":{"type":"boolean"}
        }),
    );
    let unverified = object(
        &["state", "account", "identityVerified"],
        json!({
            "state":{"enum":["not_started","preparing","prepared","starting","login_required","stopped","stopping","offline","failed","unknown"]},
            "account":{"type":"null"},"identityVerified":{"const":false},"loginObservation":observation
        }),
    );
    let identified = object(
        &["state", "online", "version"],
        json!({
            "state":{"enum":["ready","offline","account_changed"]},"online":{"type":"boolean"},"version":text(),"nickname":text()
        }),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[
        object(&["schemaVersion","ok","data"],json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":unverified})),
        scoped_success(identified),error_envelope()
    ]})
}

pub fn stop_response() -> Value {
    let data = object(
        &["instanceId", "state", "reused"],
        json!({
            "instanceId":{"type":"string","pattern":"^[A-Za-z0-9_-]{1,128}$"},
            "state":{"const":"stopped"},"reused":{"type":"boolean"}
        }),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[
        object(&["schemaVersion","ok","data"],json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":data})),error_envelope()
    ]})
}

pub fn prune_response() -> Value {
    let data = object(
        &[
            "cache",
            "removedFiles",
            "removedBytes",
            "removedEntries",
            "preservedEntries",
            "retentionSeconds",
        ],
        json!({"cache":{"const":"attachments"},"removedFiles":count(),"removedBytes":count(),"removedEntries":count(),"preservedEntries":count(),"retentionSeconds":{"const":86400}}),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[object(&["schemaVersion","ok","data"],json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":data})),error_envelope()]})
}

pub fn media_response() -> Value {
    let data = object(
        &["mediaRef", "kind", "sizeBytes", "sha256"],
        json!({"mediaRef":reference("f1"),"kind":{"enum":["image","file"]},"sizeBytes":{"type":"integer","minimum":0,"maximum":134217728},"sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"}}),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[scoped_success(data),error_envelope()]})
}

fn fingerprint_schema() -> Value {
    object(
        &["kind", "name", "sizeBytes", "sha256"],
        json!({"kind":{"enum":["image","file"]},"name":{"type":"string","minLength":1,"maxLength":512},"sizeBytes":{"type":"integer","minimum":1,"maximum":134217728},"sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"}}),
    )
}
fn attachment_array() -> Value {
    json!({"type":"array","minItems":1,"maxItems":16,"items":fingerprint_schema()})
}
fn receipt_schema() -> Value {
    let mut value = object(
        &["messageId", "conversation"],
        json!({"messageId":{"type":"string","maxLength":20,"pattern":"^-?[1-9][0-9]*$"},"conversation":conversation(),"attachments":attachment_array(),"fileTransfer":{"const":"not_observable"}}),
    );
    value["if"] = json!({"required":["attachments"]});
    value["then"] = json!({"required":["fileTransfer"]});
    value["else"] = json!({"not":{"required":["fileTransfer"]}});
    value
}
fn scoped_success(data: Value) -> Value {
    object(
        &["schemaVersion", "ok", "data", "meta"],
        json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":data,"meta":object(&["scope"],json!({"scope":scope_schema()}))}),
    )
}

pub fn write_response(path: &str) -> Option<Value> {
    if !matches!(path, "message.send" | "message.reply" | "request.get") {
        return None;
    }
    let key = json!({"type":"string","pattern":"^[A-Za-z0-9_-]{1,128}$"});
    if path == "request.get" {
        let receipts = json!({"anyOf":[receipt_schema(),object(&["attachments","fileTransfer"],json!({"attachments":attachment_array(),"fileTransfer":{"const":"unknown"}})),object(&["code","attempted"],json!({"code":text(),"attempted":{"type":"boolean"}}))]});
        let mut data = object(
            &["requestId", "operation", "outcome"],
            json!({"requestId":key,"operation":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_.-]+$"},"outcome":{"enum":["pending","accepted","rejected","unknown"]},"receipt":{"type":"object"}}),
        );
        data["allOf"] = json!([{"if":{"properties":{"operation":{"enum":["send","reply"]}}},"then":{"properties":{"receipt":receipts},"allOf":[{"if":{"properties":{"outcome":{"const":"accepted"}}},"then":{"required":["receipt"],"properties":{"receipt":receipt_schema()}}}]}}]);
        let start_data = object(
            &[
                "requestId",
                "operation",
                "outcome",
                "instanceId",
                "runId",
                "instanceState",
                "identityVerified",
                "nextAction",
            ],
            json!({"requestId":key,"operation":{"const":"napcat.start"},"outcome":{"enum":["pending","dispatched","unknown"]},
            "instanceId":key,"runId":key,"instanceState":{"enum":["unregistered","changed","preparing","prepared","starting","login_required","ready","stopping","stopped","offline","failed","unknown"]},
            "identityVerified":{"const":false},"nextAction":{"oneOf":[
                object(&["command","args"],json!({"command":{"const":"status"},"args":object(&["instance"],json!({"instance":key}))})),
                object(&["command","args"],json!({"command":{"const":"napcat.instances"},"args":object(&[],json!({}))}))
            ]}}),
        );
        let start_success = object(
            &["schemaVersion", "ok", "data"],
            json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":start_data}),
        );
        return Some(
            json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[scoped_success(data),start_success,error_envelope()]}),
        );
    }
    let operation = if path == "message.reply" {
        "reply"
    } else {
        "send"
    };
    let accepted = object(
        &["requestId", "outcome", "cached", "receipt"],
        json!({"requestId":key,"outcome":{"const":"accepted"},"cached":{"type":"boolean"},"receipt":receipt_schema()}),
    );
    let dry = object(
        &[
            "dryRun",
            "operation",
            "conversation",
            "segmentCount",
            "reply",
            "attachments",
        ],
        json!({"dryRun":{"const":true},"operation":{"const":operation},"conversation":conversation(),"segmentCount":{"type":"integer","minimum":1,"maximum":81},"reply":{"const":path=="message.reply"},"attachments":{"type":"array","maxItems":16,"items":fingerprint_schema()}}),
    );
    Some(
        json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[scoped_success(accepted),scoped_success(dry),error_envelope()]}),
    )
}

fn scope_schema() -> Value {
    object(
        &["instanceId", "account"],
        json!({"instanceId":{"type":"string","pattern":"^[A-Za-z0-9_-]{1,128}$"},"account":id()}),
    )
}

pub fn read_response(path: &str) -> Option<Value> {
    let (item, coverage, paged) = match path {
        "contact.list" | "contact.get" => (
            object(
                &["userId", "conversation"],
                json!({"userId":id(),"conversation":{"type":"string","pattern":"^private:[1-9][0-9]{0,31}$"},"nickname":text(),"remark":text()}),
            ),
            "friends",
            path.ends_with("list"),
        ),
        "group.list" | "group.get" => (
            object(
                &["groupId", "conversation"],
                json!({"groupId":id(),"conversation":{"type":"string","pattern":"^group:[1-9][0-9]{0,31}$"},"name":text(),"remark":text(),"memberCount":count(),"maxMemberCount":count()}),
            ),
            "groups",
            path.ends_with("list"),
        ),
        "group.members" => (
            object(
                &["userId"],
                json!({"userId":id(),"nickname":text(),"card":text(),"role":text()}),
            ),
            "members",
            true,
        ),
        "conversation.list" | "conversation.get" => (
            object(
                &["conversation"],
                json!({"conversation":conversation(),"name":text(),"remark":text(),"lastActivitySeconds":time()}),
            ),
            "recent",
            path.ends_with("list"),
        ),
        _ => return None,
    };
    let mut properties = json!({"schemaVersion":{"const":1},"ok":{"const":true},"data":if paged {json!({"type":"array","maxItems":1000,"items":item})} else {item},"meta":object(&["scope"],json!({"scope":scope_schema()}))});
    let mut required = vec!["schemaVersion", "ok", "data", "meta"];
    if paged {
        let mut page = object(
            &["complete", "stopReason", "coverage", "nextCursor"],
            json!({"complete":{"type":"boolean"},"stopReason":{"enum":["limit","backend_end"]},"coverage":{"const":coverage},"nextCursor":{"anyOf":[reference("c1"),{"type":"null"}]}}),
        );
        page["if"] = json!({"properties":{"complete":{"const":true}}});
        page["then"] = json!({"properties":{"nextCursor":{"type":"null"},"stopReason":{"const":"backend_end"}}});
        page["else"] =
            json!({"properties":{"nextCursor":reference("c1"),"stopReason":{"const":"limit"}}});
        properties["pageInfo"] = page;
        required.push("pageInfo");
    }
    Some(
        json!({"$schema":"https://json-schema.org/draft/2020-12/schema","oneOf":[object(&required,properties),error_envelope()]}),
    )
}

fn message_contract() -> (Value, Value, Value) {
    let message = object(
        &[
            "messageRef",
            "id",
            "conversation",
            "sender",
            "timeSeconds",
            "segments",
        ],
        json!({"messageRef":reference("m1"),"id":{"type":"string","pattern":"^-?[0-9]+$"},"conversation":conversation(),"sender":object(&["userId"],json!({"userId":id(),"nickname":text()})),"timeSeconds":time(),"segments":{"type":"array","maxItems":1000,"items":{"$ref":"#/$defs/segment"}}}),
    );
    let media = object(
        &["type", "downloadable"],
        json!({"type":{"enum":["image","file"]},"downloadable":{"type":"boolean"},"name":text(),"sizeBytes":count(),"mediaRef":reference("f1")}),
    );
    let mut media = media;
    media["if"] = json!({"properties":{"downloadable":{"const":true}}});
    media["then"] = json!({"required":["mediaRef"]});
    let segment = json!({"oneOf":[
        object(&["type","text"],json!({"type":{"const":"text"},"text":text()})),
        object(&["type","userId"],json!({"type":{"const":"mention"},"userId":id()})),
        object(&["type","supported"],json!({"type":{"const":"mention"},"supported":{"const":false}})),
        object(&["type"],json!({"type":{"const":"reply"},"messageRef":reference("m1")})),
        media,
        object(&["type","expanded"],json!({"type":{"const":"forward"},"expanded":{"type":"boolean"},"nodes":{"type":"array","maxItems":100,"items":{"$ref":"#/$defs/node"}}})),
        object(&["type","supported"],json!({"type":{"type":"string","not":{"enum":["text","mention","reply","image","file","forward"]}},"supported":{"const":false}}))
    ]});
    let node = object(
        &["segments"],
        json!({"segments":{"type":"array","maxItems":1000,"items":{"$ref":"#/$defs/segment"}},"senderId":{"type":"string","pattern":"^-?[0-9]+$"},"nickname":text()}),
    );
    (message, segment, node)
}

pub fn message_response(path: &str) -> Option<Value> {
    if !matches!(path, "message.get" | "message.history" | "message.search") {
        return None;
    }
    let (message, segment, node) = message_contract();
    let success = if path == "message.get" {
        scoped_success(message)
    } else {
        let mut page = object(
            &[
                "complete",
                "scanned",
                "backendPages",
                "stopReason",
                "coverage",
                "order",
            ],
            json!({"complete":{"type":"boolean"},"scanned":count(),"backendPages":count(),"stopReason":{"enum":["limit","backend_end","range_satisfied","deadline","scan_limit","page_limit","byte_limit","backend_error","no_progress"]},"coverage":{"const":"backend_accessible"},"order":{"const":"newest_first"},"nextCursor":reference("h1"),"interruptionCode":text()}),
        );
        page["if"] = json!({"properties":{"complete":{"const":true}}});
        page["then"] = json!({"properties":{"stopReason":{"enum":["backend_end","range_satisfied"]}},"not":{"required":["nextCursor"]}});
        page["else"] = json!({"required":["nextCursor"],"properties":{"stopReason":{"enum":["limit","deadline","scan_limit","page_limit","byte_limit","backend_error","no_progress"]}}});
        let mut envelope = object(
            &["schemaVersion", "ok", "data", "meta", "pageInfo"],
            json!({"schemaVersion":{"const":1},"ok":{"type":"boolean"},"data":{"type":"array","maxItems":1000,"items":message},"meta":object(&["scope"],json!({"scope":scope_schema()})),"pageInfo":page,"error":object(&["code","message","retrySafe"],json!({"code":{"const":"PARTIAL_RESULT"},"message":text(),"retrySafe":{"const":false}}))}),
        );
        envelope["if"] = json!({"properties":{"ok":{"const":true}}});
        envelope["then"] = json!({"not":{"required":["error"]}});
        envelope["else"] = json!({"required":["error"],"properties":{"pageInfo":{"properties":{"complete":{"const":false}}}}});
        envelope
    };
    Some(
        json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"segment":segment,"node":node},"oneOf":[success,error_envelope()]}),
    )
}

pub fn event_stream() -> Value {
    let event_id = json!({"type":"string","pattern":"^[a-f0-9]{64}$"});
    let scope = scope_schema();
    let (message, segment, node) = message_contract();
    let mut notices = Vec::new();
    for kind in [
        "group_recall",
        "friend_recall",
        "friend_add",
        "group_increase",
        "group_decrease",
        "group_admin",
    ] {
        let mut fields = json!({"kind":{"const":kind},"timeSeconds":time(),"userId":if kind.starts_with("group_"){json!({"anyOf":[id(),{"type":"null"}]})}else{id()}});
        let mut required = vec!["kind", "timeSeconds", "userId"];
        if kind.ends_with("recall") {
            fields["messageId"] = json!({"type":"string","pattern":"^-?[0-9]+$"});
            required.push("messageId");
        }
        if matches!(kind, "group_recall" | "group_increase" | "group_decrease") {
            fields["operatorId"] = json!({"anyOf":[id(),{"type":"null"}]});
            required.push("operatorId");
        }
        if matches!(kind, "group_increase" | "group_decrease" | "group_admin") {
            fields["subType"] = match kind {
                "group_increase" => json!({"enum":["approve","invite"]}),
                "group_decrease" => json!({"enum":["leave","kick","kick_me"]}),
                _ => json!({"enum":["set","unset"]}),
            };
            required.push("subType");
        }
        notices.push(object(&required, fields));
    }
    let nullable_conversation = json!({"anyOf":[conversation(),{"type":"null"}]});
    let ready = object(
        &["type", "schemaVersion", "scope", "selfMessages", "coverage"],
        json!({"type":{"const":"ready"},"schemaVersion":{"const":1},"scope":scope,"selfMessages":{"type":["boolean","null"]},"coverage":{"const":"best_effort"}}),
    );
    let summary = object(
        &[
            "type",
            "reason",
            "emitted",
            "duplicates",
            "filtered",
            "heartbeats",
            "frames",
            "gaps",
            "bytes",
            "coverage",
        ],
        json!({"type":{"const":"summary"},"reason":text(),"emitted":count(),"duplicates":count(),"filtered":count(),"heartbeats":count(),"frames":count(),"gaps":count(),"bytes":count(),"coverage":{"const":"best_effort"},"errorCode":text()}),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"segment":segment,"node":node},"oneOf":[
        ready,
        object(&["type","eventId","direction","message"],json!({"type":{"const":"message"},"eventId":event_id,"direction":{"enum":["incoming","outgoing"]},"message":message})),
        object(&["type","eventId","conversation","supported"],json!({"type":{"const":"notice"},"eventId":event_id,"conversation":nullable_conversation,"supported":{"const":false}})),
        object(&["type","eventId","conversation","supported","notice"],json!({"type":{"const":"notice"},"eventId":event_id,"conversation":conversation(),"supported":{"const":true},"notice":{"oneOf":notices}})),
        object(&["type","reason","recovered","coverage"],json!({"type":{"const":"gap"},"reason":text(),"recovered":{"const":false},"coverage":{"const":"unknown"}})),
        summary
    ]})
}
