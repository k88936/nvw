use anyhow::{Context, Result};
use clap::{Parser, Subcommand, Args};
use proto::{TaskStatus, TaskPayload};
use uuid::Uuid;

mod client;
use client::ApiClient;

#[derive(Parser)]
#[command(name = "nvw-cli", about = "CLI for managing tasks")]
struct Cli {
    #[arg(long, env = "NVW_SERVER_URL", default_value = "http://localhost:3000")]
    server_url: String,

    #[arg(long, env = "NVW_TOKEN", default_value = "k88936")]
    token: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new task
    Create(CreateTaskArgs),

    /// List all tasks
    List,

    /// Get a specific task
    Get { id: Uuid },

    /// Update a task
    Update {
        id: Uuid,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        payload: Option<String>,
    },

    /// Delete a task
    Delete { id: Uuid },
}

#[derive(Args)]
struct CreateTaskArgs {
    /// Task payload as JSON string
    #[arg(long)]
    payload: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = ApiClient::new(&cli.server_url, &cli.token)?;

    match cli.command {
        Commands::Create(args) => {
            let payload: TaskPayload = serde_json::from_str(&args.payload)
                .context("Failed to parse payload JSON")?;
            let task = client.create_task(payload).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        Commands::List => {
            let tasks = client.list_tasks().await?;
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }
        Commands::Get { id } => {
            let task = client.get_task(id).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        Commands::Update { id, status, payload } => {
            let status_enum = if let Some(s) = status {
                match s.to_lowercase().as_str() {
                    "pending" => Some(TaskStatus::Pending),
                    "running" => Some(TaskStatus::Running),
                    "succeeded" => Some(TaskStatus::Succeeded),
                    "failed" => Some(TaskStatus::Failed),
                    _ => anyhow::bail!("Invalid status: {}. Must be pending, running, succeeded, or failed.", s),
                }
            } else {
                None
            };

            let payload_struct: Option<TaskPayload> = if let Some(p) = payload {
                Some(serde_json::from_str(&p).context("Failed to parse payload JSON")?)
            } else {
                None
            };

            let task = client.update_task(id, status_enum, payload_struct).await?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        Commands::Delete { id } => {
            client.delete_task(id).await?;
            println!("Task {} deleted successfully", id);
        }
    }

    Ok(())
}
