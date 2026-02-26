import { apiFetch } from './client';
import type { QuotaRequestRecord } from '../types';

export interface AdminStats {
  user_count: number;
}

export function getAdminStats(): Promise<AdminStats> {
  return apiFetch<AdminStats>('/admin/stats');
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
