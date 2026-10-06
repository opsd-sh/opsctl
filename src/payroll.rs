//! Separate standalone subscription and practice-client payroll vocabulary.
use crate::print_json;
use clap::Subcommand;
use opsd::{
    OpsdClient,
    types::{BusinessId, PracticeId},
};

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessPayrollCommand {
    /// Start or resume payroll. Does not itself start billing or clear suspension.
    Subscribe { business_id: BusinessId },
    /// Read the agreement and independent suspension status.
    Status { business_id: BusinessId },
    /// Cancel payroll without clearing outstanding charges.
    Cancel { business_id: BusinessId },
}
#[derive(Debug, Subcommand)]
pub(crate) enum PracticePayrollCommand {
    /// Enable client payroll. Does not itself start billing or clear suspension.
    Enable {
        practice_id: PracticeId,
        business_id: BusinessId,
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
    },
}
pub(crate) async fn execute_business(
    client: &OpsdClient,
    command: BusinessPayrollCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BusinessPayrollCommand::Subscribe { business_id } => {
            print_json(&client.subscribe_to_payroll(business_id).await?)?
        }
        BusinessPayrollCommand::Status { business_id } => {
            print_json(&client.get_payroll_subscription(business_id).await?)?
        }
        BusinessPayrollCommand::Cancel { business_id } => {
            client.cancel_payroll_subscription(business_id).await?
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
        } => print_json(
            &client
                .enable_practice_payroll(practice_id, business_id)
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
        } => {
            client
                .disable_practice_payroll(practice_id, business_id)
                .await?
        }
    }
    Ok(())
}
