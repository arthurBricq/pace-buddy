import { apiFetch } from './client';
import type { Activity, ActivityDetail, IntervalAlgorithm, IntervalResponse } from '../types';

export async function syncActivities(after?: number, before?: number) {
  return apiFetch<{ synced: number; already_running?: boolean }>('/activities/sync', {
    method: 'POST',
    body: JSON.stringify({ after: after ?? null, before: before ?? null }),
  });
}

export async function getActivitiesSyncStatus() {
  return apiFetch<{ status: 'idle' | 'running' | 'finished' | 'failed'; error?: string | null }>(
    '/activities/sync/status'
  );
}

export async function listActivities(limit = 50, offset = 0) {
  return apiFetch<Activity[]>(`/activities?limit=${limit}&offset=${offset}`);
}

export async function getActivity(id: string) {
  return apiFetch<ActivityDetail>(`/activities/${id}`);
}

export async function getIntervals(id: string, algorithm?: IntervalAlgorithm) {
  const query = algorithm ? `?algorithm=${algorithm}` : '';
  return apiFetch<IntervalResponse>(`/activities/${id}/intervals${query}`);
}

export async function updateActivityTag(id: string, tag: string) {
  return apiFetch<{ status: string }>(`/activities/${id}/tag`, {
    method: 'PATCH',
    body: JSON.stringify({tag}),
  });
}
