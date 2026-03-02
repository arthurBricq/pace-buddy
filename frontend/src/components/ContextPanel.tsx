import { useState, useEffect } from 'react';
import { addContext, type ContextRequest } from '../api/chats';
import { listActivities } from '../api/activities';
import { listTrainings } from '../api/trainings';
import type { Activity, Training } from '../types';

interface ContextPanelProps {
  chatId: string;
  onContextAdded: () => void;
  onClose?: () => void;
}

type PanelSection = 'last_activities' | 'activity_detail' | 'weekly_stats' | 'training_recap' | null;

export default function ContextPanel({ chatId, onContextAdded, onClose }: ContextPanelProps) {
  const [activeSection, setActiveSection] = useState<PanelSection>(null);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState('');

  // Form state
  const [count, setCount] = useState(5);
  const [activities, setActivities] = useState<Activity[]>([]);
  const [selectedActivityId, setSelectedActivityId] = useState('');
  const [trainings, setTrainings] = useState<Training[]>([]);
  const [selectedTrainingId, setSelectedTrainingId] = useState('');
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');

  useEffect(() => {
    if (activeSection === 'activity_detail' && activities.length === 0) {
      listActivities(20, 0).then(setActivities).catch(() => {});
    }
    if (activeSection === 'training_recap' && trainings.length === 0) {
      listTrainings().then(setTrainings).catch(() => {});
    }
  }, [activeSection]);

  const handleAdd = async (request: ContextRequest) => {
    setAdding(true);
    setError('');
    try {
      await addContext(chatId, request);
      setActiveSection(null);
      onContextAdded();
    } catch (err: any) {
      setError(err.message);
    } finally {
      setAdding(false);
    }
  };

  const toggle = (section: PanelSection) => {
    setActiveSection(activeSection === section ? null : section);
    setError('');
  };

  return (
    <div className="chat-context-drawer shrink-0 overflow-y-auto">
      <div className="px-4 py-3 border-b flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-700">Add Context</h3>
        {onClose && (
          <button
            type="button"
            onClick={onClose}
            className="rounded-md px-2 py-1 text-sm text-gray-500 hover:bg-gray-100 hover:text-gray-800 lg:hidden"
          >
            Close
          </button>
        )}
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {/* Last N activities */}
        <div>
          <button
            onClick={() => toggle('last_activities')}
            className="w-full text-left px-3 py-2 rounded-md text-sm bg-gray-50 hover:bg-gray-100 font-medium text-gray-700"
          >
            Last N activities
          </button>
          {activeSection === 'last_activities' && (
            <div className="mt-2 px-3 space-y-2">
              <label className="text-xs text-gray-500">Count</label>
              <input
                type="number"
                min={1}
                max={50}
                value={count}
                onChange={(e) => setCount(parseInt(e.target.value) || 5)}
                className="w-full px-2 py-1 border rounded text-sm"
              />
              <button
                onClick={() => handleAdd({ context_type: 'last_activities', count })}
                disabled={adding}
                className="w-full bg-purple-600 text-white py-1 rounded text-sm hover:bg-purple-700 disabled:opacity-50"
              >
                {adding ? 'Adding...' : 'Add'}
              </button>
            </div>
          )}
        </div>

        {/* Activity detail */}
        <div>
          <button
            onClick={() => toggle('activity_detail')}
            className="w-full text-left px-3 py-2 rounded-md text-sm bg-gray-50 hover:bg-gray-100 font-medium text-gray-700"
          >
            Activity detail
          </button>
          {activeSection === 'activity_detail' && (
            <div className="mt-2 px-3 space-y-2">
              <label className="text-xs text-gray-500">Activity</label>
              <select
                value={selectedActivityId}
                onChange={(e) => setSelectedActivityId(e.target.value)}
                className="w-full px-2 py-1 border rounded text-sm"
              >
                <option value="">Select...</option>
                {activities.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name} ({new Date(a.start_date).toLocaleDateString()})
                  </option>
                ))}
              </select>
              <button
                onClick={() =>
                  selectedActivityId &&
                  handleAdd({ context_type: 'activity_detail', activity_id: selectedActivityId })
                }
                disabled={adding || !selectedActivityId}
                className="w-full bg-purple-600 text-white py-1 rounded text-sm hover:bg-purple-700 disabled:opacity-50"
              >
                {adding ? 'Adding...' : 'Add'}
              </button>
            </div>
          )}
        </div>

        {/* Weekly stats */}
        <div>
          <button
            onClick={() => toggle('weekly_stats')}
            className="w-full text-left px-3 py-2 rounded-md text-sm bg-gray-50 hover:bg-gray-100 font-medium text-gray-700"
          >
            Weekly stats
          </button>
          {activeSection === 'weekly_stats' && (
            <div className="mt-2 px-3 space-y-2">
              <label className="text-xs text-gray-500">From</label>
              <input
                type="date"
                value={fromDate}
                onChange={(e) => setFromDate(e.target.value)}
                className="w-full px-2 py-1 border rounded text-sm"
              />
              <label className="text-xs text-gray-500">To</label>
              <input
                type="date"
                value={toDate}
                onChange={(e) => setToDate(e.target.value)}
                className="w-full px-2 py-1 border rounded text-sm"
              />
              <button
                onClick={() =>
                  fromDate &&
                  toDate &&
                  handleAdd({ context_type: 'weekly_stats', from: fromDate, to: toDate })
                }
                disabled={adding || !fromDate || !toDate}
                className="w-full bg-purple-600 text-white py-1 rounded text-sm hover:bg-purple-700 disabled:opacity-50"
              >
                {adding ? 'Adding...' : 'Add'}
              </button>
            </div>
          )}
        </div>

        {/* Training recap */}
        <div>
          <button
            onClick={() => toggle('training_recap')}
            className="w-full text-left px-3 py-2 rounded-md text-sm bg-gray-50 hover:bg-gray-100 font-medium text-gray-700"
          >
            Training recap
          </button>
          {activeSection === 'training_recap' && (
            <div className="mt-2 px-3 space-y-2">
              <label className="text-xs text-gray-500">Training</label>
              <select
                value={selectedTrainingId}
                onChange={(e) => setSelectedTrainingId(e.target.value)}
                className="w-full px-2 py-1 border rounded text-sm"
              >
                <option value="">Select...</option>
                {trainings.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
              <button
                onClick={() =>
                  selectedTrainingId &&
                  handleAdd({ context_type: 'training_recap', training_id: selectedTrainingId })
                }
                disabled={adding || !selectedTrainingId}
                className="w-full bg-purple-600 text-white py-1 rounded text-sm hover:bg-purple-700 disabled:opacity-50"
              >
                {adding ? 'Adding...' : 'Add'}
              </button>
            </div>
          )}
        </div>

        {error && <p className="text-red-600 text-xs mt-2">{error}</p>}
      </div>
    </div>
  );
}
