//! Commands for employments linking employees to PAYE schemes.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{BusinessId, CreateEmploymentRequest, EmployeeId, PayeSchemeId},
};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub(crate) enum EmploymentsCommand {
    /// List a business's employments.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Create an employment linking an employee to a PAYE scheme.
    Create {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the employee.
        #[arg(long)]
        employee_id: EmployeeId,
        /// Public ID of the PAYE scheme.
        #[arg(long)]
        paye_scheme_id: PayeSchemeId,
    },
}

/// Executes an employment command against the authenticated public API.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: EmploymentsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        EmploymentsCommand::List { business_id } => {
            print_json(&client.list_employments(business_id).await?)?
        }
        EmploymentsCommand::Create {
            business_id,
            employee_id,
            paye_scheme_id,
        } => print_json(
            &client
                .create_employment(
                    business_id,
                    &CreateEmploymentRequest {
                        employee_id,
                        paye_scheme_id,
                    },
                )
                .await?,
        )?,
    }

    Ok(())
}
