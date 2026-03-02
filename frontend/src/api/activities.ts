import {apiFetch} from './client';
import type {Activity, ActivityDetail, IntervalResult} from '../types';

type SyncActivitiesResult = { synced: number };
type SyncListener = (syncing: boolean) => void;

let syncActivitiesPromise: Promise<SyncActivitiesResult> | null = null;
const syncListeners = new Set<SyncListener>();

function emitSyncState() {
  const syncing = syncActivitiesPromise !== null;
  syncListeners.forEach((listener) => listener(syncing));
}

export function isActivitiesSyncInProgress() {
  return syncActivitiesPromise !== null;
}

export function subscribeActivitiesSync(listener: SyncListener) {
  syncListeners.add(listener);
  listener(isActivitiesSyncInProgress());
  return () => {
    syncListeners.delete(listener);
  };
}

export async function syncActivities(after?: number, before?: number) {
  if (syncActivitiesPromise) {
    return syncActivitiesPromise;
  }

  syncActivitiesPromise = apiFetch<SyncActivitiesResult>('/activities/sync', {
    method: 'POST',
    body: JSON.stringify({after: after ?? null, before: before ?? null}),
  }).finally(() => {
    syncActivitiesPromise = null;
    emitSyncState();
  });

  emitSyncState();
  return syncActivitiesPromise;
}

export async function listActivities(limit = 50, offset = 0) {
  return apiFetch<Activity[]>(`/activities?limit=${limit}&offset=${offset}`);
}

export async function getActivity(id: string) {
  return apiFetch<ActivityDetail>(`/activities/${id}`);
}

export async function getIntervals(id: string) {
  return apiFetch<IntervalResult>(`/activities/${id}/intervals`);
}

export async function updateActivityTag(id: string, tag: string) {
  return apiFetch<{ status: string }>(`/activities/${id}/tag`, {
    method: 'PATCH',
    body: JSON.stringify({tag}),
  });
}
