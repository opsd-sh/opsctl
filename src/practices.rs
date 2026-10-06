//! Commands for practices and resources that administer practice access.

use crate::{payroll::PracticePayrollCommand, practice_billing::PracticeBillingCommand};
use clap::{Subcommand, ValueEnum};
use opsd::types::{BusinessName, CreatePracticeBusinessRequest};
use opsd::types::{
    CreatePracticeInvitationRequest, CreatePracticeRequest, EmailAddress, PracticeId,
    PracticeInvitationId, PracticeName, PracticeRole, UpdatePracticeMemberRequest, UserId,
};

use crate::{auth::ServerUrl, authenticated_client, print_json};

#[derive(Debug, Subcommand)]
pub(crate) enum PracticesCommand {
    /// List practices available to the authenticated user.
    List,
    /// Create a practice.
    Create {
        /// Practice name.
        #[arg(long)]
        name: PracticeName,
    },
    /// Get a practice.
    Get {
        /// Public ID of the practice.
        practice_id: PracticeId,
    },
    /// Manage billing for a practice.
    Billing {
        #[command(subcommand)]
        command: PracticeBillingCommand,
    },
    /// Manage payroll for a practice-owned client.
    Payroll {
        #[command(subcommand)]
        command: PracticePayrollCommand,
    },
    /// Create and list practice-owned client businesses.
    Businesses {
        #[command(subcommand)]
        command: PracticeBusinessesCommand,
    },
    /// Manage practice members.
    Members {
        #[command(subcommand)]
        command: PracticeMembersCommand,
    },
    /// Manage invitations sent by a practice.
    Invitations {
        #[command(subcommand)]
        command: OutgoingPracticeInvitationsCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PracticeBusinessesCommand {
    /// List client businesses.
    List { practice_id: PracticeId },
    /// Create a new client business with no direct users.
    Create {
        practice_id: PracticeId,
        #[arg(long)]
        name: BusinessName,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PracticeMembersCommand {
    /// List users with access to a practice.
    List {
        /// Public ID of the practice.
        practice_id: PracticeId,
    },
    /// Change a practice member's role.
    Update {
        /// Public ID of the practice.
        practice_id: PracticeId,
        /// Public ID of the user whose role will change.
        user_id: UserId,
        /// Access role to grant.
        #[arg(long, value_enum)]
        role: PracticeRoleArgument,
    },
    /// Remove a user's access to a practice.
    Remove {
        /// Public ID of the practice.
        practice_id: PracticeId,
        /// Public ID of the user to remove.
        user_id: UserId,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OutgoingPracticeInvitationsCommand {
    /// List pending invitations sent by a practice.
    List {
        /// Public ID of the practice.
        practice_id: PracticeId,
    },
    /// Invite an email address to join a practice.
    Create {
        /// Public ID of the practice.
        practice_id: PracticeId,
        /// Email address to invite.
        #[arg(long)]
        email: EmailAddress,
        /// Access role to grant when the invitation is accepted.
        #[arg(long, value_enum)]
        role: PracticeRoleArgument,
    },
    /// Cancel a pending invitation sent by a practice.
    Cancel {
        /// Public ID of the practice.
        practice_id: PracticeId,
        /// Public ID of the invitation.
        invitation_id: PracticeInvitationId,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum PracticeRoleArgument {
    Admin,
    Member,
}

impl From<PracticeRoleArgument> for PracticeRole {
    fn from(value: PracticeRoleArgument) -> Self {
        match value {
            PracticeRoleArgument::Admin => Self::Admin,
            PracticeRoleArgument::Member => Self::Member,
        }
    }
}

/// Executes a practice command, authenticating only commands that call the API.
pub(crate) async fn execute(
    command: PracticesCommand,
    server_url: &ServerUrl,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        PracticesCommand::List => {
            let client = authenticated_client(server_url)?;
            print_json(&client.list_practices().await?)?;
        }
        PracticesCommand::Create { name } => {
            let client = authenticated_client(server_url)?;
            print_json(
                &client
                    .create_practice(&CreatePracticeRequest { name })
                    .await?,
            )?;
        }
        PracticesCommand::Get { practice_id } => {
            let client = authenticated_client(server_url)?;
            print_json(&client.get_practice(practice_id).await?)?
        }
        PracticesCommand::Members { command } => {
            let client = authenticated_client(server_url)?;
            match command {
                PracticeMembersCommand::List { practice_id } => {
                    print_json(&client.list_practice_members(practice_id).await?)?
                }
                PracticeMembersCommand::Update {
                    practice_id,
                    user_id,
                    role,
                } => {
                    client
                        .update_practice_member(
                            practice_id,
                            user_id,
                            &UpdatePracticeMemberRequest { role: role.into() },
                        )
                        .await?;
                }
                PracticeMembersCommand::Remove {
                    practice_id,
                    user_id,
                } => client.remove_practice_member(practice_id, user_id).await?,
            }
        }
        PracticesCommand::Invitations { command } => {
            let client = authenticated_client(server_url)?;
            match command {
                OutgoingPracticeInvitationsCommand::List { practice_id } => print_json(
                    &client
                        .list_outgoing_practice_invitations(practice_id)
                        .await?,
                )?,
                OutgoingPracticeInvitationsCommand::Create {
                    practice_id,
                    email,
                    role,
                } => print_json(
                    &client
                        .create_practice_invitation(
                            practice_id,
                            &CreatePracticeInvitationRequest {
                                email,
                                role: role.into(),
                            },
                        )
                        .await?,
                )?,
                OutgoingPracticeInvitationsCommand::Cancel {
                    practice_id,
                    invitation_id,
                } => {
                    client
                        .cancel_practice_invitation(practice_id, invitation_id)
                        .await?;
                }
            }
        }
        PracticesCommand::Businesses { command } => {
            let client = authenticated_client(server_url)?;
            match command {
                PracticeBusinessesCommand::List { practice_id } => {
                    print_json(&client.list_practice_businesses(practice_id).await?)?
                }
                PracticeBusinessesCommand::Create { practice_id, name } => print_json(
                    &client
                        .create_practice_business(
                            practice_id,
                            &CreatePracticeBusinessRequest { name },
                        )
                        .await?,
                )?,
            }
        }
        PracticesCommand::Billing { command } => {
            crate::practice_billing::execute(command, server_url).await?;
        }
        PracticesCommand::Payroll { command } => {
            crate::payroll::execute_practice(&authenticated_client(server_url)?, command).await?;
        }
    }

    Ok(())
}
