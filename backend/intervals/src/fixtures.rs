//! Fixture loader for the labeled corpus under `backend/intervals/fixtures/`.
//!
//! Layout:
//! ```text
//! fixtures/
//!   intervals/<uuid>.json   <- ground-truth: actual interval workouts
//!   races/<uuid>.json       <- ground-truth: race efforts
//!   runs/<uuid>.json        <- ground-truth: easy / steady runs
//!   trails/<uuid>.json      <- ground-truth: trail running (out of v1 scope)
//! ```
//!
//! The directory name is the label; no manifest needed. Files are gitignored
//! and only present on developer machines.

use std::fs;
use std::path::{Path, PathBuf};

use domain::{Activity, ActivityLap, ActivityStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum FixtureCategory {
    Intervals,
    Races,
    Runs,
    Trails,
}

impl FixtureCategory {
    pub const ALL: &'static [Self] = &[
        Self::Intervals,
        Self::Races,
        Self::Runs,
        Self::Trails,
    ];

    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Intervals => "intervals",
            Self::Races => "races",
            Self::Runs => "runs",
            Self::Trails => "trails",
        }
    }

    /// True iff this fixture is in scope for v1 (road running).
    pub fn in_scope_v1(self) -> bool {
        !matches!(self, Self::Trails)
    }

    /// True iff a road-running parser should classify this as a positive
    /// interval workout. Only the `intervals` bucket is positive; everything
    /// else is a negative.
    pub fn expected_interval_positive(self) -> bool {
        matches!(self, Self::Intervals)
    }
}

#[derive(Debug, Deserialize)]
pub struct IntervalResultDump {
    pub algorithm: String,
    pub result: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ActivityDump {
    pub activity: Activity,
    #[serde(default)]
    pub streams: Vec<ActivityStream>,
    #[serde(default)]
    pub laps: Vec<ActivityLap>,
    #[serde(default)]
    pub interval_result: Option<IntervalResultDump>,
}

pub struct LoadedFixture {
    pub category: FixtureCategory,
    pub path: PathBuf,
    pub dump: ActivityDump,
}

/// Default fixtures root: `backend/intervals/fixtures/`.
pub fn default_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Load every `<root>/<category>/*.json` fixture. Missing category dirs are
/// silently skipped. Malformed JSON returns `Err`.
pub fn load_fixtures(root: &Path) -> std::io::Result<Vec<LoadedFixture>> {
    let mut out = Vec::new();
    for category in FixtureCategory::ALL {
        let dir = root.join(category.dir_name());
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            let dump: ActivityDump = serde_json::from_str(&text).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("malformed fixture {}: {}", path.display(), e),
                )
            })?;
            out.push(LoadedFixture {
                category: *category,
                path,
                dump,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_skips_missing_root() {
        let dir = PathBuf::from("/nonexistent-fixture-dir");
        let fixtures = load_fixtures(&dir).expect("load");
        assert!(fixtures.is_empty());
    }

    #[test]
    fn corpus_loads_or_is_empty() {
        let dir = default_fixtures_dir();
        let fixtures = load_fixtures(&dir).expect("load corpus");
        // If the developer has a corpus, all files must parse cleanly.
        // If they don't, the result is just empty — tests stay green.
        for f in &fixtures {
            assert!(f.path.exists(), "loaded path should exist");
        }
    }
}
