use std::process::Command;

fn cargo_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lint-quality"))
}

#[test]
fn scan_fixtures_human() {
    let output = cargo_bin()
        .args(["scan", "tests/fixtures", "--format", "human"])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Lint Quality Report"));
    assert!(stdout.contains("eslint-disable-next-line"));
    assert!(stdout.contains("ts-ignore"));
    assert!(stdout.contains("ts-expect-error"));
}

#[test]
fn scan_fixtures_json() {
    let output = cargo_bin()
        .args(["scan", "tests/fixtures", "--format", "json"])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");

    // Check structure
    assert!(report["metadata"]["files_scanned"].as_u64().unwrap() >= 3);
    assert!(report["files"].as_array().unwrap().len() >= 3); // sample.ts, sample.js, sample.vue
    assert!(report["summary"]["total_violations"].as_u64().unwrap() >= 8);

    // Check that clean.ts is NOT in the files list (no violations)
    let files = report["files"].as_array().unwrap();
    for f in files {
        assert!(
            !f["path"].as_str().unwrap().ends_with("clean.ts"),
            "clean.ts should not appear in results"
        );
    }
}

#[test]
fn scan_fixtures_json_roundtrip() {
    let output = cargo_bin()
        .args(["scan", "tests/fixtures", "--format", "json"])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    // Parse as our Report type would
    let first: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    let reserialized = serde_json::to_string_pretty(&first).unwrap();
    let second: serde_json::Value =
        serde_json::from_str(&reserialized).expect("invalid JSON on roundtrip");
    assert_eq!(first, second);
}

#[test]
fn scan_with_extension_filter() {
    let output = cargo_bin()
        .args([
            "scan",
            "tests/fixtures",
            "--format",
            "json",
            "--extensions",
            "js",
        ])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    let files = report["files"].as_array().unwrap();

    // Only .js files should be present
    for f in files {
        assert!(
            f["path"].as_str().unwrap().ends_with(".js"),
            "only .js files expected, got: {}",
            f["path"]
        );
    }
}

#[test]
fn scan_nonexistent_path() {
    let output = cargo_bin()
        .args(["scan", "does/not/exist"])
        .output()
        .expect("failed to execute");

    // Should fail gracefully
    assert!(!output.status.success() || output.stdout.is_empty());
}

#[test]
fn rule_extraction() {
    let output = cargo_bin()
        .args(["scan", "tests/fixtures", "--format", "json"])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");

    // Check by_rule in summary has expected rules
    let by_rule = &report["summary"]["by_rule"];
    assert!(
        by_rule["@typescript-eslint/no-explicit-any"]
            .as_u64()
            .unwrap()
            >= 1,
        "should detect @typescript-eslint/no-explicit-any"
    );
    assert!(
        by_rule["*"].as_u64().unwrap() >= 1,
        "should detect wildcard (*) for eslint-disable without rules"
    );
}

#[test]
fn codeowners_integration() {
    let output = cargo_bin()
        .args([
            "scan",
            "tests/fixtures",
            "--format",
            "json",
            "--codeowners",
            "tests/fixtures/CODEOWNERS",
        ])
        .output()
        .expect("failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    let files = report["files"].as_array().unwrap();

    // Find the .vue file — should be owned by @frontend-team (last match wins)
    let vue_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().ends_with(".vue"))
        .expect("should have a .vue file");
    assert_eq!(
        vue_file["owner"].as_str().unwrap(),
        "@frontend-team",
        ".vue file should be owned by @frontend-team"
    );

    // .ts files match *.ts → @test-team (last match wins over * @default-team)
    let ts_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().ends_with("sample.ts"))
        .expect("should have sample.ts");
    assert_eq!(
        ts_file["owner"].as_str().unwrap(),
        "@test-team",
        "sample.ts should be owned by @test-team"
    );

    // .js files only match * → @default-team
    let js_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().ends_with("sample.js"))
        .expect("should have sample.js");
    assert_eq!(
        js_file["owner"].as_str().unwrap(),
        "@default-team",
        "sample.js should be owned by @default-team"
    );

    // Summary should have by_owner counts
    let by_owner = &report["summary"]["by_owner"];
    assert!(
        by_owner["@test-team"].as_u64().unwrap() >= 1,
        "should have @test-team in by_owner"
    );
    assert!(
        by_owner["@frontend-team"].as_u64().unwrap() >= 1,
        "should have @frontend-team in by_owner"
    );
    assert!(
        by_owner["@default-team"].as_u64().unwrap() >= 1,
        "should have @default-team in by_owner"
    );
}

#[test]
fn no_patterns_error() {
    // Create a minimal config with no patterns
    let dir = std::env::temp_dir().join("lint-quality-test-no-patterns");
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("lint-quality.toml");
    std::fs::write(&config, "extensions = [\"ts\"]\n").unwrap();

    let output = cargo_bin()
        .args([
            "scan",
            dir.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute");

    assert!(!output.status.success(), "should fail with no patterns");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No patterns configured"),
        "should mention missing patterns in error"
    );

    std::fs::remove_dir_all(&dir).ok();
}
