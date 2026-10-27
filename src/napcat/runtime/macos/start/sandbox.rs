//! Fixed launch isolation, rendered as literal profile data rather than shell code.
use super::*;
use std::path::Path;
pub(super) mod history;

pub(in crate::napcat::runtime::macos) fn profile(
    root: &Path,
    framework: &Path,
    profile_home: &Path,
) -> Result<String> {
    profile_with_history(root, framework, profile_home, &[])
}

pub(super) fn profile_with_history(
    root: &Path,
    framework: &Path,
    profile_home: &Path,
    predecessors: &[String],
) -> Result<String> {
    let quote = |path: &Path| -> Result<String> {
        let text = path.to_str().ok_or_else(conflict)?;
        if !path.is_absolute() || text.chars().any(char::is_control) {
            return Err(conflict());
        }
        serde_json::to_string(text).map_err(|_| conflict())
    };
    let app = quote(&root.join("QQ.app"))?;
    let adapter = quote(&root.join("adapter"))?;
    let state = root.parent().and_then(Path::parent).ok_or_else(conflict)?;
    if state.parent().is_none() {
        return Err(conflict());
    }
    let runtime = root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(conflict)?;
    let uploads = state.join("attachments").join("prepare-");
    let uploads = uploads.to_str().ok_or_else(conflict)?;
    // The regex filter takes a Scheme string: escape both the regex path and
    // its string representation, rather than interpolating filesystem text.
    let mut runtimes = std::collections::BTreeSet::from([runtime.to_owned()]);
    for previous in predecessors {
        crate::core::domain::request_key(previous).map_err(|_| conflict())?;
        runtimes.insert(previous.clone());
    }
    let alternatives = runtimes
        .iter()
        .map(|s| regex_literal(s))
        .collect::<Vec<_>>()
        .join("|");
    let uploads = serde_json::to_string(&format!(
        "^{}({alternatives})-[^/]+(/.*)?$",
        regex_literal(uploads)
    ))
    .map_err(|_| conflict())?;
    let state = quote(state)?;
    if profile_home
        .file_name()
        .is_none_or(|name| name != "profile-home")
        || profile_home.parent().and_then(Path::parent) != root.parent()
    {
        return Err(conflict());
    }
    let profile_home = quote(profile_home)?;
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
(deny file-read* (subpath "/Users") (subpath "/var/root") (subpath "/private/var/root") (subpath {home}) (subpath {state}))
(allow file-read* (subpath {root}) (subpath {framework}) (subpath {profile_home}))
(allow file-read* (regex {uploads}))
(allow file-read-metadata)
(deny file-write*)
(allow file-write* (subpath {root}) (subpath {profile_home}))
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

fn regex_literal(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        if "\\.^$|?*+()[]{}".contains(ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[cfg(test)]
#[path = "sandbox/tests.rs"]
mod tests;
