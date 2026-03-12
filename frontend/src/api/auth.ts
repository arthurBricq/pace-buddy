import { apiFetch } from './client';
import type { User, ProfileResponse, QuotaStatus, QuotaRequestRecord, MASEstimate } from '../types';

export async function startStravaAuth() {
  return apiFetch<{ url: string }>('/auth/strava/start', { method: 'POST' });
}

export async function logout() {
  return apiFetch<{ status: string }>('/auth/logout', { method: 'POST' });
}

export async function getMe() {
  return apiFetch<User>('/auth/me');
}

export async function getMAS() {
  return apiFetch<{ mas_kmh: number | null }>('/auth/mas');
}

export async function updateMAS(mas_kmh: number | null) {
  return apiFetch<{ status: string }>('/auth/mas', {
    method: 'PATCH',
    body: JSON.stringify({ mas_kmh }),
  });
}

export async function recomputeMAS() {
  return apiFetch<{ status: string; mas_kmh: number }>('/auth/mas/recompute', {
    method: 'POST',
  });
}

export async function getMASEstimates() {
  return apiFetch<MASEstimate[]>('/auth/mas/estimates');
}

export async function getProfile() {
  return apiFetch<ProfileResponse>('/auth/profile');
}

export interface ExpensiveRequest {
  id: string;
  type: 'insight' | 'chat';
  title: string;
  model: string | null;
  cost: number;
  created_at: string;
}

export interface AiCostSummary {
  total_cost: number;
  expensive_requests: ExpensiveRequest[];
}

export async function getAiCostSummary() {
  return apiFetch<AiCostSummary>('/auth/ai-cost-summary');
}

export async function getQuotaStatus() {
  return apiFetch<QuotaStatus>('/auth/quota');
}

export async function requestQuota() {
  return apiFetch<QuotaRequestRecord>('/auth/quota/request', { method: 'POST' });
}
