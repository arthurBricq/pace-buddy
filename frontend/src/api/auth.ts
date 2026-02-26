import { apiFetch } from './client';
import type { User, ProfileResponse, QuotaStatus, QuotaRequestRecord } from '../types';

export async function registerStart(username: string, displayName: string) {
  return apiFetch<{ user_id: string; options: PublicKeyCredentialCreationOptions }>(
    '/auth/register/start',
    {
      method: 'POST',
      body: JSON.stringify({ username, display_name: displayName }),
    },
  );
}

export async function registerFinish(userId: string, credential: unknown) {
  return apiFetch<{ status: string }>('/auth/register/finish', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, credential }),
  });
}

export async function loginStart(username: string) {
  return apiFetch<{ user_id: string; options: PublicKeyCredentialRequestOptions }>(
    '/auth/login/start',
    {
      method: 'POST',
      body: JSON.stringify({ username }),
    },
  );
}

export async function loginFinish(userId: string, credential: unknown) {
  return apiFetch<{ status: string }>('/auth/login/finish', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, credential }),
  });
}

export async function logout() {
  return apiFetch<{ status: string }>('/auth/logout', { method: 'POST' });
}

export async function getMe() {
  return apiFetch<User>('/auth/me');
}

export async function getMAS() {
  return apiFetch<{ mas_mps: number | null }>('/auth/mas');
}

export async function updateMAS(mas_mps: number | null) {
  return apiFetch<{ status: string }>('/auth/mas', {
    method: 'PATCH',
    body: JSON.stringify({ mas_mps }),
  });
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
