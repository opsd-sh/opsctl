use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const P: &str = "11111111-1111-4111-8111-111111111111";
const B: &str = "22222222-2222-4222-8222-222222222222";
const U: &str = "33333333-3333-4333-8333-333333333333";
const I: &str = "44444444-4444-4444-8444-444444444444";
const DATE: &str = "2026-10-01T12:00:00Z";

fn request(args: &[&str], status: u16, content_type: &str, body: &[u8]) -> (Output, String) {
    let (output, mut requests) =
        requests(args, vec![(status, content_type.to_owned(), body.to_vec())]);
    (output, requests.remove(0))
}
fn requests(args: &[&str], responses: Vec<(u16, String, Vec<u8>)>) -> (Output, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base_url = format!("http://{}/", listener.local_addr().unwrap());

    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, content_type, body) in responses {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "CLI did not send a request");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&request[..end])
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
            stream.write_all(&body).unwrap();
            requests.push(String::from_utf8(request).unwrap());
        }
        requests
    });
    let config = tempfile::tempdir().unwrap();
    let path = config.path().join("credentials.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "server_url":base_url,"access_token":"test-token",
            "expires_at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()+3600,
        }))
        .unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_opsctl"))
        .env("OPSCTL_CONFIG_DIR", config.path())
        .args(["--base-url", &base_url])
        .args(args)
        .output()
        .unwrap();
    let request = server.join().unwrap();
    for request in &request {
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
    }
    (output, request)
}

fn check(args: &[&str], method: &str, path: &str, sent: Option<Value>, response: Option<Value>) {
    let body = response.as_ref().map(Value::to_string).unwrap_or_default();
    let (output, request) = request(
        args,
        if response.is_some() { 200 } else { 204 },
        "application/json",
        body.as_bytes(),
    );
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        request.lines().next().unwrap(),
        format!("{method} /v1/{path} HTTP/1.1")
    );
    let actual_body = request.split_once("\r\n\r\n").unwrap().1;
    if let Some(sent) = sent {
        assert_eq!(serde_json::from_str::<Value>(actual_body).unwrap(), sent);
    } else {
        assert!(actual_body.is_empty());
    }
    if let Some(response) = response {
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            response
        );
    } else {
        assert!(output.stdout.is_empty());
    }
}

fn scheme() -> Value {
    json!({"id":P,"rollover_generation":0,"name":"PAYE","company_utr":null,"employer_reference":"123/AB456","accounts_office_reference":"123PA00045678","tax_year_start":2026,"employment_allowance":"not_claiming","created_at":DATE})
}
fn employment() -> Value {
    json!({"id":I,"employee_id":U,"paye_scheme_id":P,"pay_schedule_id":P,"created_at":DATE,"student_loan":"not_applicable","postgraduate_loan":"not_applicable","normal_weekly_hours_band":"thirty_or_more","monthly_gross_pay_pence":300000,"start_date":"2026-09-01","ni_category":"A","tax_year_start":2026,"tax_code":"1257L","tax_basis":"cumulative","reporting_starter_declaration":"A","previous_employment":{"taxable_pay_pence":0,"tax_paid_pence":0},"starter_information":{"type":"checklist","declaration":"A"}})
}
fn run() -> Value {
    json!({"id":P,"paye_scheme_id":P,"pay_schedule_id":P,"payment_date":"2026-09-30","status":"draft","version":0,"revision_id":null,"available_actions":[],"employment_count":1,"finalized_at":null,"finalized_by_user_id":null,"created_at":DATE})
}
fn calculation() -> Value {
    let amounts = json!({"gross_pay_pence":300000,"income_tax_pence":0,"employee_ni_pence":0,"employer_ni_pence":0,"net_pay_pence":300000});
    let zero = json!({"gross_pay_pence":0,"income_tax_pence":0,"employee_ni_pence":0,"employer_ni_pence":0,"net_pay_pence":0});
    let mut payment = amounts.clone();
    payment["employment_id"] = json!(I);
    payment["ytd"] = amounts.clone();
    let bands = json!({"at_lel_pence":0,"above_lel_to_pt_pence":0,"above_pt_to_uel_pence":0});
    payment["ni_earnings"] = bands.clone();
    payment["ni_earnings_ytd"] = bands.clone();
    payment["inputs"] = json!({"tax_code":"NT","tax_basis":"non_cumulative","ni_category":"X","previous_employment_taxable_pay_pence":0,"previous_employment_tax_paid_pence":0,"previous_ytd":zero});
    payment["inputs"]["previous_ni_earnings_ytd"] = bands;
    json!({"payroll_run_id":P,"payment_date":"2026-09-30","tax_year_start":2026,"tax_month":6,"payments":[payment],"totals":amounts})
}
fn finalised() -> Value {
    let mut v = run();
    v["status"] = json!("accepted");
    v["finalized_at"] = json!(DATE);
    v["finalized_by_user_id"] = json!(U);
    v["employment_ids"] = json!([I]);
    v["payments"] = json!([{"employment_id":I,"gross_pay_pence":300000,"normal_weekly_hours_band":"thirty_or_more"}]);
    v["calculation"] = calculation();
    v
}
#[test]
fn payroll_setup_commands_send_complete_typed_requests() {
    check(
        &[
            "paye-schemes",
            "create",
            B,
            "--name",
            "PAYE",
            "--employer-reference",
            "123/AB456",
            "--accounts-office-reference",
            "123PA00045678",
            "--tax-year-start",
            "2026",
            "--employment-allowance",
            "not_claiming",
        ],
        "POST",
        &format!("businesses/{B}/paye-schemes"),
        Some(
            json!({"name":"PAYE","company_utr":null,"employer_reference":"123/AB456","accounts_office_reference":"123PA00045678","tax_year_start":2026,"employment_allowance":"not_claiming"}),
        ),
        Some(scheme()),
    );
    let schedule = json!({"id":P,"paye_scheme_id":P,"name":"Monthly staff","frequency":"monthly","created_at":DATE});
    check(
        &[
            "pay-schedules",
            "create",
            B,
            "--paye-scheme-id",
            P,
            "--name",
            "Monthly staff",
        ],
        "POST",
        &format!("businesses/{B}/pay-schedules"),
        Some(json!({"paye_scheme_id":P,"name":"Monthly staff","frequency":"monthly"})),
        Some(schedule.clone()),
    );
    check(
        &["pay-schedules", "list", B],
        "GET",
        &format!("businesses/{B}/pay-schedules"),
        None,
        Some(json!([schedule])),
    );
    check(
        &[
            "employments",
            "create",
            B,
            "--employee-id",
            U,
            "--paye-scheme-id",
            P,
            "--pay-schedule-id",
            P,
            "--start-date",
            "2026-09-01",
            "--ni-category",
            "A",
            "--monthly-gross-pay-pence",
            "300000",
            "--normal-weekly-hours-band",
            "thirty_or_more",
            "--student-loan",
            "not_applicable",
            "--postgraduate-loan",
            "not_applicable",
            "checklist",
            "--declaration",
            "A",
        ],
        "POST",
        &format!("businesses/{B}/employments"),
        Some(
            json!({"employee_id":U,"paye_scheme_id":P,"pay_schedule_id":P,"start_date":"2026-09-01","ni_category":"A","student_loan":"not_applicable","postgraduate_loan":"not_applicable","normal_weekly_hours_band":"thirty_or_more","monthly_gross_pay_pence":300000,"starter_information":{"type":"checklist","declaration":"A"}}),
        ),
        Some(employment()),
    );
    check(
        &["employments", "get", B, I],
        "GET",
        &format!("businesses/{B}/employments/{I}"),
        None,
        Some(employment()),
    );
    check(
        &[
            "employments",
            "update",
            B,
            I,
            "--monthly-gross-pay-pence",
            "310000",
        ],
        "PATCH",
        &format!("businesses/{B}/employments/{I}"),
        Some(json!({"monthly_gross_pay_pence":310000})),
        Some(employment()),
    );
    check(
        &[
            "employments",
            "tax-code-notice",
            B,
            I,
            "--tax-year-start",
            "2026",
            "--tax-code",
            "1257L",
            "cumulative",
            "--previous-taxable-pay-pence",
            "100000",
            "--previous-tax-paid-pence",
            "-20",
        ],
        "PATCH",
        &format!("businesses/{B}/employments/{I}"),
        Some(
            json!({"tax_code_notice":{"tax_year_start":2026,"tax_code":"1257L","tax_basis":"cumulative","previous_employment":{"taxable_pay_pence":100000,"tax_paid_pence":-20}}}),
        ),
        Some(employment()),
    );
}
#[test]
fn payroll_run_commands_preserve_saved_results_and_updates() {
    let password = password_file();
    check(
        &[
            "payroll-runs",
            "create",
            B,
            "--paye-scheme-id",
            P,
            "--pay-schedule-id",
            P,
            "--payment-date",
            "2026-09-30",
        ],
        "POST",
        &format!("businesses/{B}/payroll-runs"),
        Some(json!({"paye_scheme_id":P,"pay_schedule_id":P,"payment_date":"2026-09-30"})),
        Some(run()),
    );
    check(
        &["payroll-runs", "preview", B, P],
        "GET",
        &format!("businesses/{B}/payroll-runs/{P}/preview"),
        None,
        Some(calculation()),
    );
    check(
        &["payroll-runs", "get", B, P],
        "GET",
        &format!("businesses/{B}/payroll-runs/{P}"),
        None,
        Some(finalised()),
    );
    check(
        &[
            "payroll-runs",
            "finalize",
            B,
            P,
            "--expected-version",
            "0",
            "--sender-id",
            "sender",
            "--password-file",
            password.path().to_str().unwrap(),
        ],
        "POST",
        &format!("businesses/{B}/payroll-runs/{P}/finalize"),
        Some(
            json!({"expected_version":0,"sender_id":"sender","password":"secret","late_reporting_reason":null}),
        ),
        Some(finalised()),
    );
    check(
        &[
            "payroll-runs",
            "update-payment",
            B,
            P,
            I,
            "--gross-pay-pence",
            "150000",
        ],
        "PATCH",
        &format!("businesses/{B}/payroll-runs/{P}/employments/{I}"),
        Some(json!({"gross_pay_pence":150000})),
        None,
    );
    check(
        &["payroll-runs", "discard", B, P],
        "DELETE",
        &format!("businesses/{B}/payroll-runs/{P}"),
        None,
        None,
    );
}
#[test]
fn rollover_sends_explicit_notices_and_preserves_api_errors() {
    let password = password_file();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "[]").unwrap();
    let args = [
        "paye-schemes",
        "rollover",
        B,
        P,
        "--tax-year-start",
        "2026",
        "--employment-allowance",
        "not_claiming",
        "--tax-code-notices",
        file.path().to_str().unwrap(),
        "--expected-generation",
        "0",
        "--sender-id",
        "sender",
        "--password-file",
        password.path().to_str().unwrap(),
    ];
    check(
        &args,
        "POST",
        &format!("businesses/{B}/paye-schemes/{P}/rollover"),
        Some(
            json!({"expected_generation":0,"sender_id":"sender","password":"secret","tax_year_start":2026,"employment_allowance":"not_claiming","tax_code_notices":[]}),
        ),
        Some(
            json!({"id":I,"paye_scheme_id":P,"version":1,"created_at":DATE,"from_tax_year_start":2025,"tax_year_start":2026,"employment_allowance":"not_claiming","status":"in_progress","progress":{"phase":"final_fps","final_fps":{"attempt_id":I,"attempt_count":1,"status":"pending","message":null}},"preparation_failure":null,"available_actions":[],"tax_code_notices":[]}),
        ),
    );
    let (output,_)=request(&args,409,"application/problem+json",br#"{"type":"https://api.opsd.sh/problems/conflict","title":"Conflict","status":409,"detail":"discard outstanding drafts first","category":"request"}"#);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("discard outstanding drafts first"));
}
#[test]
fn payment_list_uses_saved_net_pay_and_employee_names() {
    let employees = json!([{"id":U,"forenames":["Jamie","Lee"],"surname":"Morgan","created_at":DATE,"title":null,"passport_number":null,"national_insurance_number":null,"date_of_birth":"1985-05-05","sex":"M","address":{"type":"uk","lines":["1 Example Road"],"postcode":"SW1A 1AA"}}]);
    for state in [
        json!({"status":"accepted"}),
        json!({"status":"finalized","submission":{"attempt_id":I,"attempt_count":1,
            "recorded_at":DATE,"initial_outcome_at":DATE,"business_outcome_at":null,"status":"pending"}}),
        json!({"status":"finalized","submission":{"attempt_id":I,"attempt_count":1,
            "recorded_at":DATE,"initial_outcome_at":DATE,"business_outcome_at":DATE,
            "status":"rejected","reason":{"code":"validation_failed","message":"Check payroll details"}}}),
    ] {
        let mut payroll = finalised();
        payroll["revision_id"] = json!(I);
        payroll
            .as_object_mut()
            .unwrap()
            .extend(state.as_object().unwrap().clone());
        let responses = [payroll, json!([employment()]), employees.clone()]
            .into_iter()
            .map(|v| {
                (
                    200,
                    "application/json".to_owned(),
                    v.to_string().into_bytes(),
                )
            })
            .collect();
        let (output, requests) = requests(&["payroll-runs", "payments", B, P], responses);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(requests.len(), 3);
        let mut expected = json!({"payroll_run_id":P,"revision_id":I,"payment_date":"2026-09-30",
            "payments":[{"employment_id":I,"employee_id":U,"forenames":["Jamie","Lee"],"surname":"Morgan","net_pay_pence":300000}],"total_net_pay_pence":300000});
        expected
            .as_object_mut()
            .unwrap()
            .extend(state.as_object().unwrap().clone());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            expected
        );
    }
    let mut draft = run();
    draft["employment_ids"] = json!([I]);
    draft["payments"] = json!([]);
    draft["calculation"] = Value::Null;
    let (output, _) = request(
        &["payroll-runs", "payments", B, P],
        200,
        "application/json",
        draft.to_string().as_bytes(),
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("finalised run"));
}

fn password_file() -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "secret\r\n").unwrap();
    file
}

#[test]
fn payroll_actions_send_explicit_versions_and_only_required_credentials() {
    let password = password_file();
    for (action, extra, body) in [
        (
            "retry",
            vec![
                "--sender-id",
                "sender",
                "--password-file",
                password.path().to_str().unwrap(),
            ],
            json!({"expected_version":7,"sender_id":"sender","password":"secret"}),
        ),
        ("reopen", vec![], json!({"expected_version":7})),
        ("cancel", vec![], json!({"expected_version":7})),
        (
            "resume",
            vec!["polling"],
            json!({"expected_version":7,"action":"polling"}),
        ),
        (
            "resume",
            vec![
                "reconciliation",
                "--password-file",
                password.path().to_str().unwrap(),
            ],
            json!({"expected_version":7,"action":"reconciliation","password":"secret"}),
        ),
    ] {
        let mut args = vec!["payroll-runs", action, B, P, "--expected-version", "7"];
        args.extend(extra);
        check(
            &args,
            "POST",
            &format!("businesses/{B}/payroll-runs/{P}/{action}"),
            Some(body),
            Some(finalised()),
        );
    }
}

#[test]
fn hours_only_update_and_nullable_employee_updates_preserve_intent() {
    check(
        &[
            "payroll-runs",
            "update-payment",
            B,
            P,
            I,
            "--normal-weekly-hours-band",
            "other",
        ],
        "PATCH",
        &format!("businesses/{B}/payroll-runs/{P}/employments/{I}"),
        Some(json!({"normal_weekly_hours_band":"other"})),
        None,
    );
    let employee = json!({"id":U,"forenames":["Jamie"],"surname":"Morgan","created_at":DATE,"title":null,"passport_number":null,"national_insurance_number":null,"date_of_birth":"1985-05-05","sex":"M","address":{"type":"uk","lines":["1 Example Road"],"postcode":"SW1A 1AA"}});
    check(
        &["employees", "update", B, U, "--clear-passport-number"],
        "PATCH",
        &format!("businesses/{B}/employees/{U}"),
        Some(json!({"passport_number":null})),
        Some(employee),
    );
}

#[test]
fn hmrc_wait_is_reported_without_automatically_retrying() {
    let (output,_)=request(&["payroll-runs","resume",B,P,"--expected-version","7","polling"],409,"application/problem+json\r\nRetry-After: 42",br#"{"type":"https://api.opsd.sh/problems/conflict","title":"Conflict","status":409,"detail":"HMRC requires a wait before polling again","category":"request"}"#);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("HMRC requires a wait"));
    assert!(stderr.contains("42 seconds"));
}
