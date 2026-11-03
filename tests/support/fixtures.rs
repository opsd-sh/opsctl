//! Shared synthetic IDs and payroll response fixtures.
use serde_json::{Value, json};

pub const P: &str = "11111111-1111-4111-8111-111111111111";
pub const B: &str = "22222222-2222-4222-8222-222222222222";
pub const U: &str = "33333333-3333-4333-8333-333333333333";
pub const I: &str = "44444444-4444-4444-8444-444444444444";
pub const DATE: &str = "2026-10-01T12:00:00Z";

pub fn scheme() -> Value {
    json!({"id":P,"rollover_generation":0,"name":"PAYE","company_utr":null,"employer_reference":"123/AB456","accounts_office_reference":"123PA00045678","tax_year_start":2026,"employment_allowance":"not_claiming","created_at":DATE})
}
pub fn employment() -> Value {
    json!({"id":I,"employee_id":U,"paye_scheme_id":P,"pay_schedule_id":P,"created_at":DATE,"student_loan":"not_applicable","postgraduate_loan":"not_applicable","normal_weekly_hours_band":"thirty_or_more","monthly_gross_pay_pence":300000,"start_date":"2026-09-01","ni_category":"A","tax_year_start":2026,"tax_code":"1257L","tax_basis":"cumulative","reporting_starter_declaration":"A","previous_employment":{"taxable_pay_pence":0,"tax_paid_pence":0},"starter_information":{"type":"checklist","declaration":"A"}})
}
pub fn run() -> Value {
    json!({"id":P,"paye_scheme_id":P,"pay_schedule_id":P,"payment_date":"2026-09-30","status":"draft","version":0,"revision_id":null,"available_actions":[],"employment_count":1,"finalized_at":null,"finalized_by_user_id":null,"created_at":DATE})
}

pub fn password_file() -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "secret\r\n").unwrap();
    file
}
