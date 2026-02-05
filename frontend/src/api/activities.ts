import { apiFetch } from './client';
import { Activity, ActivityDetail } from '../types';

export async function syncActivities(after?: number, before?: number) {
  return apiFetch<{ synced: number }>('/activities/sync', {
    method: 'POST',
    body: JSON.stringify({ after: after ?? null, before: before ?? null }),
  });
}

export async function listActivities(limit = 50, offset = 0) {
  return apiFetch<Activity[]>(`/activities?limit=${limit}&offset=${offset}`);
}

export async function getActivity(id: string) {
  return apiFetch<ActivityDetail>(`/activities/${id}`);
}

export async function updateActivityTag(id: string, tag: string) {
  return apiFetch<{ status: string }>(`/activities/${id}/tag`, {
    method: 'PATCH',
    body: JSON.stringify({ tag }),
  });
}
