import { apiFetch } from './client';
import type { Training, Activity } from '../types';

export async function createTraining(name: string, description?: string) {
  return apiFetch<Training>('/trainings', {
    method: 'POST',
    body: JSON.stringify({ name, description: description || null }),
  });
}

export async function listTrainings() {
  return apiFetch<Training[]>('/trainings');
}

export async function getTraining(id: string) {
  return apiFetch<Training>(`/trainings/${id}`);
}

export async function updateTraining(
  id: string,
  name?: string,
  description?: string,
) {
  return apiFetch<Training>(`/trainings/${id}`, {
    method: 'PATCH',
    body: JSON.stringify({ name, description: description || null }),
  });
}

export async function deleteTraining(id: string) {
  return apiFetch<{ status: string }>(`/trainings/${id}`, {
    method: 'DELETE',
  });
}

export async function addActivityToTraining(trainingId: string, activityId: string) {
  return apiFetch<{ status: string }>(
    `/trainings/${trainingId}/activities/${activityId}`,
    {
      method: 'POST',
    },
  );
}

export async function removeActivityFromTraining(
  trainingId: string,
  activityId: string,
) {
  return apiFetch<{ status: string }>(
    `/trainings/${trainingId}/activities/${activityId}`,
    {
      method: 'DELETE',
    },
  );
}

export async function getTrainingActivities(trainingId: string) {
  return apiFetch<Activity[]>(`/trainings/${trainingId}/activities`);
}

export async function getActivityTrainings(activityId: string) {
  return apiFetch<Training[]>(`/activities/${activityId}/trainings`);
}
