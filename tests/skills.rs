use std::process::Command;

use qq_cli::{cli::registry, cli::skill};
use serde_json::Value;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .args(["--config", "missing-synthetic-config.json"])
        .env_remove("QQ_CLI_CONFIG")
        .env_remove("LOCALAPPDATA")
        .env_remove("HOME")
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap()
}

#[test]
fn task_discovery_is_offline_and_every_routed_command_resolves_from_the_registry() {
    let output = run(&["skill", "list"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tasks = envelope["data"].as_array().unwrap();
    assert_eq!(tasks.len(), 10);
    for task in tasks {
        let id = task["task"].as_str().unwrap();
        let output = run(&["skill", "get", id]);
        assert!(output.status.success());
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["data"]["task"], id);
        let guide = skill::get(id).unwrap();
        for command in guide.commands {
            assert!(registry::get(command).is_ok());
        }
        assert!(guide.instructions.len() < 9000);
    }
    let output = run(&["skill", "get", "missing-synthetic-task"]);
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "TASK_NOT_FOUND");
}

#[test]
fn exported_skill_is_self_contained_and_existing_targets_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("qq-cli");
    let result = run(&["skill", "export", "--output", output.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert!(result.stderr.is_empty());
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["data"]["files"], 13);
    let entry = std::fs::read_to_string(output.join("SKILL.md")).unwrap();
    assert!(entry.starts_with("---\nname: qq-cli\n"));
    // Validate the exported routing graph, rather than just matching its prose.
    let mut pending = vec![output.join("SKILL.md")];
    let mut visited = std::collections::HashSet::new();
    while let Some(path) = pending.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap();
        for part in body.split("](").skip(1) {
            let target = part.split(')').next().unwrap();
            assert!(
                !target.contains("://"),
                "skill should not depend on hosted instructions"
            );
            let target = path.parent().unwrap().join(target);
            assert!(target.is_file());
            pending.push(target);
        }
    }
    assert_eq!(visited.len(), 13);
    std::fs::write(output.join("user-note.txt"), b"synthetic user file").unwrap();
    let result = run(&["skill", "export", "--output", output.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        std::fs::read(output.join("user-note.txt")).unwrap(),
        b"synthetic user file"
    );
    assert_eq!(
        std::fs::read_to_string(output.join("SKILL.md")).unwrap(),
        entry
    );
    assert_eq!(
        std::fs::read_dir(temp.path()).unwrap().count(),
        1,
        "no staging directory should remain"
    );
}

#[test]
fn task_routes_cover_every_business_command_and_stream_rules_are_precise() {
    let mut covered = std::collections::HashSet::new();
    for task in skill::list() {
        let guide = skill::get(task.task).unwrap();
        for command in guide.commands {
            covered.insert(command.to_string());
        }
    }
    for command in registry::list() {
        let name = command.name.as_str();
        if !name.starts_with("schema.") && !name.starts_with("skill.") {
            assert!(covered.contains(name), "No task route: {name}");
        }
    }
    let events = skill::get("events").unwrap();
    let guide = events.instructions.as_str();
    for term in [
        "ready.scope",
        "original selected instance",
        "no pageInfo",
        "--resume",
        "24 hours",
        "256",
    ] {
        assert!(guide.contains(term), "Missing stream contract: {term}");
    }
}
