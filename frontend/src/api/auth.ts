import { apiFetch } from './client';
import { User } from '../types';

export async function registerStart(username: string, displayName: string) {
  return apiFetch<{ user_id: string; options: PublicKeyCredentialCreationOptions }>(
    '/auth/register/start',
    {
      method: 'POST',
      body: JSON.stringify({ username, display_name: displayName }),
    },
  );
}

export async function registerFinish(userId: string, credential: unknown) {
  return apiFetch<{ status: string }>('/auth/register/finish', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, credential }),
  });
}

export async function loginStart(username: string) {
  return apiFetch<{ user_id: string; options: PublicKeyCredentialRequestOptions }>(
    '/auth/login/start',
    {
      method: 'POST',
      body: JSON.stringify({ username }),
    },
  );
}

export async function loginFinish(userId: string, credential: unknown) {
  return apiFetch<{ status: string }>('/auth/login/finish', {
    method: 'POST',
    body: JSON.stringify({ user_id: userId, credential }),
  });
}

export async function logout() {
  return apiFetch<{ status: string }>('/auth/logout', { method: 'POST' });
}

export async function getMe() {
  return apiFetch<User>('/auth/me');
}
