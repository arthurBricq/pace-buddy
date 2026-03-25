import { apiFetch } from './client';
import type {
  RunningCoachMessage,
  RunningCoachResponse,
  RunningCoachSettings,
} from '../types';

export interface RunningCoachSettingsInput {
  model: string;
  personality: string;
  consider_trail_runs_as_runs: boolean;
  volume_weeks: number;
  last_workouts_count: number;
  last_long_runs_count: number;
  last_races_count: number;
  new_activities_count: number;
  normalizer_every_n_messages: number;
}

export async function getCoach() {
  return apiFetch<RunningCoachResponse>('/coach');
}

export async function sendCoachMessage(content: string) {
  return apiFetch<RunningCoachMessage>('/coach/messages', {
    method: 'POST',
    body: JSON.stringify({ content }),
  });
}

export async function updateCoachSettings(input: RunningCoachSettingsInput) {
  return apiFetch<RunningCoachSettings>('/coach/settings', {
    method: 'PUT',
    body: JSON.stringify(input),
  });
}

export async function resetCoach() {
  return apiFetch<{ status: string }>('/coach/reset', {
    method: 'POST',
  });
}
