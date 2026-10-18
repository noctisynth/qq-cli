use std::path::{Path, PathBuf};

pub const VERSION: &str = "9.9.99-60001";
pub fn pe(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0_u8; 128];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&64_u32.to_le_bytes());
    bytes[64..68].copy_from_slice(b"PE\0\0");
    bytes[68..70].copy_from_slice(&machine.to_le_bytes());
    bytes
}
pub fn fixture(root: &Path, alternate: bool) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let executable = root.join("QQ.exe");
    std::fs::write(&executable, pe(0x8664)).unwrap();
    let (config, app, major) = if alternate {
        let versions = root.join("resources/app/versions");
        let app = versions.join(VERSION);
        (
            versions.join("config.json"),
            app.clone(),
            app.join("major.node"),
        )
    } else {
        let versions = root.join("versions");
        let app = versions.join(VERSION).join("resources/app");
        (
            versions.join("config.json"),
            app,
            versions.join(VERSION).join("major.node"),
        )
    };
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(config, format!(r#"{{"curVersion":"{VERSION}"}}"#)).unwrap();
    std::fs::write(app.join("package.json"),r#"{"version":"9.9.99","buildVersion":"60001","main":"launcher.js","unused":"synthetic omitted field"}"#).unwrap();
    std::fs::write(
        app.join("launcher.js"),
        b"synthetic entry bytes; never executed",
    )
    .unwrap();
    std::fs::write(major, pe(0x8664)).unwrap();
    executable
}
