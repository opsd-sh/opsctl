mod support;

use serde_json::{Value, json};
use support::{
    fixtures::{B, DATE, I, P, U, password_file},
    http::{check, request},
};

fn rollover() -> Value {
    json!({"id":I,"paye_scheme_id":P,"version":7,"created_at":DATE,
        "from_tax_year_start":2026,"tax_year_start":2027,"employment_allowance":"not_claiming",
        "status":"in_progress","progress":{"phase":"new_year_eps",
            "final_fps":{"status":"accepted","attempt_id":U,"attempt_count":2},
            "eps":{"status":"credentials_required_to_send","message":"Supply credentials"}},
        "preparation_failure":null,"available_actions":[{"action":"send_eps"}],
        "tax_code_notices":[{"employment_id":I,"tax_code":"1257L","tax_basis":"cumulative"}]})
}

#[test]
fn rollover_reads_preserve_progress_and_available_actions() {
    check(
        &["paye-schemes", "rollovers", B, P],
        "GET",
        &format!("businesses/{B}/paye-schemes/{P}/rollovers"),
        None,
        Some(json!([rollover()])),
    );
    check(
        &["paye-schemes", "rollover-status", B, P, I],
        "GET",
        &format!("businesses/{B}/paye-schemes/{P}/rollovers/{I}"),
        None,
        Some(rollover()),
    );
}

#[test]
fn rollover_actions_send_versions_and_only_the_required_credentials() {
    let password = password_file();
    let path = password.path().to_str().unwrap();
    for (command, suffix, extra, body) in [
        (
            "retry-rollover",
            "retry",
            vec!["--sender-id", "sender", "--password-file", path],
            json!({"expected_version":7,"sender_id":"sender","password":"secret"}),
        ),
        (
            "cancel-rollover",
            "cancel",
            vec![],
            json!({"expected_version":7}),
        ),
        (
            "resume-rollover",
            "resume",
            vec!["polling"],
            json!({"expected_version":7,"action":"polling"}),
        ),
        (
            "resume-rollover",
            "resume",
            vec!["reconciliation", "--password-file", path],
            json!({"expected_version":7,"action":"reconciliation","password":"secret"}),
        ),
        (
            "resume-rollover",
            "resume",
            vec!["send-eps", "--sender-id", "sender", "--password-file", path],
            json!({"expected_version":7,"action":"send_eps","sender_id":"sender","password":"secret"}),
        ),
    ] {
        let mut args = vec!["paye-schemes", command, B, P, I, "--expected-version", "7"];
        args.extend(extra);
        check(
            &args,
            "POST",
            &format!("businesses/{B}/paye-schemes/{P}/rollovers/{I}/{suffix}"),
            Some(body),
            Some(rollover()),
        );
    }
}

fn attempt() -> Value {
    json!({"id":U,"attempt_number":2,"recorded_at":DATE,"transaction_id":"test-transaction",
        "environment":"test","status":"rejected","reason":{"code":"validation_failed","message":"Check payroll details"},
        "initial_outcome":{"outcome":"acknowledged","recorded_at":DATE},
        "business_outcome":{"outcome":"rejected","reason":{"code":"validation_failed","message":"Check payroll details"},"recorded_at":DATE},
        "cleanup":{"status":"paused","paused_at":DATE}})
}

#[test]
fn history_commands_preserve_nested_attempts_actions_and_preparation_failures() {
    // Nonempty histories catch lost fields and incorrectly flattened tagged enums.
    let payroll = json!({"payroll_run_id":P,"version":7,"current_revision_id":I,"current_attempt_id":U,
        "revisions":[{"id":I,"revision_number":2,"payment_date":"2026-09-30","finalized_at":DATE,
            "finalized_by_user_id":U,"attempts":[attempt()],
            "actions":[{"action":"retry","attempt_id":U,"user_id":U,"expected_version":6,"recorded_at":DATE}],
            "preparation_failures":[{"attempt_id":U,"operation":"retry","message":"Preparation failed",
                "recorded_at":DATE,"resolved_at":null}]}]});
    check(
        &["payroll-runs", "history", B, P],
        "GET",
        &format!("businesses/{B}/payroll-runs/{P}/history"),
        None,
        Some(payroll),
    );
    let rollover = json!({"rollover_id":I,"paye_scheme_id":P,"version":7,"current_attempt_id":U,
        "final_fps":{"attempts":[attempt()],"actions":[{"action":"retry","attempt_id":U,
            "user_id":U,"expected_version":5,"recorded_at":DATE}],"preparation_failures":[]},
        "new_year_eps":{"attempts":[],"actions":[{"action":"send_eps","user_id":U,"expected_version":6,"recorded_at":DATE}],
            "preparation_failures":[{"operation":"prepare_eps","message":"Preparation failed","recorded_at":DATE,"resolved_at":null}]}});
    check(
        &["paye-schemes", "rollover-history", B, P, I],
        "GET",
        &format!("businesses/{B}/paye-schemes/{P}/rollovers/{I}/history"),
        None,
        Some(rollover),
    );
}

#[test]
fn rollover_conflicts_preserve_operator_version_and_retry_after() {
    let args = [
        "paye-schemes",
        "resume-rollover",
        B,
        P,
        I,
        "--expected-version",
        "7",
        "polling",
    ];
    let (output, sent) = request(&args, 409, "application/problem+json\r\nRetry-After: 42",
        br#"{"type":"https://api.opsd.sh/problems/conflict","title":"Conflict","status":409,"detail":"Wait before polling again","category":"request"}"#);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Wait before polling again"));
    assert!(stderr.contains("42 seconds"));
    assert_eq!(
        sent.lines().next().unwrap(),
        format!("POST /v1/businesses/{B}/paye-schemes/{P}/rollovers/{I}/resume HTTP/1.1")
    );
    assert_eq!(
        serde_json::from_str::<Value>(sent.split_once("\r\n\r\n").unwrap().1).unwrap(),
        json!({"expected_version":7,"action":"polling"})
    );
}

#[test]
fn practice_reporting_reads_and_replacements_preserve_all_fields() {
    let reporting = json!({"name":"Accountants","address":{"lines":["1 Example Road","London"],"post_code":"SW1A 1AA","country":null},
        "contact_name":{"title":"Dr","forenames":["Jamie","Lee"],"surname":"Morgan"},
        "email":"payroll@example.com","telephone":"02079460000"});
    check(
        &["practices", "hmrc-reporting", P],
        "GET",
        &format!("practices/{P}/hmrc-reporting"),
        None,
        Some(reporting.clone()),
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), reporting.to_string()).unwrap();
    let args = [
        "practices",
        "update-hmrc-reporting",
        P,
        "--file",
        file.path().to_str().unwrap(),
    ];
    check(
        &args,
        "PUT",
        &format!("practices/{P}/hmrc-reporting"),
        Some(reporting.clone()),
        Some(reporting),
    );
    let (output, _) = request(&args, 403, "application/problem+json",
        br#"{"type":"https://api.opsd.sh/problems/forbidden","title":"Forbidden","status":403,"detail":"Practice admin required","category":"request"}"#);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Practice admin required"));
}
