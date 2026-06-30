//! Typed schema for `TrainingSession.prescription_json`.
//!
//! Domain-level only: the storage layer keeps prescriptions as raw `TEXT`,
//! per the codebase convention for JSON blobs. Callers that need to reason
//! about a prescription (the coach in Phase 3, the comparison helpers in
//! Phase 4, the frontend display) parse on demand via `Prescription::parse`.
//!
//! Deliberately independent of the `intervals` crate. That crate models
//! *executed* sessions (`Rep` carries measured `avg_pace_s_per_km`,
//! `pace_std`, `pct_mas`); a planned `Set` carries *targets* (ranges,
//! distance-or-duration choice). Phase 4's comparison helpers map planned
//! `Target` → executed `Rep` for diffing.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Prescription {
    pub warmup: Option<OpenBlock>,
    #[schemars(length(min = 1))]
    pub sets: Vec<Set>,
    pub cooldown: Option<OpenBlock>,
    pub notes: Option<String>,
}

impl Prescription {
    /// Parse a prescription from a JSON string. Returns an error when the
    /// blob is malformed or missing required fields. Callers may render
    /// the raw string instead on error (fail-soft).
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// Warmup or cooldown block — no formal target; the intent is "easy".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenBlock {
    #[schemars(range(min = 1))]
    pub duration_s: Option<u32>,
    #[schemars(range(min = 1))]
    pub distance_m: Option<u32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Set {
    #[schemars(range(min = 1))]
    pub repeat: u32,
    pub work: WorkBlock,
    pub recovery: Option<RecoveryBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkBlock {
    #[schemars(range(min = 1))]
    pub duration_s: Option<u32>,
    #[schemars(range(min = 1))]
    pub distance_m: Option<u32>,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryBlock {
    #[schemars(range(min = 1))]
    pub duration_s: Option<u32>,
    #[schemars(range(min = 1))]
    pub distance_m: Option<u32>,
    pub target: Option<Target>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    Pace {
        min_s_per_km: u32,
        max_s_per_km: u32,
    },
    Speed {
        min_mps: f64,
        max_mps: f64,
    },
    HeartRate {
        min_bpm: u32,
        max_bpm: u32,
    },
    PercentMas {
        min: f64,
        max: f64,
    },
    Rpe {
        min: u8,
        max: u8,
    },
    Effort {
        label: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(json: &str) -> Prescription {
        let parsed = Prescription::parse(json).expect("parse");
        let reserialized = serde_json::to_string(&parsed).expect("serialize");
        let reparsed = Prescription::parse(&reserialized).expect("re-parse");
        assert_eq!(parsed, reparsed, "round-trip equality");
        parsed
    }

    #[test]
    fn intervals_round_trip() {
        let json = include_str!("prescription_fixtures/intervals.json");
        let p = round_trip(json);
        assert_eq!(p.sets.len(), 1);
        assert_eq!(p.sets[0].repeat, 6);
        assert_eq!(p.sets[0].work.distance_m, Some(800));
        match &p.sets[0].work.target {
            Target::Pace {
                min_s_per_km,
                max_s_per_km,
            } => {
                assert_eq!(*min_s_per_km, 200);
                assert_eq!(*max_s_per_km, 210);
            }
            other => panic!("unexpected target: {:?}", other),
        }
    }

    #[test]
    fn tempo_round_trip() {
        let json = include_str!("prescription_fixtures/tempo.json");
        let p = round_trip(json);
        assert_eq!(p.sets.len(), 1);
        assert_eq!(p.sets[0].repeat, 1);
        assert_eq!(p.sets[0].work.distance_m, Some(5000));
        assert!(p.sets[0].recovery.is_none());
    }

    #[test]
    fn hill_round_trip() {
        let json = include_str!("prescription_fixtures/hill.json");
        let p = round_trip(json);
        assert_eq!(p.sets.len(), 1);
        assert_eq!(p.sets[0].repeat, 8);
        match &p.sets[0].work.target {
            Target::Effort { label } => assert!(label.contains("hard")),
            other => panic!("unexpected target: {:?}", other),
        }
    }

    #[test]
    fn fartlek_round_trip() {
        let json = include_str!("prescription_fixtures/fartlek.json");
        let p = round_trip(json);
        assert_eq!(p.sets.len(), 2);
    }

    #[test]
    fn malformed_yields_error() {
        assert!(Prescription::parse("nonsense").is_err());
        assert!(Prescription::parse("{}").is_err());
    }

    #[test]
    fn prescription_requires_sets() {
        let err = Prescription::parse(r#"{"warmup":{"duration_s":600}}"#)
            .expect_err("missing sets should fail");
        assert!(
            err.to_string().contains("sets"),
            "expected missing sets error, got: {err}"
        );
    }

    #[test]
    fn target_variants_serialize_with_type_tag() {
        let pace = Target::Pace {
            min_s_per_km: 200,
            max_s_per_km: 210,
        };
        let json = serde_json::to_string(&pace).expect("serialize");
        assert!(json.contains(r#""type":"pace""#));

        let effort = Target::Effort {
            label: "easy".into(),
        };
        let json = serde_json::to_string(&effort).expect("serialize");
        assert!(json.contains(r#""type":"effort""#));
        assert!(json.contains(r#""label":"easy""#));
    }
}
