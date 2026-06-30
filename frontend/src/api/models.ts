import { apiFetch } from './client';
import type { ModelCostTier, ModelInfo } from '../types';

export async function listModels() {
  return apiFetch<ModelInfo[]>('/llm/models');
}

export async function getModelCostTiers() {
  return apiFetch<ModelCostTier[]>('/llm/models/cost-tiers');
}
