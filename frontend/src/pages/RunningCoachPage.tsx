import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import ReactMarkdown from 'react-markdown';
import Navbar from '../components/Navbar';
import CoachSettingsModal from '../components/CoachSettingsModal';
import SessionCard from '../components/SessionCard';
import { errorMessage } from '../api/client';
import { getCoach, resetCoach, sendCoachMessage, updateCoachSettings } from '../api/coach';
import {
  listTrainingSessions,
  updateTrainingSessionStatus,
} from '../api/training-sessions';
import type {
  RunningCoachMessage,
  RunningCoachResponse,
  RunningCoachSettings,
  SessionStatus,
  TrainingSession,
} from '../types';

export default function RunningCoachPage() {
  const [coach, setCoach] = useState<RunningCoachResponse | null>(null);
  const [messages, setMessages] = useState<RunningCoachMessage[]>([]);
  const [sessions, setSessions] = useState<TrainingSession[]>([]);
  const [pendingSessionId, setPendingSessionId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState('');
  const [input, setInput] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const loadCoach = async () => {
    setLoading(true);
    try {
      const [data, sess] = await Promise.all([
        getCoach(),
        listTrainingSessions(),
      ]);
      setCoach(data);
      setMessages(data.messages);
      setSessions(sess);
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to load coach'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadCoach();
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || sending || !coach) return;
    const content = input.trim();
    setInput('');
    setSending(true);
    setError('');

    const optimisticUserMessage: RunningCoachMessage = {
      id: 'pending-user',
      user_id: coach.settings.user_id,
      role: 'user',
      content,
      prompt_tokens: 0,
      completion_tokens: 0,
      total_tokens: 0,
      cost: 0,
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, optimisticUserMessage]);

    try {
      const assistant = await sendCoachMessage(content);
      setMessages((prev) => [...prev.filter((m) => m.id !== 'pending-user'), assistant]);
      const [refreshed, sess] = await Promise.all([
        getCoach(),
        listTrainingSessions(),
      ]);
      setCoach(refreshed);
      setMessages(refreshed.messages);
      setSessions(sess);
    } catch (err: unknown) {
      setMessages((prev) => prev.filter((m) => m.id !== 'pending-user'));
      setError(errorMessage(err, 'Failed to send message'));
    } finally {
      setSending(false);
    }
  };

  const handleSessionStatus = async (id: string, status: SessionStatus) => {
    setPendingSessionId(id);
    try {
      const updated = await updateTrainingSessionStatus(id, status);
      setSessions((prev) => prev.map((s) => (s.id === id ? updated : s)));
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to update session status'));
    } finally {
      setPendingSessionId(null);
    }
  };

  const sessionsByMessageId = useMemo(() => {
    const map = new Map<string, TrainingSession[]>();
    for (const s of sessions) {
      if (!s.coach_message_id) continue;
      const list = map.get(s.coach_message_id) ?? [];
      list.push(s);
      map.set(s.coach_message_id, list);
    }
    return map;
  }, [sessions]);

  const handleSaveSettings = async (next: RunningCoachSettings) => {
    const updated = await updateCoachSettings({
      model: next.model,
      personality: next.personality,
      consider_trail_runs_as_runs: next.consider_trail_runs_as_runs,
      volume_weeks: next.volume_weeks,
      last_workouts_count: next.last_workouts_count,
      last_long_runs_count: next.last_long_runs_count,
      last_races_count: next.last_races_count,
      new_activities_count: next.new_activities_count,
      normalizer_every_n_messages: next.normalizer_every_n_messages,
    });
    setCoach((prev) => (prev ? { ...prev, settings: updated } : prev));
  };

  const handleResetCoach = async () => {
    await resetCoach();
    await loadCoach();
  };

  const formatCost = (cost: number) => {
    if (cost === 0) return '$0';
    return `$${cost.toFixed(4)}`;
  };

  const visibleMessages = messages.filter((m) => m.role !== 'system');

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-narrow">
          <p className="text-gray-500">Loading running coach...</p>
        </div>
      </div>
    );
  }

  if (!coach) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-narrow">
          <p className="text-red-600">{error || 'Unable to load running coach'}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell-chat">
      <div className="shrink-0 z-50">
        <Navbar />
        <div className="bg-white border-b px-4 py-3">
          <div className="chat-header-row">
            <div className="flex min-w-0 items-center gap-3">
              <h1 className="text-lg font-semibold truncate">Running Coach</h1>
            </div>
            <div className="chat-meta">
              <span className="font-mono">{coach.settings.model.split('/').pop()}</span>
              <span>{coach.total_tokens.toLocaleString()} tokens</span>
              <span>{formatCost(coach.total_cost)}</span>
              <Link
                to="/runner-profile?returnTo=/coach"
                className="px-3 py-1 rounded text-xs font-medium transition-colors bg-gray-100 text-gray-700 hover:bg-gray-200"
              >
                Runner Profile
              </Link>
              <button
                onClick={() => setShowSettings(true)}
                className="px-3 py-1 rounded text-xs font-medium transition-colors bg-gray-100 text-gray-700 hover:bg-gray-200"
              >
                Coach Settings
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="relative flex flex-1 overflow-hidden">
        <div className="flex-1 flex flex-col min-w-0">
          <div className="flex-1 overflow-y-auto">
            <div className="max-w-4xl mx-auto px-4 py-4 space-y-4">
              {visibleMessages.length === 0 && (
                <div className="text-sm text-gray-500 bg-gray-50 border border-gray-200 rounded-lg p-4">
                  Ask your coach anything. It already includes your latest training context and
                  coaching memory.
                </div>
              )}

              {visibleMessages.map((msg) => {
                const attached =
                  msg.role === 'assistant'
                    ? sessionsByMessageId.get(msg.id) ?? []
                    : [];
                return (
                  <div
                    key={msg.id}
                    className={`flex flex-col ${
                      msg.role === 'user' ? 'items-end' : 'items-start'
                    }`}
                  >
                    <div
                      className={`rounded-lg px-4 py-3 max-w-[85%] ${
                        msg.role === 'user'
                          ? 'bg-purple-100 text-purple-900'
                          : 'bg-gray-100 text-gray-900'
                      }`}
                    >
                      {msg.role === 'assistant' ? (
                        <div className="prose prose-sm max-w-none">
                          <ReactMarkdown>{msg.content}</ReactMarkdown>
                        </div>
                      ) : (
                        <p className="whitespace-pre-wrap text-sm">{msg.content}</p>
                      )}
                      {msg.role === 'assistant' && msg.cost > 0 && (
                        <div className="mt-2 text-xs text-gray-400">
                          {msg.total_tokens} tokens &middot; {formatCost(msg.cost)}
                        </div>
                      )}
                    </div>
                    {attached.length > 0 && (
                      <div className="mt-2 w-full max-w-[85%] space-y-2">
                        {attached.map((s) => (
                          <SessionCard
                            key={s.id}
                            session={s}
                            pending={pendingSessionId === s.id}
                            onStatus={(status) => handleSessionStatus(s.id, status)}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}

              {sending && (
                <div className="flex justify-start">
                  <div className="bg-gray-100 rounded-lg px-4 py-3">
                    <div className="flex items-center gap-2 text-gray-500 text-sm">
                      <div className="animate-spin h-4 w-4 border-2 border-purple-600 border-t-transparent rounded-full" />
                      Thinking...
                    </div>
                  </div>
                </div>
              )}

              <div ref={messagesEndRef} />
            </div>
          </div>

          <div className="bg-white border-t px-4 py-3">
            <div className="chat-input-row">
              <textarea
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                  }
                }}
                placeholder="Type your message... (Enter to send, Shift+Enter for newline)"
                rows={2}
                className="flex-1 w-full min-w-0 min-h-11 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 resize-none text-base sm:text-sm"
                disabled={sending}
              />
              <button
                onClick={handleSend}
                disabled={!input.trim() || sending}
                className="chat-send-btn bg-purple-600 text-white px-6 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
              >
                Send
              </button>
            </div>
            {error && <p className="max-w-4xl mx-auto text-red-600 text-xs mt-2">{error}</p>}
          </div>
        </div>
      </div>

      <CoachSettingsModal
        isOpen={showSettings}
        initial={coach.settings}
        onClose={() => setShowSettings(false)}
        onSave={handleSaveSettings}
        onResetCoach={handleResetCoach}
      />
    </div>
  );
}
