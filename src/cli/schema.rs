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
    serde_json::to_value(crate::core::contracts::schema::<
        crate::core::contracts::reply::Failure,
    >())
    .expect("schema serializes")
}
fn scope_schema() -> Value {
    serde_json::to_value(crate::core::contracts::schema::<crate::core::domain::Scope>())
        .expect("schema serializes")
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
