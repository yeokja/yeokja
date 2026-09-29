//! End-to-end check for `yeokja reconcile`: `--dry-run` must not touch state,
//! and after a real run `status --check` must count only the genuinely new
//! (Pending) segment — no stray Stale/ContextChanged from the migration
//! itself.

use std::fs;
use std::process::Command;

fn yeokja_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yeokja"))
}

const YEOKJA_TOML: &str = r#"
[project]
source_lang = "en"
target_lang = "ko"

[[sources]]
path = "src"
pattern = "*.md"
parser = "markdown"
output = "{dir}/{stem}.ko{ext}"

[provider]
type = "claude_code"
model = "dummy"
"#;

const EMPTY_STATE: &str = r#"{"version": 1, "source_hash": 0, "segments": []}"#;

#[test]
fn a_file_without_state_is_skipped_and_gets_no_state_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("yeokja.toml"), YEOKJA_TOML).unwrap();
    fs::write(root.join("src/tracked.md"), "Hello world.\n").unwrap();
    fs::write(root.join("src/tracked.md.yeokja.json"), EMPTY_STATE).unwrap();
    fs::write(root.join("src/untracked.md"), "Never translated.\n").unwrap();

    let run = yeokja_cmd().args(["-C", root.to_str().unwrap(), "reconcile", "src"]).output().unwrap();
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);

    assert!(
        !root.join("src/untracked.md.yeokja.json").exists(),
        "reconcile must not create state for a file that had none"
    );
    assert!(stdout.contains("untracked.md: skipped (no state)"), "stdout was:\n{stdout}");
    assert!(!stdout.contains("Never translated."), "stdout was:\n{stdout}");

    let tracked: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("src/tracked.md.yeokja.json")).unwrap()).unwrap();
    assert_eq!(tracked["segments"].as_array().unwrap().len(), 1);
}

#[test]
fn dry_run_does_not_write_and_real_run_leaves_only_the_new_segment_pending() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("yeokja.toml"), YEOKJA_TOML).unwrap();

    let doc_path = root.join("src/doc.md");
    fs::write(&doc_path, "Hello world.\n\nSecond paragraph.\n").unwrap();

    // Seed state through the real parser so hashes line up: reconcile only
    // touches files that already have state, so start from an empty one.
    // Then mark both segments translated to simulate a project translated
    // before a parser change.
    let state_path = root.join("src/doc.md.yeokja.json");
    fs::write(&state_path, EMPTY_STATE).unwrap();
    let seed = yeokja_cmd().args(["-C", root.to_str().unwrap(), "reconcile", "src"]).status().unwrap();
    assert!(seed.success());

    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    for seg in state["segments"].as_array_mut().unwrap() {
        let source = seg["source"].as_str().unwrap().to_string();
        seg["translation"] = serde_json::Value::String(format!("번역: {source}"));
        seg["translated_at"] = serde_json::Value::String("2024-01-01T00:00:00Z".to_string());
    }
    fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();
    let seeded = fs::read_to_string(&state_path).unwrap();

    // Simulate a parser reshape (segment ids shift) plus one genuinely new
    // paragraph inserted before the two already-translated ones.
    fs::write(&doc_path, "New intro.\n\nHello world.\n\nSecond paragraph.\n").unwrap();

    let dry_run = yeokja_cmd()
        .args(["-C", root.to_str().unwrap(), "reconcile", "src", "--dry-run"])
        .status()
        .unwrap();
    assert!(dry_run.success());
    assert_eq!(
        fs::read_to_string(&state_path).unwrap(),
        seeded,
        "--dry-run must not write state"
    );

    let real_run = yeokja_cmd().args(["-C", root.to_str().unwrap(), "reconcile", "src"]).status().unwrap();
    assert!(real_run.success());

    let migrated: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let segments = migrated["segments"].as_array().unwrap();
    assert_eq!(segments.len(), 3, "two kept + one new segment");
    let pending: Vec<&serde_json::Value> =
        segments.iter().filter(|s| s["translation"].is_null()).collect();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["source"], "New intro.");
    let kept: Vec<&serde_json::Value> =
        segments.iter().filter(|s| !s["translation"].is_null()).collect();
    assert_eq!(kept.len(), 2);
    assert!(kept.iter().all(|s| s["translation"].as_str().unwrap().starts_with("번역: ")));

    let check = yeokja_cmd().args(["-C", root.to_str().unwrap(), "status", "src", "--check"]).output().unwrap();
    assert!(!check.status.success(), "one Pending segment remains, --check must fail");
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(stdout.contains("Pending:          1"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Stale:            0"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Glossary stale:   0"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Context changed:  0"), "stdout was:\n{stdout}");
}

#[test]
fn an_already_matching_state_file_is_not_rewritten_but_a_changed_one_is() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("yeokja.toml"), YEOKJA_TOML).unwrap();
    let doc_path = root.join("src/doc.md");
    fs::write(&doc_path, "Hello world.\n").unwrap();
    let state_path = root.join("src/doc.md.yeokja.json");
    fs::write(&state_path, EMPTY_STATE).unwrap();
    let root_arg = root.to_str().unwrap();
    assert!(yeokja_cmd().args(["-C", root_arg, "reconcile", "src"]).status().unwrap().success());

    // Hand-write a glossary_snapshot in reverse-alphabetical key order; a
    // HashMap round trip would almost surely reorder it, so any rewrite shows.
    let seeded = fs::read_to_string(&state_path).unwrap();
    assert!(seeded.contains("\"glossary_snapshot\": {}"), "seeded:\n{seeded}");
    let keys = ["h", "g", "f", "e", "d", "c", "b", "a"];
    let entries: Vec<String> = keys.iter().map(|k| format!("      \"{k}\": \"{k}{k}\"")).collect();
    let snapshot = format!("\"glossary_snapshot\": {{\n{}\n    }}", entries.join(",\n"));
    let seeded = seeded.replace("\"glossary_snapshot\": {}", &snapshot);
    fs::write(&state_path, &seeded).unwrap();

    assert!(yeokja_cmd().args(["-C", root_arg, "reconcile", "src"]).status().unwrap().success());
    assert_eq!(
        fs::read_to_string(&state_path).unwrap(),
        seeded,
        "an unchanged state must not be rewritten"
    );

    fs::write(&doc_path, "Hello world.\n\nA new paragraph.\n").unwrap();
    assert!(yeokja_cmd().args(["-C", root_arg, "reconcile", "src"]).status().unwrap().success());
    let after = fs::read_to_string(&state_path).unwrap();
    assert_ne!(after, seeded);
    assert!(after.contains("A new paragraph."));
}
