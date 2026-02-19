use clap::{Parser, Subcommand};
use intervals::types::IntervalConfig;
use storage::Storage;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "rt-cli", about = "Pace Buddy – DB inspection utility")]
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
    /// Export raw streams for an activity as JSON (for test fixtures)
    DumpStreams {
        /// The activity UUID
        activity_id: Uuid,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run interval parsing on an activity
    Intervals {
        /// The activity UUID
        activity_id: Uuid,
        /// MAS in km/h (optional, for %MAS computation)
        #[arg(long)]
        mas: Option<f64>,
        /// Output format: summary, json, debug
        #[arg(long, default_value = "summary")]
        format: String,
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
                let types: Vec<String> =
                    streams.iter().map(|s| s.stream_type.to_string()).collect();
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
        Command::DumpStreams {
            activity_id,
            output,
        } => {
            let streams = db.get_streams(activity_id).await?;
            if streams.is_empty() {
                anyhow::bail!("No streams found for activity {activity_id}");
            }

            let json = serde_json::to_string_pretty(&streams)?;
            if let Some(path) = output {
                std::fs::write(&path, &json)?;
                println!("Wrote {} streams to {path}", streams.len());
            } else {
                println!("{json}");
            }
        }
        Command::Intervals {
            activity_id,
            mas,
            format,
        } => {
            let streams = db.get_streams(activity_id).await?;
            if streams.is_empty() {
                anyhow::bail!("No streams found for activity {activity_id}");
            }

            let config = IntervalConfig::default();
            let result = intervals::parse_intervals(&streams, &config, mas)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                "debug" => {
                    println!("=== Clustering ===");
                    println!(
                        "  Low cluster:  {:.2} m/s ({:.1} km/h)",
                        result.cluster_low_mps,
                        result.cluster_low_mps * 3.6
                    );
                    println!(
                        "  High cluster: {:.2} m/s ({:.1} km/h)",
                        result.cluster_high_mps,
                        result.cluster_high_mps * 3.6
                    );
                    println!(
                        "  Threshold:    {:.2} m/s ({:.1} km/h)",
                        result.threshold_speed_mps,
                        result.threshold_speed_mps * 3.6
                    );
                    println!();

                    println!("=== Segments ({}) ===", result.segments.len());
                    for (i, seg) in result.segments.iter().enumerate() {
                        println!(
                            "  [{:2}] {:?} | {:.0}s-{:.0}s | dur={:.0}s dist={:.0}m | avg={:.2}m/s ({}) | std={:.2}",
                            i,
                            seg.kind,
                            seg.start_t,
                            seg.end_t,
                            seg.duration_s,
                            seg.distance_m,
                            seg.avg_speed_mps,
                            format_pace(seg.avg_speed_mps),
                            seg.speed_std_mps,
                        );
                    }
                    println!();

                    println!("=== Reps ({}) ===", result.reps.len());
                    for rep in &result.reps {
                        let mas_str = rep
                            .pct_mas
                            .map(|p| format!("{:.0}%MAS", p * 100.0))
                            .unwrap_or_default();
                        let rec_str = rep
                            .recovery_duration_s
                            .map(|r| format!("rec={:.0}s", r))
                            .unwrap_or_else(|| "no recovery".into());
                        println!(
                            "  Rep {:2}: {:.0}m in {:.0}s @ {} ({}) | steady={:.2} fade={:.2} | {rec_str}",
                            rep.rep_index,
                            rep.distance_m,
                            rep.duration_s,
                            format_pace(rep.avg_speed_mps),
                            mas_str,
                            rep.steadiness,
                            rep.fade,
                        );
                    }
                    println!();

                    println!("=== Summary ===");
                    println!("  Interval workout: {}", result.is_interval_workout);
                    println!("  Interval score:   {:.2}", result.interval_score);
                }
                _ => {
                    // summary (default)
                    if result.is_interval_workout {
                        println!(
                            "Interval workout detected ({} reps, score={:.2})",
                            result.reps.len(),
                            result.interval_score
                        );
                        println!(
                            "Threshold: {:.2} m/s ({})",
                            result.threshold_speed_mps,
                            format_pace(result.threshold_speed_mps)
                        );
                        println!();
                        println!(
                            "{:>4} {:>8} {:>8} {:>10} {:>8} {:>6} {:>6}",
                            "Rep", "Dist(m)", "Dur(s)", "Pace", "%MAS", "Steady", "Fade"
                        );
                        for rep in &result.reps {
                            let mas_str = rep
                                .pct_mas
                                .map(|p| format!("{:.0}%", p * 100.0))
                                .unwrap_or_else(|| "-".into());
                            println!(
                                "{:>4} {:>8.0} {:>8.0} {:>10} {:>8} {:>6.2} {:>6.2}",
                                rep.rep_index + 1,
                                rep.distance_m,
                                rep.duration_s,
                                format_pace(rep.avg_speed_mps),
                                mas_str,
                                rep.steadiness,
                                rep.fade,
                            );
                        }
                    } else {
                        println!(
                            "Not an interval workout ({} work segments detected)",
                            result.reps.len()
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Format m/s as min:sec /km pace string.
fn format_pace(speed_mps: f64) -> String {
    if speed_mps < 0.1 {
        return "-".into();
    }
    let pace_s_per_km = 1000.0 / speed_mps;
    let mins = (pace_s_per_km / 60.0).floor() as u32;
    let secs = (pace_s_per_km % 60.0).round() as u32;
    format!("{mins}:{secs:02}/km")
}
