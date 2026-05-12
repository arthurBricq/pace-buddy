import { useEffect, useState } from 'react';
import Navbar from '../components/Navbar';
import SessionCard from '../components/SessionCard';
import {
  listTrainingSessions,
  updateTrainingSessionStatus,
} from '../api/training-sessions';
import type { SessionStatus, TrainingSession } from '../types';

const STATUS_OPTIONS: { label: string; value: SessionStatus | 'all' }[] = [
  { label: 'All', value: 'all' },
  { label: 'Suggested', value: 'suggested' },
  { label: 'Planned', value: 'planned' },
  { label: 'Done', value: 'done' },
  { label: 'Skipped', value: 'skipped' },
  { label: 'Rejected', value: 'rejected' },
];

export default function TrainingSessionsPage() {
  const [sessions, setSessions] = useState<TrainingSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [filter, setFilter] = useState<SessionStatus | 'all'>('all');
  const [pendingId, setPendingId] = useState<string | null>(null);

  const load = async (status: SessionStatus | 'all') => {
    setLoading(true);
    setError('');
    try {
      const data = await listTrainingSessions(
        status === 'all' ? undefined : status,
      );
      setSessions(data);
    } catch (e: any) {
      setError(e.message || 'Failed to load sessions');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load(filter);
  }, [filter]);

  const handleStatus = async (id: string, status: SessionStatus) => {
    setPendingId(id);
    try {
      const updated = await updateTrainingSessionStatus(id, status);
      // If the updated row no longer matches the active filter, drop it.
      if (filter !== 'all' && updated.status !== filter) {
        setSessions((prev) => prev.filter((s) => s.id !== id));
      } else {
        setSessions((prev) => prev.map((s) => (s.id === id ? updated : s)));
      }
    } catch (e: any) {
      setError(e.message || 'Failed to update status');
    } finally {
      setPendingId(null);
    }
  };

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-narrow section-stack">
        <div>
          <h1 className="text-2xl font-bold">Quality Sessions</h1>
          <p className="text-sm text-gray-500 mt-1">
            Coach-suggested workouts you can accept, skip, or mark as done.
          </p>
        </div>

        <div className="flex gap-2 flex-wrap">
          {STATUS_OPTIONS.map((opt) => {
            const active = filter === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                onClick={() => setFilter(opt.value)}
                className={`text-xs px-3 py-1 rounded-full border ${
                  active
                    ? 'bg-gray-900 text-white border-gray-900'
                    : 'bg-white text-gray-700 border-gray-300 hover:bg-gray-50'
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>

        {error && (
          <div className="theme-notice theme-notice-error">{error}</div>
        )}

        {loading ? (
          <p className="text-gray-500">Loading sessions…</p>
        ) : sessions.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-gray-700">
              No quality sessions{filter === 'all' ? ' yet' : ` with status “${filter}”`}.
            </p>
            <p className="mt-2 text-sm text-gray-500">
              Your coach will propose some when you ask.
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {sessions.map((s) => (
              <SessionCard
                key={s.id}
                session={s}
                onStatus={(status) => handleStatus(s.id, status)}
                pending={pendingId === s.id}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
