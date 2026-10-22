//! Commands for payroll runs belonging to a business.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{
        BusinessId, CreatePayrollRunRequest, EmploymentId, NonNegativePence, PayScheduleId,
        PayeSchemeId, PayrollPaymentDate, PayrollRunId, UpdatePayrollPaymentRequest,
    },
};

use crate::{
    hmrc_credentials::{HmrcCredentials, read_password},
    print_json,
};
use opsd::types::*;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum PayrollRunsCommand {
    /// List a business's payroll runs.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Create a draft payroll run.
    Create {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the PAYE scheme.
        #[arg(long)]
        paye_scheme_id: PayeSchemeId,
        #[arg(long)]
        pay_schedule_id: PayScheduleId,
        /// Contractual payment date in YYYY-MM-DD format.
        #[arg(long)]
        payment_date: PayrollPaymentDate,
    },
    /// Get a payroll run and its included employments.
    Get {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the payroll run.
        payroll_run_id: PayrollRunId,
    },
    /// Include an employment in a draft payroll run.
    IncludeEmployment {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the payroll run.
        payroll_run_id: PayrollRunId,
        /// Public ID of the employment to include.
        employment_id: EmploymentId,
    },
    /// Exclude an employment from a draft payroll run.
    ExcludeEmployment {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the payroll run.
        payroll_run_id: PayrollRunId,
        /// Public ID of the employment to exclude.
        employment_id: EmploymentId,
    },
    /// Finalize a payroll run.
    Finalize {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        #[arg(long)]
        expected_version: u64,
        #[command(flatten)]
        credentials: HmrcCredentials,
        /// HMRC late-reporting reason, A through H, when applicable.
        #[arg(long)]
        late_reporting_reason: Option<LateReportingReason>,
    },
    /// Read immutable revisions, submission attempts and actions.
    History {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
    },
    /// Start another submission attempt when the saved state permits it.
    Retry {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        #[arg(long)]
        expected_version: u64,
        #[command(flatten)]
        credentials: HmrcCredentials,
    },
    /// Reopen rejected payroll for correction, when allowed.
    Reopen {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        #[arg(long)]
        expected_version: u64,
    },
    /// Cancel an eligible run, preserving its history.
    Cancel {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        #[arg(long)]
        expected_version: u64,
    },
    /// Resume a paused polling or reconciliation operation.
    Resume {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        #[arg(long)]
        expected_version: u64,
        #[command(subcommand)]
        action: ResumeArgs,
    },
    /// Calculate a draft without saving results.
    Preview {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
    },
    /// Override one draft payment. Does not change the employment's monthly default.
    UpdatePayment {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
        employment_id: EmploymentId,
        #[arg(long)]
        gross_pay_pence: Option<NonNegativePence>,
        #[arg(long)]
        normal_weekly_hours_band: Option<NormalWeeklyHoursBand>,
    },
    /// Delete a draft payroll run. Finalised runs cannot be discarded.
    Discard {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
    },
    /// List employees and their saved net amounts to pay for a finalised run.
    Payments {
        business_id: BusinessId,
        payroll_run_id: PayrollRunId,
    },
}

/// Executes a payroll-run command against the authenticated public API.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: PayrollRunsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PayrollRunsCommand::List { business_id } => {
            print_json(&client.list_payroll_runs(business_id).await?)?
        }
        PayrollRunsCommand::Create {
            business_id,
            paye_scheme_id,
            pay_schedule_id,
            payment_date,
        } => print_json(
            &client
                .create_payroll_run(
                    business_id,
                    &CreatePayrollRunRequest {
                        paye_scheme_id,
                        pay_schedule_id,
                        payment_date,
                    },
                )
                .await?,
        )?,
        PayrollRunsCommand::Get {
            business_id,
            payroll_run_id,
        } => print_json(&client.get_payroll_run(business_id, payroll_run_id).await?)?,
        PayrollRunsCommand::IncludeEmployment {
            business_id,
            payroll_run_id,
            employment_id,
        } => {
            client
                .include_payroll_run_employment(business_id, payroll_run_id, employment_id)
                .await?;
        }
        PayrollRunsCommand::ExcludeEmployment {
            business_id,
            payroll_run_id,
            employment_id,
        } => {
            client
                .exclude_payroll_run_employment(business_id, payroll_run_id, employment_id)
                .await?;
        }
        PayrollRunsCommand::Finalize {
            business_id,
            payroll_run_id,
            expected_version,
            credentials,
            late_reporting_reason,
        } => print_json(
            &client
                .finalize_payroll_run(
                    business_id,
                    payroll_run_id,
                    &FinalizePayrollRunRequest {
                        expected_version,
                        sender_id: credentials.sender_id,
                        password: read_password(&credentials.password_file)?,
                        late_reporting_reason,
                    },
                )
                .await?,
        )?,
        PayrollRunsCommand::Retry {
            business_id,
            payroll_run_id,
            expected_version,
            credentials,
        } => print_json(
            &client
                .retry_payroll_run(
                    business_id,
                    payroll_run_id,
                    &RetryPayrollRunRequest {
                        expected_version,
                        sender_id: credentials.sender_id,
                        password: read_password(&credentials.password_file)?,
                    },
                )
                .await?,
        )?,
        PayrollRunsCommand::Reopen {
            business_id,
            payroll_run_id,
            expected_version,
        } => print_json(
            &client
                .reopen_payroll_run(
                    business_id,
                    payroll_run_id,
                    &VersionedActionRequest { expected_version },
                )
                .await?,
        )?,
        PayrollRunsCommand::Cancel {
            business_id,
            payroll_run_id,
            expected_version,
        } => print_json(
            &client
                .cancel_payroll_run(
                    business_id,
                    payroll_run_id,
                    &VersionedActionRequest { expected_version },
                )
                .await?,
        )?,
        PayrollRunsCommand::History {
            business_id,
            payroll_run_id,
        } => print_json(
            &client
                .get_payroll_run_history(business_id, payroll_run_id)
                .await?,
        )?,
        PayrollRunsCommand::Resume {
            business_id,
            payroll_run_id,
            expected_version,
            action,
        } => {
            let request = match action {
                ResumeArgs::Polling => ResumePayrollRunRequest::Polling { expected_version },
                ResumeArgs::Reconciliation { password_file } => {
                    ResumePayrollRunRequest::Reconciliation {
                        expected_version,
                        password: read_password(&password_file)?,
                    }
                }
            };
            print_json(
                &client
                    .resume_payroll_run(business_id, payroll_run_id, &request)
                    .await?,
            )?;
        }
        PayrollRunsCommand::Preview {
            business_id,
            payroll_run_id,
        } => print_json(
            &client
                .preview_payroll_run(business_id, payroll_run_id)
                .await?,
        )?,
        PayrollRunsCommand::UpdatePayment {
            business_id,
            payroll_run_id,
            employment_id,
            gross_pay_pence,
            normal_weekly_hours_band,
        } => {
            let request = match (gross_pay_pence, normal_weekly_hours_band) {
                (Some(gross_pay_pence), normal_weekly_hours_band) => {
                    UpdatePayrollPaymentRequest::GrossPay {
                        gross_pay_pence,
                        normal_weekly_hours_band,
                    }
                }
                (None, Some(normal_weekly_hours_band)) => UpdatePayrollPaymentRequest::HoursBand {
                    normal_weekly_hours_band,
                },
                (None, None) => return Err("supply gross pay or normal weekly hours band".into()),
            };
            client
                .update_payroll_payment(business_id, payroll_run_id, employment_id, &request)
                .await?
        }
        PayrollRunsCommand::Discard {
            business_id,
            payroll_run_id,
        } => {
            client
                .discard_payroll_run(business_id, payroll_run_id)
                .await?
        }
        PayrollRunsCommand::Payments {
            business_id,
            payroll_run_id,
        } => {
            let result = client.get_payroll_run(business_id, payroll_run_id).await?;
            if !matches!(
                result.details.run.status,
                PayrollRunStatus::Finalized { .. } | PayrollRunStatus::Accepted
            ) {
                return Err(
                    "payment lists require a finalised run; use preview for a draft".into(),
                );
            }
            let calculation = result
                .details
                .calculation
                .ok_or("this historical run has no saved calculation")?;
            let employments = client.list_employments(business_id).await?.employments;
            let employees = client.list_employees(business_id).await?.employees;
            let mut payments = Vec::new();
            for payment in calculation.payments {
                let employment = employments
                    .iter()
                    .find(|e| e.id == payment.employment_id)
                    .ok_or("payment employment was not found")?;
                let employee = employees
                    .iter()
                    .find(|e| e.id == employment.employee_id)
                    .ok_or("payment employee was not found")?;
                payments.push(PaymentListEntry {
                    employment_id: employment.id,
                    employee_id: employee.id,
                    forenames: employee
                        .forenames
                        .as_slice()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    surname: employee.surname.to_string(),
                    net_pay_pence: payment.amounts.net_pay_pence,
                });
            }
            print_json(&PaymentList {
                payroll_run_id,
                payment_date: calculation.payment_date,
                revision_id: result.details.run.revision_id,
                status: result.details.run.status,
                payments,
                total_net_pay_pence: calculation.totals.net_pay_pence,
            })?;
        }
    }

    Ok(())
}

#[derive(Debug, Subcommand)]
pub(crate) enum ResumeArgs {
    Polling,
    Reconciliation {
        /// HMRC password file, or - for stdin, for the original sender.
        #[arg(long)]
        password_file: PathBuf,
    },
}

/// Export the saved amounts with the revision and its observed submission state.
#[derive(serde::Serialize)]
struct PaymentList {
    payroll_run_id: PayrollRunId,
    payment_date: PayrollPaymentDate,
    revision_id: Option<PayrollRevisionId>,
    #[serde(flatten)]
    status: PayrollRunStatus,
    payments: Vec<PaymentListEntry>,
    total_net_pay_pence: i64,
}

#[derive(serde::Serialize)]
struct PaymentListEntry {
    employment_id: EmploymentId,
    employee_id: EmployeeId,
    forenames: Vec<String>,
    surname: String,
    net_pay_pence: i64,
}
