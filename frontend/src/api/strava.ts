import { apiFetch } from './client';
import { StravaStatus } from '../types';

export async function getStravaLink() {
  return apiFetch<{ url: string }>('/strava/link');
}

export async function getStravaStatus() {
  return apiFetch<StravaStatus>('/strava/status');
}
