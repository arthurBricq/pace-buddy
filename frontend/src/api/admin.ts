import { apiFetch } from './client';
import type { QuotaRequestRecord } from '../types';

export interface AdminStats {
  user_count: number;
}

export interface AdminUserQuotaSpending {
  user_id: string;
  username: string;
  display_name: string;
  email: string | null;
  created_at: string;
  quota_balance_usd: number;
  total_granted_usd: number;
  total_spent_usd: number;
}

export interface AdminInviteCode {
  id: string;
  created_by_user_id: string | null;
  created_for: string | null;
  created_at: string;
  expires_at: string | null;
  used_at: string | null;
  used_by_strava_athlete_id: number | null;
  revoked_at: string | null;
  is_redeemable: boolean;
}

export interface CreateInviteCodePayload {
  created_for?: string;
  expires_in_days?: number;
  code?: string;
}

export interface CreateInviteCodeResponse {
  code: string;
  invite: AdminInviteCode;
}

export function getAdminStats(): Promise<AdminStats> {
  return apiFetch<AdminStats>('/admin/stats');
}

export function getAdminUsersByQuotaSpent(): Promise<AdminUserQuotaSpending[]> {
  return apiFetch<AdminUserQuotaSpending[]>('/admin/users');
}

export function getQuotaRequests(): Promise<QuotaRequestRecord[]> {
  return apiFetch<QuotaRequestRecord[]>('/admin/quota-requests');
}

export function approveQuotaRequest(id: string, amount_usd: number): Promise<{ status: string }> {
  return apiFetch('/admin/quota-requests/' + id + '/approve', {
    method: 'POST',
    body: JSON.stringify({ amount_usd }),
  });
}

export function rejectQuotaRequest(id: string): Promise<{ status: string }> {
  return apiFetch('/admin/quota-requests/' + id + '/reject', { method: 'POST' });
}

export function deleteAllData(): Promise<{ status: string }> {
  return apiFetch('/admin/delete-all-data', { method: 'POST' });
}

export function listInviteCodes(): Promise<AdminInviteCode[]> {
  return apiFetch<AdminInviteCode[]>('/admin/invite-codes');
}

export function createInviteCode(
  payload: CreateInviteCodePayload,
): Promise<CreateInviteCodeResponse> {
  return apiFetch<CreateInviteCodeResponse>('/admin/invite-codes', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function revokeInviteCode(id: string): Promise<{ status: string }> {
  return apiFetch('/admin/invite-codes/' + id + '/revoke', { method: 'POST' });
}
