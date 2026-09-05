//! Commands for payroll runs belonging to a business.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{
        BusinessId, CreatePayrollRunRequest, EmploymentId, PayeSchemeId, PayrollPaymentDate,
        PayrollRunId,
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
            payment_date,
        } => print_json(
            &client
                .create_payroll_run(
                    business_id,
                    &CreatePayrollRunRequest {
                        paye_scheme_id,
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
    }

    Ok(())
}
