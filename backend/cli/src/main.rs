use clap::{Parser, Subcommand};
use storage::Storage;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "rt-cli", about = "Running Tool – DB inspection utility")]
struct Cli {
    /// Path to the SQLite database (default: sqlite:data.db?mode=rwc)
    #[arg(long, default_value = "sqlite:data.db?mode=rwc")]
    db: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all users
    Users,
    /// List all streams stored for a given user
    Streams {
        /// The user UUID
        user_id: Uuid,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db = storage::SqliteStorage::new(&cli.db).await?;

    match cli.command {
        Command::Users => {
            let users = db.list_users().await?;
            if users.is_empty() {
                println!("No users found.");
            } else {
                println!("{:<38} {:<20} {}", "ID", "USERNAME", "DISPLAY NAME");
                for u in &users {
                    println!("{:<38} {:<20} {}", u.id, u.username, u.display_name);
                }
                println!("\n{} user(s)", users.len());
            }
        }
        Command::Streams { user_id } => {
            let activities = db.get_activities(user_id, 10_000, 0).await?;
            if activities.is_empty() {
                println!("No activities found for user {user_id}.");
                return Ok(());
            }

            let mut total_streams = 0usize;
            for a in &activities {
                let streams = db.get_streams(a.id).await?;
                if streams.is_empty() {
                    continue;
                }
                total_streams += streams.len();
                let types: Vec<String> = streams.iter().map(|s| s.stream_type.to_string()).collect();
                println!(
                    "{} | {} | {} stream(s): [{}]",
                    a.id,
                    a.name,
                    streams.len(),
                    types.join(", ")
                );
            }
            println!(
                "\n{} activity(ies), {} stream(s) total",
                activities.len(),
                total_streams
            );
        }
    }

    Ok(())
}
