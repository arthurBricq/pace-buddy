import { apiFetch } from './client';
import type { AiChat, AiChatMessage, ChatResponse, ChatListItem, ModelInfo } from '../types';

export async function createChat(title: string, model?: string, trainingId?: string) {
  return apiFetch<AiChat>('/chats', {
    method: 'POST',
    body: JSON.stringify({
      title,
      model: model || null,
      training_id: trainingId || null,
    }),
  });
}

export async function listChats() {
  return apiFetch<ChatListItem[]>('/chats');
}

export async function getChat(id: string) {
  return apiFetch<ChatResponse>(`/chats/${id}`);
}

export async function deleteChat(id: string) {
  return apiFetch<{ status: string }>(`/chats/${id}`, { method: 'DELETE' });
}

export async function sendMessage(chatId: string, content: string) {
  return apiFetch<AiChatMessage>(`/chats/${chatId}/messages`, {
    method: 'POST',
    body: JSON.stringify({ content }),
  });
}

export async function listModels() {
  return apiFetch<ModelInfo[]>('/chats/models');
}

export async function createChatFromInsight(
  insightId: string,
  model?: string,
  conversationLength?: number
) {
  return apiFetch<AiChat>(`/chats/from-insight/${insightId}`, {
    method: 'POST',
    body: JSON.stringify({
      model: model || null,
      conversation_length: conversationLength || null,
    }),
  });
}
