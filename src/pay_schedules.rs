//! Monthly pay schedules beneath a PAYE scheme.
use crate::print_json;
use clap::Subcommand;
use opsd::{OpsdClient, types::*};
#[derive(Debug, Subcommand)]
pub(crate) enum PaySchedulesCommand {
    /// List schedules belonging to a business.
    List { business_id: BusinessId },
    /// Create a schedule. Only monthly is currently supported.
    Create {
        business_id: BusinessId,
        #[arg(long)]
        paye_scheme_id: PayeSchemeId,
        #[arg(long)]
        name: PayScheduleName,
        #[arg(long, default_value = "monthly")]
        frequency: PayFrequency,
    },
}
pub(crate) async fn execute(
    client: &OpsdClient,
    command: PaySchedulesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PaySchedulesCommand::List { business_id } => {
            print_json(&client.list_pay_schedules(business_id).await?)?
        }
        PaySchedulesCommand::Create {
            business_id,
            paye_scheme_id,
            name,
            frequency,
        } => print_json(
            &client
                .create_pay_schedule(
                    business_id,
                    &CreatePayScheduleRequest {
                        paye_scheme_id,
                        name,
                        frequency,
                    },
                )
                .await?,
        )?,
    }
    Ok(())
}
