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

pub fn event_stream() -> Value {
    let event_id = json!({"type":"string","pattern":"^[a-f0-9]{64}$"});
    let scope = scope_schema();
    let generated = crate::core::contracts::schema::<crate::core::contracts::message::Message>();
    let generated = serde_json::to_value(generated).expect("schema serializes");
    let message = generated.clone();
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
        json!({"type":{"const":"ready"},"schemaVersion":{"const":1},"scope":scope,"selfMessages":{"type":["boolean","null"]},"coverage":{"const":"best_effort"},"checkpoint":reference("w1")}),
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
        json!({"type":{"const":"summary"},"reason":text(),"emitted":count(),"duplicates":count(),"filtered":count(),"heartbeats":count(),"frames":count(),"gaps":count(),"bytes":count(),"coverage":{"const":"best_effort"},"errorCode":text(),"checkpoint":reference("w1"),"recovery":object(&["attempts","scanned","emitted","sinceSeconds","stopReason","historyComplete","coverage"],json!({"attempts":count(),"scanned":count(),"emitted":count(),"sinceSeconds":time(),"stopReason":text(),"historyComplete":{"type":"boolean"},"coverage":{"const":"backend_accessible"}}))}),
    );
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":generated["$defs"],"oneOf":[
        ready,
        object(&["type","eventId","direction","message"],json!({"type":{"const":"message"},"eventId":event_id,"direction":{"enum":["incoming","outgoing"]},"message":message,"source":{"enum":["live","history"]},"checkpoint":reference("w1")})),
        object(&["type","eventId","conversation","supported"],json!({"type":{"const":"notice"},"eventId":event_id,"conversation":nullable_conversation,"supported":{"const":false},"checkpoint":reference("w1")})),
        object(&["type","eventId","conversation","supported","notice"],json!({"type":{"const":"notice"},"eventId":event_id,"conversation":conversation(),"supported":{"const":true},"notice":{"oneOf":notices},"checkpoint":reference("w1")})),
        object(&["type","reason","recovered","coverage"],json!({"type":{"const":"gap"},"reason":text(),"recovered":{"const":false},"coverage":{"const":"unknown"}})),
        summary
    ]})
}
