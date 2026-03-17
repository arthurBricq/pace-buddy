import { apiFetch } from './client';
import type {
  AthleteProfile,
  IdentityProfile,
  MASEstimate,
  ProfileResponse,
  QuotaRequestRecord,
  QuotaStatus,
  User,
} from '../types';

export async function startStravaAuth(invite_code?: string) {
  return apiFetch<{ url: string }>('/auth/strava/start', {
    method: 'POST',
    body: JSON.stringify({ invite_code: invite_code || undefined }),
  });
}

export async function logout() {
  return apiFetch<{ status: string }>('/auth/logout', { method: 'POST' });
}

export async function getMe() {
  return apiFetch<User>('/auth/me');
}

export async function getOnboardingStatus() {
  return apiFetch<{ needs_onboarding: boolean }>('/auth/onboarding/status');
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

export interface UpsertIdentityProfilePayload {
  name?: string | null;
  age?: number | null;
  email?: string | null;
  gender?: string | null;
  height_cm?: number | null;
  weight_kg?: number | null;
}

export interface UpsertAthleteProfilePayload {
  goal_description?: string | null;
  goal_date?: string | null;
  goal_distance_km?: number | null;
  goal_target_time_seconds?: number | null;
  goal_sport_type?: string | null;
  goal_elevation_gain_m?: number | null;
  additional_info?: string | null;
}

export async function getIdentityProfile() {
  return apiFetch<IdentityProfile | null>('/auth/profile/identity');
}

export async function upsertIdentityProfile(payload: UpsertIdentityProfilePayload) {
  return apiFetch<IdentityProfile>('/auth/profile/identity', {
    method: 'PUT',
    body: JSON.stringify(payload),
  });
}

export async function getAthleteProfile() {
  return apiFetch<AthleteProfile | null>('/auth/profile/athlete');
}

export async function upsertAthleteProfile(payload: UpsertAthleteProfilePayload) {
  return apiFetch<AthleteProfile>('/auth/profile/athlete', {
    method: 'PUT',
    body: JSON.stringify(payload),
  });
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
