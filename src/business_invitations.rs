//! Commands for business invitations received by the authenticated user.

use clap::Subcommand;
use opsd::{OpsdClient, types::BusinessInvitationId};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessInvitationsCommand {
    /// List pending invitations sent to the authenticated user.
    List,
    /// Accept a pending invitation.
    Accept {
        /// Public ID of the invitation.
        invitation_id: BusinessInvitationId,
    },
    /// Decline a pending invitation.
    Decline {
        /// Public ID of the invitation.
        invitation_id: BusinessInvitationId,
    },
}

/// Executes a command for invitations received by the authenticated user.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: BusinessInvitationsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BusinessInvitationsCommand::List => {
            print_json(&client.list_pending_business_invitations().await?)?
        }
        BusinessInvitationsCommand::Accept { invitation_id } => {
            print_json(&client.accept_business_invitation(invitation_id).await?)?
        }
        BusinessInvitationsCommand::Decline { invitation_id } => {
            client.decline_business_invitation(invitation_id).await?;
        }
    }

    Ok(())
}
