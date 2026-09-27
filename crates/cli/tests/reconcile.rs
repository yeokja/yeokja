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

#[test]
fn dry_run_does_not_write_and_real_run_leaves_only_the_new_segment_pending() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("yeokja.toml"), YEOKJA_TOML).unwrap();

    let doc_path = root.join("src/doc.md");
    fs::write(&doc_path, "Hello world.\n\nSecond paragraph.\n").unwrap();

    // Seed state through the real parser so hashes line up, then mark both
    // segments translated to simulate a project translated before a parser
    // change.
    let seed = yeokja_cmd().args(["-C", root.to_str().unwrap(), "reconcile", "src"]).status().unwrap();
    assert!(seed.success());

    let state_path = root.join("src/doc.md.yeokja.json");
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
