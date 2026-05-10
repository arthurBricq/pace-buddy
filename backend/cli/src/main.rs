use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use domain::{Prescription, SessionStatus, SessionType, TrainingSession};
use intervals::fixtures::{self, FixtureCategory};
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
        /// Interval parsing algorithm
        #[arg(long, value_enum, default_value_t = IntervalAlgorithmArg::SpeedBased)]
        algorithm: IntervalAlgorithmArg,
        /// Output format: summary, json, debug
        #[arg(long, default_value = "summary")]
        format: String,
    },
    /// Run interval parsing on the labeled fixture corpus and print a score
    /// distribution by category. Use this to calibrate the confidence
    /// threshold for Phase 0.
    Calibrate {
        /// Include trail-running fixtures (out of v1 scope by default)
        #[arg(long)]
        include_trails: bool,
    },
    /// Seed a TrainingSession row from a JSON prescription file. Dev/test only.
    /// The JSON is validated via Prescription::parse before insertion.
    SeedTrainingSession {
        /// Owner of the new session (must be an existing user UUID)
        #[arg(long)]
        user: Uuid,
        /// Display title for the session
        #[arg(long)]
        title: String,
        /// Session type (e.g. intervals, tempo, threshold, hill, fartlek, ...)
        #[arg(long = "session-type")]
        session_type: String,
        /// Path to a JSON file matching the Prescription schema
        #[arg(long)]
        prescription: String,
        /// Initial status for the seeded session (default: suggested)
        #[arg(long, default_value = "suggested")]
        status: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "snake_case")]
enum IntervalAlgorithmArg {
    SpeedBased,
    ManualLaps,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Calibrate doesn't need the DB; handle before opening storage.
    if let Command::Calibrate { include_trails } = &cli.command {
        return run_calibrate(*include_trails);
    }

    let db = storage::SqliteStorage::new(&cli.db).await?;

    match cli.command {
        Command::Calibrate { .. } => unreachable!("handled above"),
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
            algorithm,
            format,
        } => {
            let config = IntervalConfig::default();
            let result = match algorithm {
                IntervalAlgorithmArg::SpeedBased => {
                    let streams = db.get_streams(activity_id).await?;
                    if streams.is_empty() {
                        anyhow::bail!("No streams found for activity {activity_id}");
                    }
                    let algorithm_impl = intervals::AutoSpeedSegmentationAlgorithm;
                    intervals::parse_intervals_with_algorithm(
                        &algorithm_impl,
                        &streams,
                        &config,
                        mas,
                    )
                }
                IntervalAlgorithmArg::ManualLaps => {
                    let laps = db.get_laps(activity_id).await?;
                    if laps.is_empty() {
                        anyhow::bail!("No laps found for activity {activity_id}");
                    }
                    let algorithm_impl = intervals::ManualLapIntervalAlgorithm::new(&laps);
                    intervals::parse_intervals_with_algorithm(&algorithm_impl, &[], &config, mas)
                }
            }
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                "debug" => {
                    println!("=== Clustering ===");
                    println!(
                        "  Low cluster:  {:.2} mps ({:.1} km/h)",
                        result.cluster_low_mps,
                        result.cluster_low_mps * 3.6
                    );
                    println!(
                        "  High cluster: {:.2} mps ({:.1} km/h)",
                        result.cluster_high_mps,
                        result.cluster_high_mps * 3.6
                    );
                    println!(
                        "  Threshold:    {:.2} mps ({:.1} km/h)",
                        result.threshold_speed_mps,
                        result.threshold_speed_mps * 3.6
                    );
                    println!();

                    println!("=== Segments ({}) ===", result.segments.len());
                    for (i, seg) in result.segments.iter().enumerate() {
                        println!(
                            "  [{:2}] {:?} | {:.0}s-{:.0}s | dur={:.0}s dist={:.0}m | avg={:.2}mps ({}) | std={:.2}",
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
                            "Threshold: {:.2} mps ({})",
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
        Command::SeedTrainingSession {
            user,
            title,
            session_type,
            prescription,
            status,
        } => {
            let raw = std::fs::read_to_string(&prescription)
                .map_err(|e| anyhow::anyhow!("read {prescription}: {e}"))?;
            // Validate the prescription up-front so we don't insert garbage.
            Prescription::parse(&raw)
                .map_err(|e| anyhow::anyhow!("invalid prescription JSON: {e}"))?;

            let session_type: SessionType = session_type
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid --session-type: {e}"))?;
            let status: SessionStatus = status
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid --status: {e}"))?;

            let now = Utc::now();
            let session = TrainingSession {
                id: Uuid::new_v4(),
                user_id: user,
                training_id: None,
                status,
                title,
                session_type,
                expiry: None,
                estimated_duration_s: None,
                estimated_distance_m: None,
                intensity_summary: None,
                prescription_json: raw,
                coach_message_id: None,
                created_at: now,
                updated_at: now,
            };

            db.create_training_session(&session).await?;
            println!("Seeded training session {}", session.id);
        }
    }

    Ok(())
}

/// Format speed (mps) as min:sec /km pace string.
fn format_pace(speed_mps: f64) -> String {
    if speed_mps < 0.1 {
        return "-".into();
    }
    let pace_s_per_km = 1000.0 / speed_mps;
    let mins = (pace_s_per_km / 60.0).floor() as u32;
    let secs = (pace_s_per_km % 60.0).round() as u32;
    format!("{mins}:{secs:02}/km")
}

fn run_calibrate(include_trails: bool) -> anyhow::Result<()> {
    let dir = fixtures::default_fixtures_dir();
    let mut all = fixtures::load_fixtures(&dir)?;
    if !include_trails {
        all.retain(|f| f.category.in_scope_v1());
    }

    if all.is_empty() {
        eprintln!("No fixtures found under {}", dir.display());
        return Ok(());
    }

    println!(
        "Calibrating on {} fixture(s) from {}\n",
        all.len(),
        dir.display()
    );

    let config = IntervalConfig::default();
    let algorithm = intervals::AutoSpeedSegmentationAlgorithm;

    println!(
        "{:<10} {:<5} {:<6} {:<6} {:<6} {:<6} {:<6} {:<6} {:<6} {:<6} {}",
        "category",
        "score",
        "lo_kmh",
        "hi_kmh",
        "ratio",
        "reps",
        "work%",
        "rec%",
        "ovr_cv",
        "rec_dur",
        "name"
    );
    println!("{}", "-".repeat(120));

    #[derive(Clone)]
    struct Row {
        category: FixtureCategory,
        score: f64,
        ratio: f64,
        gap_mps: f64,
        low_kmh: f64,
        reps: usize,
        work_pct: f64,
        rec_pct: f64,
        overall_cv: f64,
        median_rec_dur: f64,
    }
    let mut rows: Vec<Row> = Vec::new();

    for f in &all {
        let result = intervals::parse_intervals_with_algorithm(
            &algorithm,
            &f.dump.streams,
            &config,
            None,
        );
        let r = match result {
            Ok(r) => r,
            Err(e) => {
                println!(
                    "{:<10} ERROR: {} ({})",
                    f.category.dir_name(),
                    e,
                    f.dump.activity.name
                );
                continue;
            }
        };

        let lo_kmh = r.cluster_low_mps * 3.6;
        let hi_kmh = r.cluster_high_mps * 3.6;
        let ratio = if r.cluster_low_mps > 0.01 {
            r.cluster_high_mps / r.cluster_low_mps
        } else {
            f64::INFINITY
        };

        let (mut work_dur, mut rec_dur, mut total_dur) = (0.0, 0.0, 0.0);
        for seg in &r.segments {
            total_dur += seg.duration_s;
            match seg.kind {
                intervals::types::SegmentKind::Work => work_dur += seg.duration_s,
                intervals::types::SegmentKind::Recovery => rec_dur += seg.duration_s,
                _ => {}
            }
        }
        let work_pct = if total_dur > 0.0 {
            work_dur / total_dur * 100.0
        } else {
            0.0
        };
        let rec_pct = if total_dur > 0.0 {
            rec_dur / total_dur * 100.0
        } else {
            0.0
        };

        // Overall speed CV across all segments, weighted by duration.
        let weighted_mean: f64 = if total_dur > 0.0 {
            r.segments.iter().map(|s| s.avg_speed_mps * s.duration_s).sum::<f64>() / total_dur
        } else {
            0.0
        };
        let overall_cv = if weighted_mean > 0.01 && total_dur > 0.0 {
            let var: f64 = r
                .segments
                .iter()
                .map(|s| {
                    let d = s.avg_speed_mps - weighted_mean;
                    d * d * s.duration_s
                })
                .sum::<f64>()
                / total_dur;
            var.sqrt() / weighted_mean
        } else {
            0.0
        };

        let mut rec_durs: Vec<f64> = r
            .reps
            .iter()
            .filter_map(|rep| rep.recovery_duration_s)
            .collect();
        rec_durs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_rec_dur = if rec_durs.is_empty() {
            0.0
        } else {
            rec_durs[rec_durs.len() / 2]
        };

        let name: String = f.dump.activity.name.chars().take(45).collect();
        println!(
            "{:<10} {:<5.2} {:<6.1} {:<6.1} {:<6.2} {:<6} {:<6.0} {:<6.0} {:<6.2} {:<6.0} {}",
            f.category.dir_name(),
            r.interval_score,
            lo_kmh,
            hi_kmh,
            ratio,
            r.reps.len(),
            work_pct,
            rec_pct,
            overall_cv,
            median_rec_dur,
            name,
        );
        rows.push(Row {
            category: f.category,
            score: r.interval_score,
            ratio,
            gap_mps: r.cluster_high_mps - r.cluster_low_mps,
            low_kmh: lo_kmh,
            reps: r.reps.len(),
            work_pct,
            rec_pct,
            overall_cv,
            median_rec_dur,
        });
    }

    // Per-category summary stats
    println!("\n=== Per-signal distribution by category (median) ===");
    println!(
        "{:<10} {:<4} {:<8} {:<8} {:<8} {:<8} {:<8}",
        "category", "n", "score", "ratio", "work%", "rec%", "ovr_cv"
    );
    println!("{}", "-".repeat(70));
    let median = |v: &mut Vec<f64>| -> f64 {
        if v.is_empty() {
            return f64::NAN;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };
    for cat in FixtureCategory::ALL {
        if !include_trails && !cat.in_scope_v1() {
            continue;
        }
        let cat_rows: Vec<&Row> = rows.iter().filter(|r| r.category == *cat).collect();
        if cat_rows.is_empty() {
            continue;
        }
        let mut scores: Vec<f64> = cat_rows.iter().map(|r| r.score).collect();
        let mut ratios: Vec<f64> = cat_rows.iter().map(|r| r.ratio).collect();
        let mut works: Vec<f64> = cat_rows.iter().map(|r| r.work_pct).collect();
        let mut recs: Vec<f64> = cat_rows.iter().map(|r| r.rec_pct).collect();
        let mut cvs: Vec<f64> = cat_rows.iter().map(|r| r.overall_cv).collect();
        println!(
            "{:<10} {:<4} {:<8.3} {:<8.2} {:<8.0} {:<8.0} {:<8.2}",
            cat.dir_name(),
            cat_rows.len(),
            median(&mut scores),
            median(&mut ratios),
            median(&mut works),
            median(&mut recs),
            median(&mut cvs),
        );
    }

    // Try a v2 score: weight recovery%, ratio, and overall CV. See if it
    // separates intervals from non-intervals where the v1 score didn't.
    println!("\n=== Candidate v2 scores (positive class = intervals) ===");
    println!(
        "{:<22} {:<6} {:<6} {:<6} {:<6} {:<8} {:<8}",
        "rule", "TP", "FP", "TN", "FN", "precision", "recall"
    );
    println!("{}", "-".repeat(74));

    let try_rule = |label: &str, predicate: &dyn Fn(&Row) -> bool| {
        let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
        for r in &rows {
            let predicted = predicate(r);
            let actual = r.category.expected_interval_positive();
            match (predicted, actual) {
                (true, true) => tp += 1,
                (true, false) => fp += 1,
                (false, false) => tn += 1,
                (false, true) => fn_ += 1,
            }
        }
        let precision = if tp + fp == 0 {
            f64::NAN
        } else {
            tp as f64 / (tp + fp) as f64
        };
        let recall = if tp + fn_ == 0 {
            f64::NAN
        } else {
            tp as f64 / (tp + fn_) as f64
        };
        println!(
            "{:<22} {:<6} {:<6} {:<6} {:<6} {:<8.2} {:<8.2}",
            label, tp, fp, tn, fn_, precision, recall
        );
    };

    try_rule("ratio>=1.5", &|r| r.ratio >= 1.5);
    try_rule("ratio>=1.7", &|r| r.ratio >= 1.7);
    try_rule("ovr_cv>=0.20", &|r| r.overall_cv >= 0.20);
    try_rule("ovr_cv>=0.25", &|r| r.overall_cv >= 0.25);
    try_rule("low_kmh<=10.5", &|r| r.low_kmh <= 10.5);
    try_rule("low_kmh<=11.0", &|r| r.low_kmh <= 11.0);
    try_rule("gap_mps>=1.0", &|r| r.gap_mps >= 1.0);
    try_rule("gap_mps>=1.2", &|r| r.gap_mps >= 1.2);
    try_rule("ratio>=1.5 & ovr_cv>=0.20", &|r| {
        r.ratio >= 1.5 && r.overall_cv >= 0.20
    });
    try_rule("ratio>=1.5 & low_kmh<=11.0", &|r| {
        r.ratio >= 1.5 && r.low_kmh <= 11.0
    });
    try_rule("ratio>=1.5 & low_kmh<=10.5", &|r| {
        r.ratio >= 1.5 && r.low_kmh <= 10.5
    });
    try_rule("ovr_cv>=0.20 & low_kmh<=11.0", &|r| {
        r.overall_cv >= 0.20 && r.low_kmh <= 11.0
    });
    try_rule("ovr_cv>=0.25 & low_kmh<=11.0", &|r| {
        r.overall_cv >= 0.25 && r.low_kmh <= 11.0
    });
    try_rule("(ratio>=1.5|ovr_cv>=0.30) & low_kmh<=11.0", &|r| {
        (r.ratio >= 1.5 || r.overall_cv >= 0.30) && r.low_kmh <= 11.0
    });
    try_rule("ratio>=1.4 & low_kmh<=11.0 & reps>=3", &|r| {
        r.ratio >= 1.4 && r.low_kmh <= 11.0 && r.reps >= 3
    });
    try_rule("ratio>=1.5 & low_kmh<=11.0 & reps>=3", &|r| {
        r.ratio >= 1.5 && r.low_kmh <= 11.0 && r.reps >= 3
    });
    try_rule("median_rec_dur>=60 & low_kmh<=11.0", &|r| {
        r.median_rec_dur >= 60.0 && r.low_kmh <= 11.0
    });
    try_rule("med_rec_dur>=60 & ratio>=1.4", &|r| {
        r.median_rec_dur >= 60.0 && r.ratio >= 1.4
    });

    // Candidate v2 score: weighted combination of gap, overall CV, rep count,
    // and recovery slowness. Each term is normalized to [0, 1] before weighting.
    let v2_score = |r: &Row| -> f64 {
        if r.reps < 3 {
            return 0.0;
        }
        let gap_term = (r.gap_mps / 1.5).clamp(0.0, 1.0);
        let cv_term = (r.overall_cv / 0.4).clamp(0.0, 1.0);
        let reps_term = ((r.reps as f64 - 2.0) / 5.0).clamp(0.0, 1.0);
        let recovery_term = ((13.0 - r.low_kmh) / 3.0).clamp(0.0, 1.0);
        0.35 * gap_term + 0.30 * cv_term + 0.15 * reps_term + 0.20 * recovery_term
    };

    println!("\n=== v2 score per fixture ===");
    println!("{:<10} {:<6} {:<8} {}", "category", "v2", "v1", "name");
    println!("{}", "-".repeat(80));
    let mut v2_rows: Vec<(FixtureCategory, f64, String)> = Vec::new();
    for f in &all {
        // Recompute via fixture (needs to share logic with rows above)
        if let Some(row) = rows.iter().find(|r| {
            r.category == f.category
                && (r.score - 0.0).abs() < 100.0 // dummy match — use index instead
        }) {
            let _ = row;
        }
    }
    // Simpler: just re-iterate rows alongside `all`. We pushed in order, with skips
    // only on parse error. Assume no errors in the corpus.
    for (row, fx) in rows.iter().zip(all.iter()) {
        let v2 = v2_score(row);
        let name: String = fx.dump.activity.name.chars().take(50).collect();
        println!(
            "{:<10} {:<6.2} {:<8.2} {}",
            row.category.dir_name(),
            v2,
            row.score,
            name
        );
        v2_rows.push((row.category, v2, name));
    }

    println!("\n=== v2 threshold sweep ===");
    println!(
        "{:<10} {:<6} {:<6} {:<6} {:<6} {:<8} {:<8}",
        "threshold", "TP", "FP", "TN", "FN", "precision", "recall"
    );
    println!("{}", "-".repeat(60));
    for &t in &[0.30_f64, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70] {
        let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
        for (cat, score, _) in &v2_rows {
            let predicted = *score >= t;
            let actual = cat.expected_interval_positive();
            match (predicted, actual) {
                (true, true) => tp += 1,
                (true, false) => fp += 1,
                (false, false) => tn += 1,
                (false, true) => fn_ += 1,
            }
        }
        let precision = if tp + fp == 0 {
            f64::NAN
        } else {
            tp as f64 / (tp + fp) as f64
        };
        let recall = if tp + fn_ == 0 {
            f64::NAN
        } else {
            tp as f64 / (tp + fn_) as f64
        };
        println!(
            "{:<10.2} {:<6} {:<6} {:<6} {:<6} {:<8.2} {:<8.2}",
            t, tp, fp, tn, fn_, precision, recall
        );
    }

    Ok(())
}
