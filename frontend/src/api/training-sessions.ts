import { apiFetch } from './client';
import type { SessionStatus, TrainingSession } from '../types';

export function listTrainingSessions(
  status?: SessionStatus,
): Promise<TrainingSession[]> {
  const qs = status ? `?status=${encodeURIComponent(status)}` : '';
  return apiFetch<TrainingSession[]>(`/training-sessions${qs}`);
}

export function getTrainingSession(id: string): Promise<TrainingSession> {
  return apiFetch<TrainingSession>(`/training-sessions/${id}`);
}

export function updateTrainingSessionStatus(
  id: string,
  status: SessionStatus,
): Promise<TrainingSession> {
  return apiFetch<TrainingSession>(`/training-sessions/${id}/status`, {
    method: 'PATCH',
    body: JSON.stringify({ status }),
  });
}
