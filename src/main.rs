use std::io;

use clap::{CommandFactory, Parser, Subcommand, builder::styling::AnsiColor};
use clap_complete::{Shell, generate};
use opsd::{ApiCredential, CreateUserRequest, OpsdClient};
use serde::Serialize;
use url::Url;

mod auth;

#[derive(Debug, Parser)]
#[command(name = "opsctl")]
#[command(about = "CLI for the Opsd API")]
#[command(version)]
struct Cli {
    /// Override the default Opsd server URL.
    #[arg(long)]
    base_url: Option<Url>,

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
    /// Manage users.
    Users {
        #[command(subcommand)]
        command: UsersCommand,
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
enum UsersCommand {
    /// List users.
    List,
    /// Create a user.
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
    },
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

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => auth::login(&server_url).await?,
            AuthCommand::Status => auth::print_status(&server_url)?,
            AuthCommand::Logout => auth::logout(&server_url).await?,
        },
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
        Command::Users { command } => {
            let client = authenticated_client(&server_url)?;
            match command {
                UsersCommand::List => {
                    let response = client.list_users().await?;
                    print_json(&response)?;
                }
                UsersCommand::Create { name, email } => {
                    let response = client
                        .create_user(&CreateUserRequest { name, email })
                        .await?;
                    print_json(&response)?;
                }
            }
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

fn print_json<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
