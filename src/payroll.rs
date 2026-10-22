//! Separate standalone subscription and practice-client payroll vocabulary.
use crate::print_json;
use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{BusinessId, PayrollSubscriptionAction, PracticeId},
};

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessPayrollCommand {
    /// Start or resume payroll. Starts monthly base fees; does not clear suspension.
    Subscribe {
        business_id: BusinessId,
        #[arg(long)]
        expected_version: u64,
    },
    /// Read the agreement and independent suspension status.
    Status { business_id: BusinessId },
    /// Cancel payroll without clearing outstanding charges.
    Cancel {
        business_id: BusinessId,
        #[arg(long)]
        expected_version: u64,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum PracticePayrollCommand {
    /// Enable client payroll. Starts monthly base fees; does not clear suspension.
    Enable {
        practice_id: PracticeId,
        business_id: BusinessId,
        #[arg(long)]
        expected_version: u64,
    },
    /// Read client payroll status and practice-wide suspension.
    Status {
        practice_id: PracticeId,
        business_id: BusinessId,
    },
    /// Disable client payroll without clearing outstanding charges.
    Disable {
        practice_id: PracticeId,
        business_id: BusinessId,
        #[arg(long)]
        expected_version: u64,
    },
}
pub(crate) async fn execute_business(
    client: &OpsdClient,
    command: BusinessPayrollCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BusinessPayrollCommand::Subscribe {
            business_id,
            expected_version,
        } => print_json(
            &client
                .subscribe_to_payroll(business_id, &PayrollSubscriptionAction { expected_version })
                .await?,
        )?,
        BusinessPayrollCommand::Status { business_id } => {
            print_json(&client.get_payroll_subscription(business_id).await?)?
        }
        BusinessPayrollCommand::Cancel {
            business_id,
            expected_version,
        } => {
            client
                .cancel_payroll_subscription(
                    business_id,
                    &PayrollSubscriptionAction { expected_version },
                )
                .await?
        }
    }
    Ok(())
}
pub(crate) async fn execute_practice(
    client: &OpsdClient,
    command: PracticePayrollCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PracticePayrollCommand::Enable {
            practice_id,
            business_id,
            expected_version,
        } => print_json(
            &client
                .enable_practice_payroll(
                    practice_id,
                    business_id,
                    &PayrollSubscriptionAction { expected_version },
                )
                .await?,
        )?,
        PracticePayrollCommand::Status {
            practice_id,
            business_id,
        } => print_json(
            &client
                .get_practice_payroll(practice_id, business_id)
                .await?,
        )?,
        PracticePayrollCommand::Disable {
            practice_id,
            business_id,
            expected_version,
        } => {
            client
                .disable_practice_payroll(
                    practice_id,
                    business_id,
                    &PayrollSubscriptionAction { expected_version },
                )
                .await?
        }
    }
    Ok(())
}
