import { apiFetch } from './client';
import type { StravaStatus } from '../types';

export async function getStravaLink() {
  return apiFetch<{ url: string }>('/strava/link');
}

export async function getStravaStatus() {
  return apiFetch<StravaStatus>('/strava/status');
}

export async function disconnectStrava() {
  return apiFetch<{ status: string }>('/strava/disconnect', { method: 'POST' });
}
