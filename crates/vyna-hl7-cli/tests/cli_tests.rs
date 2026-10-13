//! End-to-end CLI tests: `diff` and `send` against mock servers.

use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use vyna_hl7_http::testing::MockHttpServer;
use vyna_hl7_mllp::testing::MockMllpServer;

fn vynahl7() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vynahl7"))
}

/// Async variant: blocking `output()` would starve in-process mock servers
/// on the single-threaded test runtime.
fn vynahl7_async() -> tokio::process::Command {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_vynahl7"))
}

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
    let seq = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path =
        std::env::temp_dir().join(format!("vyna-cli-test-{}-{seq}-{name}", std::process::id()));
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

const ADT_A: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||DOE^JOHN\r";
const ADT_B: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||DOE^JANE\r";

#[test]
fn diff_identical_exits_zero() {
    let a = write_temp("a.hl7", ADT_A);
    let b = write_temp("b.hl7", ADT_A);
    let output = vynahl7()
        .args(["diff", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("No differences"));
    std::fs::remove_file(a).unwrap();
    std::fs::remove_file(b).unwrap();
}

#[test]
fn diff_changed_exits_one_with_path() {
    let a = write_temp("a.hl7", ADT_A);
    let b = write_temp("b.hl7", ADT_B);
    let output = vynahl7()
        .args(["diff", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PID-5-2"), "{stdout}");
    assert!(
        stdout.contains("JOHN") && stdout.contains("JANE"),
        "{stdout}"
    );
    std::fs::remove_file(a).unwrap();
    std::fs::remove_file(b).unwrap();
}

#[test]
fn diff_json_reports_structure() {
    let a = write_temp("a.hl7", ADT_A);
    let b = write_temp("b.hl7", ADT_B);
    let output = vynahl7()
        .args([
            "diff",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(json["different"], true);
    assert_eq!(json["count"], 1);
    assert_eq!(json["differences"][0]["path"], "PID-5-2");
    assert_eq!(json["differences"][0]["kind"], "changed");
    assert_eq!(json["differences"][0]["old"], "JOHN");
    assert_eq!(json["differences"][0]["new"], "JANE");
    std::fs::remove_file(a).unwrap();
    std::fs::remove_file(b).unwrap();
}

#[test]
fn diff_ignore_suppresses_paths() {
    let a = write_temp("a.hl7", ADT_A);
    let b = write_temp("b.hl7", ADT_B);
    let output = vynahl7()
        .args([
            "diff",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            "--ignore",
            "PID-5",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    std::fs::remove_file(a).unwrap();
    std::fs::remove_file(b).unwrap();
}

#[test]
fn diff_rejects_unknown_format() {
    let a = write_temp("a.hl7", ADT_A);
    let output = vynahl7()
        .args([
            "diff",
            a.to_str().unwrap(),
            a.to_str().unwrap(),
            "--format",
            "yaml",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    std::fs::remove_file(a).unwrap();
}

const ACK_AA: &str = "MSH|^~\\&|R|D|S|F|20240101||ACK^A01^ACK|M9|P|2.5\rMSA|AA|M1\r";

#[tokio::test]
async fn send_mllp_returns_ack() {
    let server = MockMllpServer::new()
        .with_handler(|_msg| Ok(vyna_hl7_parser::parse_message(ACK_AA).unwrap()))
        .start()
        .await
        .unwrap();
    let file = write_temp("send.hl7", ADT_A);
    let output = vynahl7_async()
        .args([
            "send",
            file.to_str().unwrap(),
            "--to",
            &format!("mllp://{}", server.url()),
        ])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("MSA|AA"), "{stdout}");
    server.shutdown().await.unwrap();
    std::fs::remove_file(file).unwrap();
}

#[tokio::test]
async fn send_mllp_negative_ack_fails() {
    let server = MockMllpServer::new()
        .with_handler(|_msg| {
            Ok(vyna_hl7_parser::parse_message(
                "MSH|^~\\&|S|F|R|D|20240101||ACK|M9|P|2.5\rMSA|AR|M1|rejected",
            )
            .unwrap())
        })
        .start()
        .await
        .unwrap();
    let file = write_temp("send.hl7", ADT_A);
    let output = vynahl7_async()
        .args([
            "send",
            file.to_str().unwrap(),
            "--to",
            &format!("mllp://{}", server.url()),
        ])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("negative acknowledgment"), "{stderr}");
    server.shutdown().await.unwrap();
    std::fs::remove_file(file).unwrap();
}

#[tokio::test]
async fn send_http_with_auth_returns_ack() {
    let server = MockHttpServer::new()
        .with_auth("user".to_string(), "pass".to_string())
        .with_handler(|_msg| Ok(vyna_hl7_parser::parse_message(ACK_AA).unwrap()))
        .start()
        .await
        .unwrap();
    let file = write_temp("send.hl7", ADT_A);
    let output = vynahl7_async()
        .args([
            "send",
            file.to_str().unwrap(),
            "--to",
            &server.url(),
            "--user",
            "user",
            "--password",
            "pass",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("MSA|AA"));
    server.shutdown().await.unwrap();
    std::fs::remove_file(file).unwrap();
}

#[test]
fn validate_profiles_change_severity() {
    // MSH-12 2.3 validated against 2.5: strict (the default) fails on drift,
    // --strict restates it, --lenient passes.
    let drift = write_temp(
        "drift.hl7",
        "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.3\rEVN||20240101\rPID|1||7||DOE^JOHN\rPV1||I\r",
    );
    let file = drift.to_str().unwrap().to_string();
    let run = |extra: &[&str]| {
        let mut args = vec![file.as_str(), "--hl7-version", "2.5", "--format", "json"];
        args.extend_from_slice(extra);
        let output = vynahl7().args(["validate"]).args(&args).output().unwrap();
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };

    let json = run(&[]);
    assert_eq!(json["mode"], "strict");
    assert_eq!(json["valid"], false);
    assert!(
        json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["location"] == "MSH-12")
    );

    let json = run(&["--strict"]);
    assert_eq!(json["mode"], "strict");
    assert_eq!(json["valid"], false);

    let json = run(&["--standard"]);
    assert_eq!(json["mode"], "standard");
    assert_eq!(json["valid"], true);
    assert!(
        json["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["location"] == "MSH-12")
    );

    let json = run(&["--lenient"]);
    assert_eq!(json["mode"], "lenient");
    assert_eq!(json["valid"], true);

    std::fs::remove_file(drift).unwrap();
}

#[test]
fn validate_applies_message_schema() {
    // A structurally incomplete ADT must fail: schema validation runs.
    let file = write_temp(
        "incomplete.hl7",
        "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7\r",
    );
    let output = vynahl7()
        .args(["validate", file.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_applied"], true);
    assert_eq!(json["valid"], false);
    assert!(
        json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["message"].as_str().unwrap().contains("Required segment"))
    );
    std::fs::remove_file(file).unwrap();
}

#[test]
fn validate_falls_back_without_schema() {
    // QBP has no schema: structure-only validation, honestly reported.
    let file = write_temp(
        "qbp.hl7",
        "MSH|^~\\&|S|F|R|D|20240101||QBP^Q11|M1|P|2.5\rQPD|Q1^Tabular^HL7\r",
    );
    let output = vynahl7()
        .args(["validate", file.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_applied"], false);
    assert_eq!(json["valid"], true);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn validate_invalid_exits_one_with_json_intact() {
    // README contract: validation failure exits 1; the JSON report must
    // still reach stdout for script consumers.
    let file = write_temp(
        "incomplete.hl7",
        "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7\r",
    );
    let output = vynahl7()
        .args(["validate", file.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["valid"], false);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn validate_valid_exits_zero() {
    let file = write_temp(
        "good.hl7",
        "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rEVN||20240101\rPID|1||7||DOE^JOHN\rPV1||I\r",
    );
    let output = vynahl7()
        .args(["validate", file.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["valid"], true);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn extract_invalid_path_fails_on_stderr() {
    let file = write_temp("extract.hl7", ADT_A);
    for format in ["text", "json"] {
        let output = vynahl7()
            .args([
                "extract",
                file.to_str().unwrap(),
                "!!!bad-path",
                "--format",
                format,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "format={format}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid Terser path"),
            "format={format} stderr={stderr}"
        );
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn extract_absent_value_still_exits_zero() {
    // A well-formed path with no value is N/A ("" in JSON), not an error.
    let file = write_temp("extract.hl7", ADT_A);
    let output = vynahl7()
        .args(["extract", file.to_str().unwrap(), "PID-999"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("N/A"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let output = vynahl7()
        .args([
            "extract",
            file.to_str().unwrap(),
            "PID-999",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["PID-999"], "");
    std::fs::remove_file(file).unwrap();
}

#[tokio::test]
async fn send_rejects_bad_scheme_and_refused_ports() {
    let file = write_temp("send.hl7", ADT_A);
    let output = vynahl7()
        .args([
            "send",
            file.to_str().unwrap(),
            "--to",
            "ftp://example.com/x",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());

    let output = vynahl7()
        .args([
            "send",
            file.to_str().unwrap(),
            "--to",
            "mllp://127.0.0.1:9",
            "--timeout",
            "2",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    std::fs::remove_file(file).unwrap();
}
