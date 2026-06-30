use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProfile {
    pub user_id: Uuid,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub email: Option<String>,
    pub gender: Option<String>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleteProfile {
    pub user_id: Uuid,
    pub goal_description: Option<String>,
    pub goal_date: Option<String>,
    pub goal_distance_km: Option<f64>,
    pub goal_target_time_seconds: Option<i32>,
    pub goal_sport_type: Option<String>,
    pub goal_elevation_gain_m: Option<f64>,
    pub additional_info: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Display for IdentityProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IdentityProfile(")?;
        if let Some(name) = &self.name {
            write!(f, "name={name}, ")?;
        }
        if let Some(age) = self.age {
            write!(f, "age={age}, ")?;
        }
        if let Some(gender) = &self.gender {
            write!(f, "gender={gender}, ")?;
        }
        if let Some(height_cm) = self.height_cm {
            write!(f, "height_cm={height_cm:.1}, ")?;
        }
        if let Some(weight_kg) = self.weight_kg {
            write!(f, "weight_kg={weight_kg:.1}, ")?;
        }
        write!(f, "updated_at={})", self.updated_at.format("%Y-%m-%d"))
    }
}

impl std::fmt::Display for AthleteProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AthleteProfile(")?;
        if let Some(goal_description) = &self.goal_description {
            write!(f, "goal={goal_description}, ")?;
        }
        if let Some(goal_date) = &self.goal_date {
            write!(f, "goal_date={goal_date}, ")?;
        }
        if let Some(goal_distance_km) = self.goal_distance_km {
            write!(f, "goal_distance_km={goal_distance_km:.1}, ")?;
        }
        if let Some(goal_target_time_seconds) = self.goal_target_time_seconds {
            write!(f, "goal_target_time_s={goal_target_time_seconds}, ")?;
        }
        if let Some(goal_sport_type) = &self.goal_sport_type {
            write!(f, "goal_sport_type={goal_sport_type}, ")?;
        }
        if let Some(goal_elevation_gain_m) = self.goal_elevation_gain_m {
            write!(f, "goal_elevation_gain_m={goal_elevation_gain_m:.0}, ")?;
        }
        if let Some(additional_info) = &self.additional_info {
            write!(f, "additional_info={additional_info}, ")?;
        }
        write!(f, "updated_at={})", self.updated_at.format("%Y-%m-%d"))
    }
}
