//! Commands for PAYE schemes belonging to a business.

use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{
        AccountsOfficeReference, BusinessId, CreatePayeSchemeRequest, EmployerReference,
        EmploymentAllowance, PayeSchemeId, PayeSchemeName, RolloverPayeSchemeRequest,
        RolloverTaxCodeNotice,
    },
};

use std::path::PathBuf;

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
        /// Initial tax year, e.g. 2026 for 2026-27.
        #[arg(long)]
        tax_year_start: i32,
        #[arg(long)]
        employment_allowance: EmploymentAllowance,
    },
    /// Request year rollover, including an end-of-year FPS and a new-year EPS.
    ///
    /// The EPS declares the supplied Employment Allowance decision. Retry the
    /// same request to resume an interrupted rollover.
    Rollover {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        #[arg(long)]
        tax_year_start: i32,
        #[arg(long)]
        employment_allowance: EmploymentAllowance,
        /// JSON file containing the latest new-year notices, or [] after checking none apply.
        #[arg(long)]
        tax_code_notices: PathBuf,
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
            tax_year_start,
            employment_allowance,
        } => print_json(
            &client
                .create_paye_scheme(
                    business_id,
                    &CreatePayeSchemeRequest {
                        name,
                        employer_reference,
                        accounts_office_reference,
                        tax_year_start,
                        employment_allowance,
                    },
                )
                .await?,
        )?,
        PayeSchemesCommand::Rollover {
            business_id,
            paye_scheme_id,
            tax_year_start,
            employment_allowance,
            tax_code_notices,
        } => {
            let notices: Vec<RolloverTaxCodeNotice> =
                serde_json::from_reader(std::fs::File::open(tax_code_notices)?)?;
            print_json(
                &client
                    .rollover_paye_scheme(
                        business_id,
                        paye_scheme_id,
                        &RolloverPayeSchemeRequest {
                            tax_year_start,
                            employment_allowance,
                            tax_code_notices: notices,
                        },
                    )
                    .await?,
            )?;
        }
    }

    Ok(())
}
