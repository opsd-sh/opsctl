mod support;

use serde_json::json;
use support::{
    fixtures::{B, DATE, I, P, U, employment, run, scheme},
    http::check,
};

#[test]
fn business_members_and_invitations_use_the_correct_scope_and_roles() {
    let business = json!({"id":B,"name":"Example Ltd","role":"admin"});
    let member = json!({"id":U,"email":"person@example.com","email_verified":true,"role":"payroll_operator"});
    let invitation = json!({"id":I,"email":"person@example.com","role":"payroll_operator","status":"pending","expires_at":DATE});
    let pending = json!({"id":I,"business_id":B,"business_name":"Example Ltd","invited_by_email":"admin@example.com","role":"payroll_operator","expires_at":DATE});
    check(
        &["businesses", "create", "--name", "Example Ltd"],
        "POST",
        "businesses",
        Some(json!({"name":"Example Ltd"})),
        Some(business.clone()),
    );
    check(
        &["businesses", "list"],
        "GET",
        "businesses",
        None,
        Some(json!([business])),
    );
    check(
        &["businesses", "get", B],
        "GET",
        &format!("businesses/{B}"),
        None,
        Some(business.clone()),
    );
    check(
        &["businesses", "members", "list", B],
        "GET",
        &format!("businesses/{B}/members"),
        None,
        Some(json!([member])),
    );
    for (flag, wire) in [("admin", "admin"), ("payroll-operator", "payroll_operator")] {
        check(
            &["businesses", "members", "update", B, U, "--role", flag],
            "PATCH",
            &format!("businesses/{B}/members/{U}"),
            Some(json!({"role":wire})),
            None,
        );
    }
    check(
        &["businesses", "members", "remove", B, U],
        "DELETE",
        &format!("businesses/{B}/members/{U}"),
        None,
        None,
    );
    check(
        &[
            "businesses",
            "invitations",
            "create",
            B,
            "--email",
            "person@example.com",
            "--role",
            "payroll-operator",
        ],
        "POST",
        &format!("businesses/{B}/invitations"),
        Some(json!({"email":"person@example.com","role":"payroll_operator"})),
        Some(invitation.clone()),
    );
    check(
        &["businesses", "invitations", "list", B],
        "GET",
        &format!("businesses/{B}/invitations"),
        None,
        Some(json!([invitation])),
    );
    check(
        &["businesses", "invitations", "cancel", B, I],
        "DELETE",
        &format!("businesses/{B}/invitations/{I}"),
        None,
        None,
    );
    check(
        &["invitations", "list"],
        "GET",
        "business-invitations",
        None,
        Some(json!([pending])),
    );
    check(
        &["invitations", "accept", I],
        "POST",
        &format!("business-invitations/{I}/accept"),
        None,
        Some(business),
    );
    check(
        &["invitations", "decline", I],
        "POST",
        &format!("business-invitations/{I}/decline"),
        None,
        None,
    );
}

#[test]
fn employee_create_and_reads_preserve_reporting_details() {
    let address = json!({"type":"uk","lines":["1 Example Road"],"postcode":"SW1A 1AA"});
    let employee = json!({"id":U,"forenames":["Jamie","Lee"],"surname":"Morgan","created_at":DATE,
        "title":"Dr","passport_number":"EXAMPLE","national_insurance_number":"AB123456C",
        "date_of_birth":"1985-05-05","sex":"M","address":address});
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), address.to_string()).unwrap();
    check(
        &[
            "employees",
            "create",
            B,
            "--forenames",
            "Jamie",
            "Lee",
            "--surname",
            "Morgan",
            "--title",
            "Dr",
            "--passport-number",
            "EXAMPLE",
            "--national-insurance-number",
            "AB123456C",
            "--date-of-birth",
            "1985-05-05",
            "--sex",
            "M",
            "--address",
            file.path().to_str().unwrap(),
        ],
        "POST",
        &format!("businesses/{B}/employees"),
        Some(
            json!({"forenames":["Jamie","Lee"],"surname":"Morgan","title":"Dr","passport_number":"EXAMPLE",
            "national_insurance_number":"AB123456C","date_of_birth":"1985-05-05","sex":"M","address":address}),
        ),
        Some(employee.clone()),
    );
    check(
        &["employees", "list", B],
        "GET",
        &format!("businesses/{B}/employees"),
        None,
        Some(json!([employee])),
    );
    check(
        &["employees", "get", B, U],
        "GET",
        &format!("businesses/{B}/employees/{U}"),
        None,
        Some(employee),
    );
}

#[test]
fn payroll_lists_and_employment_membership_use_scoped_routes() {
    check(
        &["employments", "list", B],
        "GET",
        &format!("businesses/{B}/employments"),
        None,
        Some(json!([employment()])),
    );
    check(
        &["paye-schemes", "list", B],
        "GET",
        &format!("businesses/{B}/paye-schemes"),
        None,
        Some(json!([scheme()])),
    );
    check(
        &["payroll-runs", "list", B],
        "GET",
        &format!("businesses/{B}/payroll-runs"),
        None,
        Some(json!([run()])),
    );
    for (command, method) in [
        ("include-employment", "PUT"),
        ("exclude-employment", "DELETE"),
    ] {
        check(
            &["payroll-runs", command, B, P, I],
            method,
            &format!("businesses/{B}/payroll-runs/{P}/employments/{I}"),
            None,
            None,
        );
    }
}
