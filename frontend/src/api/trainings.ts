import { apiFetch } from './client';
import type { Training, Activity, TrainingInsightResponse, TrainingInsightRecord } from '../types';

export async function createTraining(
  name: string,
  description?: string,
  start_date?: string,
  end_date?: string,
  race_goal?: string,
  race_objectif?: string,
) {
  return apiFetch<Training>('/trainings', {
    method: 'POST',
    body: JSON.stringify({
      name,
      description: description || null,
      start_date: start_date || null,
      end_date: end_date || null,
      race_goal: race_goal || null,
      race_objectif: race_objectif || null,
    }),
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
  fields: {
    name?: string;
    description?: string;
    start_date?: string;
    end_date?: string;
    race_goal?: string;
  },
) {
  return apiFetch<Training>(`/trainings/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(fields),
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

export async function getTrainingInsight(
  trainingId: string,
  promptType: 'overview' | 'suggestions',
  model?: string,
) {
  return apiFetch<TrainingInsightResponse>(`/trainings/${trainingId}/insight`, {
    method: 'POST',
    body: JSON.stringify({
      prompt_type: promptType,
      model: model || null,
    }),
  });
}

export async function listTrainingInsights(trainingId: string) {
  return apiFetch<TrainingInsightRecord[]>(`/trainings/${trainingId}/insights`);
}
