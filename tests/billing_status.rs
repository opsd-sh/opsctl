mod support;

use std::process::Command;
use support::{fixtures::B as BUSINESS_ID, http::request};

#[test]
fn billing_status_prints_json_and_propagates_api_errors() {
    for (status, body) in [
        (200, r#"{"payment_method_saved":true}"#),
        (200, r#"{"payment_method_saved":false}"#),
        (
            403,
            r#"{"type":"https://api.opsd.sh/problems/forbidden","title":"Forbidden","status":403,"detail":"business admin role required","category":"request"}"#,
        ),
    ] {
        let (output, request) = request(
            &["businesses", "billing", "status", BUSINESS_ID],
            status,
            "application/json",
            body.as_bytes(),
        );
        assert_eq!(
            request.lines().next(),
            Some(format!("GET /v1/businesses/{BUSINESS_ID}/billing/status HTTP/1.1").as_str())
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
        if status == 200 {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
                serde_json::from_str::<serde_json::Value>(body).unwrap()
            );
        } else {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("business admin role required")
            );
        }
    }
}

#[test]
fn billing_status_requires_login_and_a_valid_business_id() {
    let config = tempfile::tempdir().unwrap();
    for id in [None, Some("invalid"), Some(BUSINESS_ID)] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_opsctl"));
        command
            .env("OPSCTL_CONFIG_DIR", config.path())
            .args(["businesses", "billing", "status"]);
        if let Some(id) = id {
            command.arg(id);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        if id == Some(BUSINESS_ID) {
            assert!(error.contains("login"), "{error}");
        } else {
            assert_eq!(output.status.code(), Some(2));
        }
    }
}
