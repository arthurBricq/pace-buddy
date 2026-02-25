import { apiFetch } from './client';

export interface AdminStats {
  user_count: number;
}

export function getAdminStats(): Promise<AdminStats> {
  return apiFetch<AdminStats>('/admin/stats');
}
