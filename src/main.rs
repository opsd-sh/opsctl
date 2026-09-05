use std::io;

use clap::{CommandFactory, Parser, Subcommand, builder::styling::AnsiColor};
use clap_complete::{Shell, generate};
use opsd::{ApiCredential, OpsdClient};
use serde::Serialize;
use url::Url;

mod auth;
mod business_invitations;
mod businesses;
mod employees;
mod employments;
mod paye_schemes;
mod payroll_runs;
mod website;

#[derive(Debug, Parser)]
#[command(name = "opsctl")]
#[command(about = "CLI for the Opsd API")]
#[command(version)]
struct Cli {
    /// Override the default Opsd server URL.
    #[arg(long)]
    base_url: Option<Url>,
    /// Override the default Opsd website URL.
    #[arg(long)]
    website_url: Option<Url>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Authenticate this CLI.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Manage businesses.
    Businesses {
        #[command(subcommand)]
        command: businesses::BusinessesCommand,
    },
    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },
    /// Call hello endpoints.
    Hello {
        #[command(subcommand)]
        command: HelloCommand,
    },
    /// Manage invitations sent to the authenticated user.
    Invitations {
        #[command(subcommand)]
        command: business_invitations::BusinessInvitationsCommand,
    },
    /// Manage employees.
    Employees {
        #[command(subcommand)]
        command: employees::EmployeesCommand,
    },
    /// Manage employments.
    Employments {
        #[command(subcommand)]
        command: employments::EmploymentsCommand,
    },
    /// Manage PAYE schemes.
    PayeSchemes {
        #[command(subcommand)]
        command: paye_schemes::PayeSchemesCommand,
    },
    /// Manage payroll runs.
    PayrollRuns {
        #[command(subcommand)]
        command: payroll_runs::PayrollRunsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    /// Sign in through the Opsd website.
    Login,
    /// Show whether this CLI has a usable credential.
    Status,
    /// Revoke and remove the saved credential.
    Logout,
}

#[derive(Debug, Subcommand)]
enum HelloCommand {
    /// Call the hello-world sandbox endpoint.
    World,
    /// Call the application-restricted hello-application sandbox endpoint.
    Application,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let error_style = AnsiColor::Red.on_default().bold();
        eprintln!(
            "{}error:{} {error}",
            error_style.render(),
            error_style.render_reset()
        );
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let server_url = match cli.base_url {
        Some(base_url) => auth::ServerUrl::from_override(base_url)?,
        None => auth::ServerUrl::production(),
    };
    let website_url = match cli.website_url {
        Some(website_url) => website::WebsiteUrl::from_override(website_url)?,
        None => website::WebsiteUrl::production(),
    };

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => auth::login(&server_url).await?,
            AuthCommand::Status => auth::print_status(&server_url)?,
            AuthCommand::Logout => auth::logout(&server_url).await?,
        },
        Command::Businesses { command } => {
            businesses::execute(command, &server_url, &website_url).await?
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            let bin_name = command.get_name().to_string();
            generate(shell, &mut command, bin_name, &mut io::stdout());
        }
        Command::Hello { command } => {
            let client = authenticated_client(&server_url)?;
            match command {
                HelloCommand::World => {
                    let response = client.hello_world().await?;
                    print_json(&response)?;
                }
                HelloCommand::Application => {
                    let response = client.hello_application().await?;
                    print_json(&response)?;
                }
            }
        }
        Command::Invitations { command } => {
            let client = authenticated_client(&server_url)?;
            business_invitations::execute(&client, command).await?;
        }
        Command::Employees { command } => {
            let client = authenticated_client(&server_url)?;
            employees::execute(&client, command).await?;
        }
        Command::Employments { command } => {
            let client = authenticated_client(&server_url)?;
            employments::execute(&client, command).await?;
        }
        Command::PayeSchemes { command } => {
            let client = authenticated_client(&server_url)?;
            paye_schemes::execute(&client, command).await?;
        }
        Command::PayrollRuns { command } => {
            let client = authenticated_client(&server_url)?;
            payroll_runs::execute(&client, command).await?;
        }
    }

    Ok(())
}

fn authenticated_client(
    server_url: &auth::ServerUrl,
) -> Result<OpsdClient, Box<dyn std::error::Error>> {
    let token = auth::access_token(server_url)?;
    let credential = ApiCredential::new(token.into_inner())?;

    Ok(OpsdClient::new_base(
        server_url.public_api_base_url(),
        credential,
    )?)
}

/// Writes a successful API response as pretty-printed JSON.
pub(crate) fn print_json<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Cli;

    const BUSINESS_ID: &str = "11111111-1111-4111-8111-111111111111";
    const USER_ID: &str = "22222222-2222-4222-8222-222222222222";
    const INVITATION_ID: &str = "33333333-3333-4333-8333-333333333333";
    const EMPLOYEE_ID: &str = "44444444-4444-4444-8444-444444444444";
    const EMPLOYMENT_ID: &str = "55555555-5555-4555-8555-555555555555";
    const PAYE_SCHEME_ID: &str = "66666666-6666-4666-8666-666666666666";
    const PAYROLL_RUN_ID: &str = "77777777-7777-4777-8777-777777777777";

    #[test]
    fn command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn business_and_payroll_commands_accept_the_documented_arguments() {
        let commands = vec![
            vec!["opsctl", "businesses", "list"],
            vec!["opsctl", "businesses", "create", "--name", "Acme Ltd"],
            vec!["opsctl", "businesses", "get", BUSINESS_ID],
            vec!["opsctl", "businesses", "billing", "setup", BUSINESS_ID],
            vec!["opsctl", "businesses", "billing", "status", BUSINESS_ID],
            vec!["opsctl", "businesses", "members", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "businesses",
                "members",
                "update",
                BUSINESS_ID,
                USER_ID,
                "--role",
                "payroll-operator",
            ],
            vec![
                "opsctl",
                "businesses",
                "members",
                "remove",
                BUSINESS_ID,
                USER_ID,
            ],
            vec!["opsctl", "businesses", "invitations", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "businesses",
                "invitations",
                "create",
                BUSINESS_ID,
                "--email",
                "payroll@example.com",
                "--role",
                "admin",
            ],
            vec![
                "opsctl",
                "businesses",
                "invitations",
                "cancel",
                BUSINESS_ID,
                INVITATION_ID,
            ],
            vec!["opsctl", "invitations", "list"],
            vec!["opsctl", "invitations", "accept", INVITATION_ID],
            vec!["opsctl", "invitations", "decline", INVITATION_ID],
            vec!["opsctl", "employees", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "employees",
                "create",
                BUSINESS_ID,
                "--forenames",
                "Ada",
                "Augusta",
                "--surname",
                "Lovelace",
            ],
            vec!["opsctl", "employees", "get", BUSINESS_ID, EMPLOYEE_ID],
            vec![
                "opsctl",
                "employees",
                "update",
                BUSINESS_ID,
                EMPLOYEE_ID,
                "--surname",
                "Byron",
            ],
            vec!["opsctl", "employments", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "employments",
                "create",
                BUSINESS_ID,
                "--employee-id",
                EMPLOYEE_ID,
                "--paye-scheme-id",
                PAYE_SCHEME_ID,
            ],
            vec!["opsctl", "paye-schemes", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "paye-schemes",
                "create",
                BUSINESS_ID,
                "--name",
                "Monthly payroll",
                "--employer-reference",
                "123/AB456",
                "--accounts-office-reference",
                "123PA00045678",
            ],
            vec!["opsctl", "payroll-runs", "list", BUSINESS_ID],
            vec![
                "opsctl",
                "payroll-runs",
                "create",
                BUSINESS_ID,
                "--paye-scheme-id",
                PAYE_SCHEME_ID,
                "--payment-date",
                "2026-09-30",
            ],
            vec!["opsctl", "payroll-runs", "get", BUSINESS_ID, PAYROLL_RUN_ID],
            vec![
                "opsctl",
                "payroll-runs",
                "include-employment",
                BUSINESS_ID,
                PAYROLL_RUN_ID,
                EMPLOYMENT_ID,
            ],
            vec![
                "opsctl",
                "payroll-runs",
                "exclude-employment",
                BUSINESS_ID,
                PAYROLL_RUN_ID,
                EMPLOYMENT_ID,
            ],
            vec![
                "opsctl",
                "payroll-runs",
                "finalize",
                BUSINESS_ID,
                PAYROLL_RUN_ID,
            ],
        ];

        for arguments in commands {
            Cli::try_parse_from(&arguments)
                .unwrap_or_else(|error| panic!("{arguments:?} did not parse: {error}"));
        }
    }

    #[test]
    fn employee_updates_accept_either_or_both_name_fields() {
        for (options, expected_forenames, expected_surname) in [
            (vec!["--surname", "Lovelace"], vec![], Some("Lovelace")),
            (
                vec!["--forenames", "Ada", "Augusta"],
                vec!["Ada", "Augusta"],
                None,
            ),
            (
                vec!["--forenames", "Ada", "Augusta", "--surname", "Lovelace"],
                vec!["Ada", "Augusta"],
                Some("Lovelace"),
            ),
        ] {
            let mut arguments = vec!["opsctl", "employees", "update", BUSINESS_ID, EMPLOYEE_ID];
            arguments.extend(options);
            let cli = Cli::try_parse_from(&arguments).unwrap();
            let super::Command::Employees {
                command:
                    crate::employees::EmployeesCommand::Update {
                        forenames, surname, ..
                    },
            } = cli.command
            else {
                panic!("expected an employee update");
            };
            assert_eq!(forenames, expected_forenames);
            assert_eq!(
                surname.as_ref().map(|value| value.as_str()),
                expected_surname
            );
        }
    }

    #[test]
    fn employee_forenames_preserve_spaces_and_require_values() {
        let base = ["opsctl", "employees", "create", BUSINESS_ID];
        let cli = Cli::try_parse_from(base.into_iter().chain([
            "--forenames",
            "Mary Ann",
            "Louise",
            "--surname",
            "Smith",
        ]))
        .unwrap();
        let super::Command::Employees {
            command: crate::employees::EmployeesCommand::Create { forenames, .. },
        } = cli.command
        else {
            panic!("expected employee creation");
        };
        assert_eq!(forenames, ["Mary Ann", "Louise"]);
        assert!(
            Cli::try_parse_from(
                base.into_iter()
                    .chain(["--forenames", "--surname", "Smith",])
            )
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "opsctl",
                "employees",
                "update",
                BUSINESS_ID,
                EMPLOYEE_ID,
                "--forenames",
                "--surname",
                "Smith",
            ])
            .is_err()
        );
    }

    #[test]
    fn commands_reject_invalid_domain_values_and_empty_employee_updates() {
        assert!(Cli::try_parse_from(["opsctl", "businesses", "get", "not-a-uuid"]).is_err());
        assert!(
            Cli::try_parse_from([
                "opsctl",
                "businesses",
                "members",
                "update",
                BUSINESS_ID,
                USER_ID,
                "--role",
                "viewer",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from(["opsctl", "employees", "update", BUSINESS_ID, EMPLOYEE_ID,])
                .is_err()
        );
    }
}
