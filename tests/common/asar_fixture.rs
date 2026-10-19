use std::path::Path;
pub fn write(app: &Path, leaf: serde_json::Value, payload: &[u8]) {
    let tree = serde_json::json!({"files":{"app_launcher":{"files":{"index.js":leaf}}}});
    let json = serde_json::to_vec(&tree).unwrap();
    let padded = (json.len() + 3) & !3;
    let mut bytes = Vec::new();
    for n in [
        4,
        (padded + 8) as u32,
        (padded + 4) as u32,
        json.len() as u32,
    ] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(json);
    bytes.resize(16 + padded, 0);
    bytes.extend(payload);
    std::fs::write(app.join("application.asar"), bytes).unwrap();
    std::fs::write(app.join("package.json"), r#"{"version":"9.9.99","buildVersion":"60001","main":"./application.asar/app_launcher/index.js"}"#).unwrap();
}
