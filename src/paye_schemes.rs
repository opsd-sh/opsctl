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

use crate::{
    hmrc_credentials::{HmrcCredentials, read_password},
    print_json,
};
use opsd::types::*;

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
        /// Optional Corporation Tax Unique Taxpayer Reference.
        #[arg(long)]
        company_utr: Option<String>,
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
    /// The EPS declares the supplied Employment Allowance decision. An
    /// explicit retry or resume command continues an interrupted rollover.
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
        #[arg(long)]
        expected_generation: u64,
        #[command(flatten)]
        credentials: HmrcCredentials,
    },
    /// List rollovers, including completed or cancelled operations.
    Rollovers {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
    },
    /// Read progress, versions and available actions.
    RolloverStatus {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        rollover_id: PayeSchemeRolloverId,
    },
    /// Read submission attempts and saved actions.
    RolloverHistory {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        rollover_id: PayeSchemeRolloverId,
    },
    /// Retry an eligible rollover submission.
    RetryRollover {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        rollover_id: PayeSchemeRolloverId,
        #[arg(long)]
        expected_version: u64,
        #[command(flatten)]
        credentials: HmrcCredentials,
    },
    /// Cancel the specified rollover when its available actions permit it.
    CancelRollover {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        rollover_id: PayeSchemeRolloverId,
        #[arg(long)]
        expected_version: u64,
    },
    /// Resume polling, reconciliation or EPS dispatch.
    ResumeRollover {
        business_id: BusinessId,
        paye_scheme_id: PayeSchemeId,
        rollover_id: PayeSchemeRolloverId,
        #[arg(long)]
        expected_version: u64,
        #[command(subcommand)]
        action: RolloverResumeArgs,
    },
}

/// Executes a PAYE scheme command against the authenticated public API.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: PayeSchemesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PayeSchemesCommand::Rollovers {
            business_id,
            paye_scheme_id,
        } => print_json(
            &client
                .list_paye_scheme_rollovers(business_id, paye_scheme_id)
                .await?,
        )?,
        PayeSchemesCommand::RolloverStatus {
            business_id,
            paye_scheme_id,
            rollover_id,
        } => print_json(
            &client
                .get_paye_scheme_rollover(business_id, paye_scheme_id, rollover_id)
                .await?,
        )?,
        PayeSchemesCommand::RolloverHistory {
            business_id,
            paye_scheme_id,
            rollover_id,
        } => print_json(
            &client
                .get_paye_scheme_rollover_history(business_id, paye_scheme_id, rollover_id)
                .await?,
        )?,
        PayeSchemesCommand::RetryRollover {
            business_id,
            paye_scheme_id,
            rollover_id,
            expected_version,
            credentials,
        } => print_json(
            &client
                .retry_paye_scheme_rollover(
                    business_id,
                    paye_scheme_id,
                    rollover_id,
                    &RetryRolloverRequest {
                        expected_version,
                        sender_id: credentials.sender_id,
                        password: read_password(&credentials.password_file)?,
                    },
                )
                .await?,
        )?,
        PayeSchemesCommand::CancelRollover {
            business_id,
            paye_scheme_id,
            rollover_id,
            expected_version,
        } => print_json(
            &client
                .cancel_paye_scheme_rollover(
                    business_id,
                    paye_scheme_id,
                    rollover_id,
                    &VersionedActionRequest { expected_version },
                )
                .await?,
        )?,
        PayeSchemesCommand::ResumeRollover {
            business_id,
            paye_scheme_id,
            rollover_id,
            expected_version,
            action,
        } => {
            let request = match action {
                RolloverResumeArgs::Polling => ResumeRolloverRequest::Polling { expected_version },
                RolloverResumeArgs::Reconciliation { password_file } => {
                    ResumeRolloverRequest::Reconciliation {
                        expected_version,
                        password: read_password(&password_file)?,
                    }
                }
                RolloverResumeArgs::SendEps { credentials } => ResumeRolloverRequest::SendEps {
                    expected_version,
                    sender_id: credentials.sender_id,
                    password: read_password(&credentials.password_file)?,
                },
            };
            print_json(
                &client
                    .resume_paye_scheme_rollover(business_id, paye_scheme_id, rollover_id, &request)
                    .await?,
            )?;
        }
        PayeSchemesCommand::List { business_id } => {
            print_json(&client.list_paye_schemes(business_id).await?)?
        }
        PayeSchemesCommand::Create {
            business_id,
            name,
            company_utr,
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
                        company_utr,
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
            expected_generation,
            credentials,
        } => {
            let notices: Vec<RolloverTaxCodeNotice> =
                serde_json::from_reader(std::fs::File::open(tax_code_notices)?)?;
            print_json(
                &client
                    .rollover_paye_scheme(
                        business_id,
                        paye_scheme_id,
                        &RolloverPayeSchemeRequest {
                            expected_generation,
                            sender_id: credentials.sender_id,
                            password: read_password(&credentials.password_file)?,
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

#[derive(Debug, Subcommand)]
pub(crate) enum RolloverResumeArgs {
    Polling,
    Reconciliation {
        #[arg(long)]
        password_file: PathBuf,
    },
    SendEps {
        #[command(flatten)]
        credentials: HmrcCredentials,
    },
}
