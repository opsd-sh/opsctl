mod support;

use serde_json::json;
use std::process::Command;
use support::{
    fixtures::{B, DATE, I, P, U},
    http::{check, request},
};

#[test]
fn practice_members_clients_and_invitations_route_and_print_correctly() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let reporting = json!({"name":"Accountants","address":null,"contact_name":null,"email":null,"telephone":null});
    std::fs::write(file.path(), reporting.to_string()).unwrap();
    let practice = json!({"id":P,"name":"Accountants","role":"admin"});
    let client = json!({"id":B,"name":"Client","practice_id":P});
    let member = json!({"id":U,"email":"person@example.com","email_verified":true,"role":"member"});
    let invitation = json!({"id":I,"email":"person@example.com","role":"member","status":"pending","expires_at":DATE});
    let pending = json!({"id":I,"practice_id":P,"practice_name":"Accountants","invited_by_email":"admin@example.com","role":"member","expires_at":DATE});
    check(
        &[
            "practices",
            "create",
            "--name",
            "Accountants",
            "--hmrc-reporting",
            file.path().to_str().unwrap(),
        ],
        "POST",
        "practices",
        Some(json!({"name":"Accountants","hmrc_reporting":reporting})),
        Some(practice.clone()),
    );
    check(
        &["practices", "list"],
        "GET",
        "practices",
        None,
        Some(json!([practice])),
    );
    check(
        &["practices", "get", P],
        "GET",
        &format!("practices/{P}"),
        None,
        Some(practice.clone()),
    );
    check(
        &["practices", "businesses", "create", P, "--name", "Client"],
        "POST",
        &format!("practices/{P}/businesses"),
        Some(json!({"name":"Client"})),
        Some(client.clone()),
    );
    check(
        &["practices", "businesses", "list", P],
        "GET",
        &format!("practices/{P}/businesses"),
        None,
        Some(json!([client])),
    );
    check(
        &["practices", "members", "list", P],
        "GET",
        &format!("practices/{P}/members"),
        None,
        Some(json!([member])),
    );
    for role in ["admin", "member"] {
        check(
            &["practices", "members", "update", P, U, "--role", role],
            "PATCH",
            &format!("practices/{P}/members/{U}"),
            Some(json!({"role":role})),
            None,
        );
    }
    check(
        &["practices", "members", "remove", P, U],
        "DELETE",
        &format!("practices/{P}/members/{U}"),
        None,
        None,
    );
    check(
        &[
            "practices",
            "invitations",
            "create",
            P,
            "--email",
            "person@example.com",
            "--role",
            "member",
        ],
        "POST",
        &format!("practices/{P}/invitations"),
        Some(json!({"email":"person@example.com","role":"member"})),
        Some(invitation.clone()),
    );
    check(
        &["practices", "invitations", "list", P],
        "GET",
        &format!("practices/{P}/invitations"),
        None,
        Some(json!([invitation])),
    );
    check(
        &["practices", "invitations", "cancel", P, I],
        "DELETE",
        &format!("practices/{P}/invitations/{I}"),
        None,
        None,
    );
    check(
        &["practice-invitations", "list"],
        "GET",
        "practice-invitations",
        None,
        Some(json!([pending])),
    );
    check(
        &["practice-invitations", "accept", I],
        "POST",
        &format!("practice-invitations/{I}/accept"),
        None,
        Some(practice),
    );
    check(
        &["practice-invitations", "decline", I],
        "POST",
        &format!("practice-invitations/{I}/decline"),
        None,
        None,
    );
}

#[test]
fn payroll_commands_preserve_agreement_and_suspension_states() {
    check(
        &[
            "businesses",
            "payroll",
            "subscribe",
            B,
            "--expected-version",
            "0",
        ],
        "POST",
        &format!("businesses/{B}/payroll-subscription?expected_version=0"),
        None,
        Some(json!({"status":"subscribed","subscribed_at":DATE,"subscription_version":0})),
    );
    check(
        &["businesses", "payroll", "status", B],
        "GET",
        &format!("businesses/{B}/payroll-subscription"),
        None,
        Some(
            json!({"status":"cancelled","subscribed_at":DATE,"subscription_version":0,"cancelled_at":DATE,"suspension":{"suspended_at":DATE}}),
        ),
    );
    check(
        &[
            "businesses",
            "payroll",
            "cancel",
            B,
            "--expected-version",
            "0",
        ],
        "DELETE",
        &format!("businesses/{B}/payroll-subscription?expected_version=0"),
        None,
        None,
    );
    check(
        &[
            "practices",
            "payroll",
            "enable",
            P,
            B,
            "--expected-version",
            "0",
        ],
        "POST",
        &format!("practices/{P}/businesses/{B}/payroll?expected_version=0"),
        None,
        Some(
            json!({"status":"enabled","enabled_at":DATE,"subscription_version":0,"suspension":null}),
        ),
    );
    check(
        &["practices", "payroll", "status", P, B],
        "GET",
        &format!("practices/{P}/businesses/{B}/payroll"),
        None,
        Some(
            json!({"status":"disabled","enabled_at":DATE,"disabled_at":DATE,"subscription_version":0,"suspension":{"suspended_at":DATE}}),
        ),
    );
    check(
        &[
            "practices",
            "payroll",
            "disable",
            P,
            B,
            "--expected-version",
            "0",
        ],
        "DELETE",
        &format!("practices/{P}/businesses/{B}/payroll?expected_version=0"),
        None,
        None,
    );
}

#[test]
fn billing_commands_support_both_payers_and_explicit_pagination() {
    for (payer, id) in [("businesses", B), ("practices", P)] {
        let base = format!("{payer}/{id}/billing");
        check(
            &[payer, "billing", "status", id],
            "GET",
            &format!("{base}/status"),
            None,
            Some(json!({"payment_method_saved":false})),
        );
        check(
            &[
                payer,
                "billing",
                "payment-methods",
                id,
                "--starting-after",
                "pm_previous",
                "--limit",
                "10",
            ],
            "GET",
            &format!("{base}/payment-methods?starting_after=pm_previous&limit=10"),
            None,
            Some(
                json!({"payment_methods":[{"id":"pm_bank","type":"bacs_debit","is_default":true,"card":null}],"next_starting_after":"pm_bank"}),
            ),
        );
        check(
            &[
                payer,
                "billing",
                "set-default-payment-method",
                id,
                "pm_bank",
            ],
            "PUT",
            &format!("{base}/default-payment-method"),
            Some(json!({"payment_method_id":"pm_bank"})),
            None,
        );
        check(
            &[
                payer, "billing", "invoices", id, "--before", "2026-10", "--limit", "10",
            ],
            "GET",
            &format!("{base}/invoices?before=2026-10&limit=10"),
            None,
            Some(
                json!({"invoices":[{"id":I,"billing_month":"2026-09","currency":"gbp","subtotal_pence":100,"created_at":DATE,"status":"paid"}],"next_before":"2026-09"}),
            ),
        );
        check(
            &[
                payer,
                "billing",
                "invoice-lines",
                id,
                I,
                "--offset",
                "10",
                "--limit",
                "10",
            ],
            "GET",
            &format!("{base}/invoices/{I}/lines?offset=10&limit=10"),
            None,
            Some(
                json!({"lines":[{"business_id":B,"business_name":"Client","charge_kind":"base_fee","usage_month":"2026-09","quantity":1,"unit_amount_pence":100,"amount_pence":100}],"next_offset":20}),
            ),
        );
        for (action, suffix, empty) in [
            (
                "payment-methods",
                "payment-methods",
                json!({"payment_methods":[],"next_starting_after":null}),
            ),
            (
                "invoices",
                "invoices",
                json!({"invoices":[],"next_before":null}),
            ),
        ] {
            check(
                &[payer, "billing", action, id],
                "GET",
                &format!("{base}/{suffix}"),
                None,
                Some(empty),
            );
        }
    }
}

#[test]
fn pdf_downloads_preserve_binary_bytes_and_never_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = b"%PDF-1.7\n\xff\xfe\n%%EOF";
    for (payer, id) in [("businesses", B), ("practices", P)] {
        let path = directory.path().join(format!("{payer}.pdf"));
        let args = [
            payer,
            "billing",
            "invoice-pdf",
            id,
            I,
            "--output",
            path.to_str().unwrap(),
        ];
        let (output, req) = request(&args, 200, "application/pdf", bytes);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            req.lines().next().unwrap(),
            format!("GET /v1/{payer}/{id}/billing/invoices/{I}/pdf HTTP/1.1")
        );
        let (output, _) = request(&args, 200, "application/pdf", b"%PDF-new");
        assert!(!output.status.success());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn failures_have_nonzero_exit_and_do_not_create_pdf_files() {
    let directory = tempfile::tempdir().unwrap();
    for status in [401, 403, 404, 409, 502] {
        let path = directory.path().join(format!("{status}.pdf"));
        let body=json!({"type":"https://api.opsd.sh/problems/test","title":"Denied","status":status,"detail":"Request denied","category":"request"}).to_string();
        for args in [
            vec![
                "practices",
                "billing",
                "invoice-pdf",
                P,
                I,
                "--output",
                path.to_str().unwrap(),
            ],
            vec![
                "practices",
                "payroll",
                "disable",
                P,
                B,
                "--expected-version",
                "0",
            ],
            vec!["practices", "get", P],
        ] {
            let (output, _) = request(&args, status, "application/problem+json", body.as_bytes());
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("Request denied"));
        }
        assert!(!path.exists());
    }
    let path = directory.path().join("malformed.pdf");
    let (output, _) = request(
        &[
            "practices",
            "billing",
            "invoice-pdf",
            P,
            I,
            "--output",
            path.to_str().unwrap(),
        ],
        200,
        "application/json",
        b"{}",
    );
    assert!(!output.status.success());
    assert!(!path.exists());
}

#[test]
fn invalid_arguments_fail_before_authentication_and_commands_require_login() {
    let config = tempfile::tempdir().unwrap();
    for args in [
        vec!["practices", "get", "bad"],
        vec!["practices", "create", "--name", ""],
        vec![
            "practices",
            "members",
            "update",
            P,
            U,
            "--role",
            "payroll-operator",
        ],
        vec!["practices", "payroll", "enable", P],
        vec!["practices", "billing", "invoice-pdf", P, I],
        vec!["practices", "billing", "invoices", P, "--limit", "0"],
        vec!["businesses", "billing", "invoices", B, "--limit", "101"],
        vec![
            "practices",
            "billing",
            "invoices",
            P,
            "--before",
            "2026-10-01",
        ],
        vec![
            "businesses",
            "billing",
            "invoice-lines",
            B,
            I,
            "--offset",
            "-1",
        ],
        vec![
            "practices",
            "billing",
            "payment-methods",
            P,
            "--starting-after",
            "cus_wrong",
        ],
        vec![
            "businesses",
            "billing",
            "set-default-payment-method",
            B,
            "pm_",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_opsctl"))
            .env("OPSCTL_CONFIG_DIR", config.path())
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
    }
    for args in [
        vec!["practices", "list"],
        vec!["practice-invitations", "list"],
        vec![
            "businesses",
            "payroll",
            "subscribe",
            B,
            "--expected-version",
            "0",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_opsctl"))
            .env("OPSCTL_CONFIG_DIR", config.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("login"));
    }
}
