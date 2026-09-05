//! Commands for PAYE schemes belonging to a business.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{
        AccountsOfficeReference, BusinessId, CreatePayeSchemeRequest, EmployerReference,
        PayeSchemeName,
    },
};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub(crate) enum PayeSchemesCommand {
    /// List a business's PAYE schemes.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Create a PAYE scheme.
    Create {
        /// Public ID of the business.
        business_id: BusinessId,
        /// User-facing name of the PAYE scheme.
        #[arg(long)]
        name: PayeSchemeName,
        /// Employer PAYE reference.
        #[arg(long)]
        employer_reference: EmployerReference,
        /// Accounts Office reference.
        #[arg(long)]
        accounts_office_reference: AccountsOfficeReference,
    },
}

/// Executes a PAYE scheme command against the authenticated public API.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: PayeSchemesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PayeSchemesCommand::List { business_id } => {
            print_json(&client.list_paye_schemes(business_id).await?)?
        }
        PayeSchemesCommand::Create {
            business_id,
            name,
            employer_reference,
            accounts_office_reference,
        } => print_json(
            &client
                .create_paye_scheme(
                    business_id,
                    &CreatePayeSchemeRequest {
                        name,
                        employer_reference,
                        accounts_office_reference,
                    },
                )
                .await?,
        )?,
    }

    Ok(())
}
