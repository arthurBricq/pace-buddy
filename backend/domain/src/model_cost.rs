use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCostCategory {
    Economical,
    Standard,
    Expensive,
}

impl ModelCostCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Economical => "economical",
            Self::Standard => "standard",
            Self::Expensive => "expensive",
        }
    }
}

impl FromStr for ModelCostCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "economical" => Ok(Self::Economical),
            "standard" => Ok(Self::Standard),
            "expensive" => Ok(Self::Expensive),
            _ => Err(format!("Invalid model cost category: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostTier {
    pub model_id: String,
    pub model_name: String,
    pub category: ModelCostCategory,
    pub computed_at: DateTime<Utc>,
}
