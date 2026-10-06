//! Commands for practice invitations received by the authenticated user.

use clap::Subcommand;
use opsd::{OpsdClient, types::PracticeInvitationId};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub(crate) enum PracticeInvitationsCommand {
    /// List pending invitations sent to the authenticated user.
    List,
    /// Accept a pending invitation.
    Accept {
        /// Public ID of the invitation.
        invitation_id: PracticeInvitationId,
    },
    /// Decline a pending invitation.
    Decline {
        /// Public ID of the invitation.
        invitation_id: PracticeInvitationId,
    },
}

/// Executes a command for invitations received by the authenticated user.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: PracticeInvitationsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PracticeInvitationsCommand::List => {
            print_json(&client.list_pending_practice_invitations().await?)?
        }
        PracticeInvitationsCommand::Accept { invitation_id } => {
            print_json(&client.accept_practice_invitation(invitation_id).await?)?
        }
        PracticeInvitationsCommand::Decline { invitation_id } => {
            client.decline_practice_invitation(invitation_id).await?;
        }
    }

    Ok(())
}
