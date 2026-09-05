//! Commands for businesses and resources that administer business access.

use clap::{Subcommand, ValueEnum};
use opsd::types::{
    BusinessId, BusinessInvitationId, BusinessName, BusinessRole, CreateBusinessInvitationRequest,
    CreateBusinessRequest, EmailAddress, UpdateBusinessMemberRequest, UserId,
};

use crate::{
    auth::ServerUrl,
    authenticated_client, print_json,
    website::{self, WebsiteUrl},
};

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessesCommand {
    /// List businesses available to the authenticated user.
    List,
    /// Create a business.
    Create {
        /// Business name.
        #[arg(long)]
        name: BusinessName,
    },
    /// Get a business.
    Get {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Manage billing for a business.
    Billing {
        #[command(subcommand)]
        command: BusinessBillingCommand,
    },
    /// Manage business members.
    Members {
        #[command(subcommand)]
        command: BusinessMembersCommand,
    },
    /// Manage invitations sent by a business.
    Invitations {
        #[command(subcommand)]
        command: OutgoingBusinessInvitationsCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessBillingCommand {
    /// Check whether payment setup has been confirmed for the business.
    /// Requires business administrator access and prints the result as JSON.
    /// Confirmation does not guarantee that a future charge will succeed.
    Status {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Open the website to set up billing details.
    Setup {
        /// Public ID of the business to configure.
        business_id: BusinessId,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum BusinessMembersCommand {
    /// List users with access to a business.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Change a business member's role.
    Update {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the user whose role will change.
        user_id: UserId,
        /// Access role to grant.
        #[arg(long, value_enum)]
        role: BusinessRoleArgument,
    },
    /// Remove a user's access to a business.
    Remove {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the user to remove.
        user_id: UserId,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum OutgoingBusinessInvitationsCommand {
    /// List pending invitations sent by a business.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Invite an email address to join a business.
    Create {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Email address to invite.
        #[arg(long)]
        email: EmailAddress,
        /// Access role to grant when the invitation is accepted.
        #[arg(long, value_enum)]
        role: BusinessRoleArgument,
    },
    /// Cancel a pending invitation sent by a business.
    Cancel {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the invitation.
        invitation_id: BusinessInvitationId,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum BusinessRoleArgument {
    Admin,
    PayrollOperator,
}

impl From<BusinessRoleArgument> for BusinessRole {
    fn from(value: BusinessRoleArgument) -> Self {
        match value {
            BusinessRoleArgument::Admin => Self::Admin,
            BusinessRoleArgument::PayrollOperator => Self::PayrollOperator,
        }
    }
}

/// Executes a business command, authenticating only commands that call the API.
pub(crate) async fn execute(
    command: BusinessesCommand,
    server_url: &ServerUrl,
    website_url: &WebsiteUrl,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BusinessesCommand::List => {
            let client = authenticated_client(server_url)?;
            print_json(&client.list_businesses().await?)?;
        }
        BusinessesCommand::Create { name } => {
            let client = authenticated_client(server_url)?;
            print_json(
                &client
                    .create_business(&CreateBusinessRequest { name })
                    .await?,
            )?;
        }
        BusinessesCommand::Get { business_id } => {
            let client = authenticated_client(server_url)?;
            print_json(&client.get_business(business_id).await?)?
        }
        BusinessesCommand::Members { command } => {
            let client = authenticated_client(server_url)?;
            match command {
                BusinessMembersCommand::List { business_id } => {
                    print_json(&client.list_business_members(business_id).await?)?
                }
                BusinessMembersCommand::Update {
                    business_id,
                    user_id,
                    role,
                } => {
                    client
                        .update_business_member(
                            business_id,
                            user_id,
                            &UpdateBusinessMemberRequest { role: role.into() },
                        )
                        .await?;
                }
                BusinessMembersCommand::Remove {
                    business_id,
                    user_id,
                } => client.remove_business_member(business_id, user_id).await?,
            }
        }
        BusinessesCommand::Invitations { command } => {
            let client = authenticated_client(server_url)?;
            match command {
                OutgoingBusinessInvitationsCommand::List { business_id } => print_json(
                    &client
                        .list_outgoing_business_invitations(business_id)
                        .await?,
                )?,
                OutgoingBusinessInvitationsCommand::Create {
                    business_id,
                    email,
                    role,
                } => print_json(
                    &client
                        .create_business_invitation(
                            business_id,
                            &CreateBusinessInvitationRequest {
                                email,
                                role: role.into(),
                            },
                        )
                        .await?,
                )?,
                OutgoingBusinessInvitationsCommand::Cancel {
                    business_id,
                    invitation_id,
                } => {
                    client
                        .cancel_business_invitation(business_id, invitation_id)
                        .await?;
                }
            }
        }
        BusinessesCommand::Billing { command } => match command {
            BusinessBillingCommand::Status { business_id } => {
                let client = authenticated_client(server_url)?;
                print_json(&client.get_billing_status(business_id).await?)?
            }
            BusinessBillingCommand::Setup { business_id } => {
                website::open_billing_setup(website_url, &business_id.to_string());
            }
        },
    }

    Ok(())
}
