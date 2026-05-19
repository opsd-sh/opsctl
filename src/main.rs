use clap::{
    Parser, Subcommand,
    builder::styling::AnsiColor,
};
use opsd_rust::{CreateUserRequest, OpsdClient};
use serde::Serialize;
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "ops")]
#[command(about = "CLI for the Opsd API")]
struct Cli {
    /// Override the default opsd API base URL.
    #[arg(long)]
    base_url: Option<Url>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Call the hello-world sandbox endpoint.
    HelloWorld,
    /// Manage users.
    Users {
        #[command(subcommand)]
        command: UsersCommand,
    },
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
    let client = match cli.base_url {
        Some(base_url) => OpsdClient::new_base(base_url)?,
        None => OpsdClient::new()?,
    };

    match cli.command {
        Command::HelloWorld => {
            let response = client.hello_world().await?;
            print_json(&response)?;
        }
        Command::Users { command } => match command {
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
        },
    }

    Ok(())
}

fn print_json<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
