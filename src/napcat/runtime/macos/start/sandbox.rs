//! Fixed launch isolation, rendered as literal profile data rather than shell code.
use super::*;
use std::path::Path;

pub(in crate::napcat::runtime::macos) fn profile(root: &Path, framework: &Path) -> Result<String> {
    let quote = |path: &Path| -> Result<String> {
        let text = path.to_str().ok_or_else(conflict)?;
        if !path.is_absolute() || text.chars().any(char::is_control) {
            return Err(conflict());
        }
        serde_json::to_string(text).map_err(|_| conflict())
    };
    let app = quote(&root.join("QQ.app"))?;
    let adapter = quote(&root.join("adapter"))?;
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .ok_or_else(conflict)?;
    if Path::new(&home).parent().is_none() {
        return Err(conflict());
    }
    let home = quote(Path::new(&home))?;
    let root = quote(root)?;
    let framework = quote(framework)?;
    // Framework executes in-process. Child executable access is confined to
    // the dedicated app; its inherited profile also confines detached children.
    Ok(format!(
        r#"(version 1)
(allow default)
(deny file-read* (subpath "/Users") (subpath "/var/root") (subpath "/private/var/root") (subpath {home}))
(allow file-read* (subpath {root}) (subpath {framework}))
(allow file-read-metadata)
(deny file-write*)
(allow file-write* (subpath {root}))
(deny file-write* (subpath {app}) (subpath {adapter}))
(deny process-exec)
(allow process-exec (subpath {app}))
(deny network*)
(allow network-outbound)
(allow network-inbound (local ip "localhost:*"))
(allow network-bind (local ip "localhost:*"))
"#
    ))
}
