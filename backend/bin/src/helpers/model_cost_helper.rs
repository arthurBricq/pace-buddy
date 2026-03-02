use chrono::Utc;
use domain::{ModelCostCategory, ModelCostTier};
use llm::LlmClient;
use storage::Storage;

use crate::state::AppState;

fn parse_price(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

pub async fn recompute_model_cost_tiers(state: &AppState) -> Result<usize, domain::DomainError> {
    let llm_client = state
        .llm_client
        .as_ref()
        .ok_or_else(|| domain::DomainError::Internal("LLM not configured".into()))?;

    let models = llm_client.list_models().await.map_err(|e| {
        domain::DomainError::Internal(format!("Failed to list models from provider: {e}"))
    })?;

    if models.is_empty() {
        state.storage.upsert_model_cost_tiers(&[]).await?;
        return Ok(0);
    }

    let mut categories = vec![ModelCostCategory::Standard; models.len()];

    let mut priced_indexes = models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| {
            let pricing = model.pricing.as_ref()?;
            let prompt = parse_price(&pricing.prompt)?;
            let completion = parse_price(&pricing.completion)?;
            // Blended token cost proxy used only for relative categorization.
            Some((index, prompt + completion))
        })
        .collect::<Vec<_>>();

    priced_indexes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let priced_len = priced_indexes.len();
    if priced_len == 1 {
        categories[priced_indexes[0].0] = ModelCostCategory::Standard;
    } else if priced_len == 2 {
        categories[priced_indexes[0].0] = ModelCostCategory::Economical;
        categories[priced_indexes[1].0] = ModelCostCategory::Expensive;
    } else if priced_len >= 3 {
        let lower_cutoff = ((priced_len as f64) / 3.0).ceil() as usize;
        let upper_cutoff = ((2.0 * priced_len as f64) / 3.0).ceil() as usize;

        for (rank, (index, _)) in priced_indexes.iter().enumerate() {
            categories[*index] = if rank < lower_cutoff {
                ModelCostCategory::Economical
            } else if rank < upper_cutoff {
                ModelCostCategory::Standard
            } else {
                ModelCostCategory::Expensive
            };
        }
    }

    let computed_at = Utc::now();
    let tiers = models
        .into_iter()
        .enumerate()
        .map(|(index, model)| ModelCostTier {
            model_id: model.id,
            model_name: if model.name.trim().is_empty() {
                "Unknown model".to_string()
            } else {
                model.name
            },
            category: categories[index].clone(),
            computed_at,
        })
        .collect::<Vec<_>>();

    state.storage.upsert_model_cost_tiers(&tiers).await?;
    Ok(tiers.len())
}
