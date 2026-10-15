//! Commands for payroll runs belonging to a business.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{
        BusinessId, CreatePayrollRunRequest, EmploymentId, NonNegativePence, PayScheduleId,
        PayeSchemeId, PayrollPaymentDate, PayrollRunId, UpdatePayrollPaymentRequest,
    },
};

use crate::print_json;

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
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the payroll run.
        payroll_run_id: PayrollRunId,
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
        gross_pay_pence: NonNegativePence,
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
        } => print_json(
            &client
                .finalize_payroll_run(business_id, payroll_run_id)
                .await?,
        )?,
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
        } => {
            client
                .update_payroll_payment(
                    business_id,
                    payroll_run_id,
                    employment_id,
                    &UpdatePayrollPaymentRequest { gross_pay_pence },
                )
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
            if result.details.run.status != opsd::types::PayrollRunStatus::Finalized {
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
                payments.push(
                    serde_json::json!({"employment_id":employment.id,"employee_id":employee.id,
                    "forenames":employee.forenames.as_slice().iter().map(|name|name.as_str()).collect::<Vec<_>>(),"surname":employee.surname.as_str(),
                    "net_pay_pence":payment.amounts.net_pay_pence}),
                );
            }
            print_json(
                &serde_json::json!({"payroll_run_id":payroll_run_id,"payment_date":calculation.payment_date,
                "payments":payments,"total_net_pay_pence":calculation.totals.net_pay_pence}),
            )?;
        }
    }

    Ok(())
}
