import { useEffect, useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import {
  getTraining,
  getTrainingActivities,
  removeActivityFromTraining,
  addActivityToTraining,
  getTrainingInsight,
  listTrainingInsights,
} from '../api/trainings';
import { listActivities } from '../api/activities';
import type { Training, Activity, TrainingInsightResponse, TrainingInsightRecord } from '../types';
import ReactMarkdown from 'react-markdown';
import Navbar from '../components/Navbar';
import TagBadge from '../components/TagBadge';
import { createChatFromInsight } from '../api/chats';

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h${m.toString().padStart(2, '0')}m`;
  return `${m}m${s.toString().padStart(2, '0')}s`;
}

function formatDistance(meters: number): string {
  return (meters / 1000).toFixed(2) + ' km';
}

function formatPace(avgSpeed: number): string {
  if (avgSpeed <= 0) return '-';
  const paceSeconds = 1000 / avgSpeed;
  const m = Math.floor(paceSeconds / 60);
  const s = Math.round(paceSeconds % 60);
  return `${m}:${s.toString().padStart(2, '0')} /km`;
}

export default function TrainingDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [training, setTraining] = useState<Training | null>(null);
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showAddForm, setShowAddForm] = useState(false);
  const [availableActivities, setAvailableActivities] = useState<Activity[]>([]);
  const [loadingActivities, setLoadingActivities] = useState(false);
  const [selectedActivityId, setSelectedActivityId] = useState('');

  // LLM Insight state
  const [insightLoading, setInsightLoading] = useState(false);
  const [insightResult, setInsightResult] = useState<TrainingInsightResponse | null>(null);
  const [showPrompt, setShowPrompt] = useState(false);
  const [insightError, setInsightError] = useState('');
  const [insightHistory, setInsightHistory] = useState<TrainingInsightRecord[]>([]);
  const [currentInsightId, setCurrentInsightId] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    Promise.all([getTraining(id), getTrainingActivities(id), listTrainingInsights(id)])
      .then(([t, a, h]) => {
        setTraining(t);
        setActivities(a);
        setInsightHistory(h);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [id]);

  const loadAvailableActivities = async (currentActivities: Activity[] = activities) => {
    if (!id) return;
    setLoadingActivities(true);
    try {
      const allActivities: Activity[] = [];
      let offset = 0;
      const limit = 100;

      for (let i = 0; i < 5; i++) {
        const page = await listActivities(limit, offset);
        if (page.length === 0) break;
        allActivities.push(...page);
        offset += limit;
        if (page.length < limit) break;
      }

      const activityIds = new Set(currentActivities.map((a) => a.id));
      const intervalActivities = allActivities.filter(
        (a) => a.tag === 'intervals' && !activityIds.has(a.id),
      );

      intervalActivities.sort(
        (a, b) =>
          new Date(b.start_date).getTime() - new Date(a.start_date).getTime(),
      );

      setAvailableActivities(intervalActivities);
    } catch (err: any) {
      console.error('Failed to load available activities:', err);
    } finally {
      setLoadingActivities(false);
    }
  };

  useEffect(() => {
    if (showAddForm && !loadingActivities) {
      loadAvailableActivities(activities);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showAddForm, activities.length]);

  const handleRemoveActivity = async (activityId: string) => {
    if (!id || !confirm('Remove this activity from the training?')) {
      return;
    }

    try {
      await removeActivityFromTraining(id, activityId);
      const updated = await getTrainingActivities(id!);
      setActivities(updated);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleAddActivity = async () => {
    if (!id || !selectedActivityId) return;

    try {
      await addActivityToTraining(id, selectedActivityId);
      const updated = await getTrainingActivities(id);
      setActivities(updated);
      setSelectedActivityId('');
      loadAvailableActivities(updated);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleInsight = async (promptType: 'overview' | 'suggestions') => {
    if (!id) return;
    setInsightLoading(true);
    setInsightError('');
    setInsightResult(null);
    setShowPrompt(false);
    try {
      const result = await getTrainingInsight(id, promptType);
      setInsightResult(result);
      setCurrentInsightId(result.id);
      // Refresh history to include the newly persisted insight
      listTrainingInsights(id).then(setInsightHistory).catch(() => {});
    } catch (err: any) {
      setInsightError(err.message || 'Failed to get insight');
    } finally {
      setInsightLoading(false);
    }
  };

  const openHistoryInsight = (record: TrainingInsightRecord) => {
    setInsightResult({
      id: record.id,
      display_label: record.display_label,
      full_prompt: record.full_prompt,
      response: record.response,
    });
    setCurrentInsightId(record.id);
    setShowPrompt(false);
    setInsightError('');
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-6xl mx-auto px-4 py-6">
          <p className="text-gray-500">Loading training...</p>
        </div>
      </div>
    );
  }

  if (error || !training) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-6xl mx-auto px-4 py-6">
          <p className="text-red-600">{error || 'Training not found'}</p>
          <Link to="/trainings" className="text-blue-600 hover:underline text-sm mt-2 inline-block">
            Back to trainings
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-6xl mx-auto px-4 py-6 space-y-6">
        <div>
          <Link to="/trainings" className="text-sm text-gray-500 hover:text-gray-700">
            &larr; Back to Trainings
          </Link>
          <h1 className="text-2xl font-bold mt-1">{training.name}</h1>
          {training.description && (
            <p className="text-gray-600 mt-2">{training.description}</p>
          )}
          <div className="flex flex-wrap gap-4 mt-2 text-sm text-gray-500">
            {training.race_goal && (
              <span>Goal: <span className="font-medium text-gray-700">{training.race_goal}</span></span>
            )}
            {training.start_date && (
              <span>From: {new Date(training.start_date).toLocaleDateString()}</span>
            )}
            {training.end_date && (
              <span>To: {new Date(training.end_date).toLocaleDateString()}</span>
            )}
            <span>Created {new Date(training.created_at).toLocaleDateString()}</span>
          </div>
        </div>

        {/* LLM Insights Section */}
        <div className="bg-white rounded-lg shadow p-4">
          <h2 className="text-lg font-semibold mb-3">AI Insights</h2>
          <div className="flex gap-3 mb-4">
            <button
              onClick={() => handleInsight('overview')}
              disabled={insightLoading}
              className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              {insightLoading ? 'Thinking...' : 'Critical Overview'}
            </button>
            <button
              onClick={() => handleInsight('suggestions')}
              disabled={insightLoading}
              className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              {insightLoading ? 'Thinking...' : 'Interval Suggestions'}
            </button>
          </div>

          {insightHistory.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-gray-500 mb-2">Previous AI Insights</h3>
              <div className="space-y-2">
                {insightHistory.map((record) => (
                  <button
                    key={record.id}
                    onClick={() => openHistoryInsight(record)}
                    className="w-full text-left bg-gray-50 hover:bg-purple-50 rounded-md px-3 py-2 border border-gray-200 hover:border-purple-300 transition-colors"
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium text-gray-800">{record.display_label}</span>
                      <span className="text-xs text-gray-400">
                        {new Date(record.created_at).toLocaleDateString(undefined, {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })}
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Insight Modal */}
        {(insightResult || insightLoading || insightError) && (
          <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
            <div className="bg-white rounded-xl shadow-2xl max-w-2xl w-full max-h-[80vh] flex flex-col">
              <div className="flex items-center justify-between px-6 py-4 border-b">
                <h3 className="text-lg font-semibold">AI Insight</h3>
                <button
                  onClick={() => {
                    setInsightResult(null);
                    setInsightError('');
                  }}
                  className="text-gray-400 hover:text-gray-600 text-2xl leading-none"
                >
                  &times;
                </button>
              </div>
              <div className="px-6 py-4 overflow-y-auto flex-1 space-y-4">
                {insightLoading && (
                  <div className="flex items-center gap-3 text-gray-500">
                    <div className="animate-spin h-5 w-5 border-2 border-purple-600 border-t-transparent rounded-full" />
                    <span>Generating insight...</span>
                  </div>
                )}

                {insightError && (
                  <div className="bg-red-50 text-red-700 p-3 rounded-md text-sm">
                    {insightError}
                  </div>
                )}

                {insightResult && (
                  <>
                    {/* User message */}
                    <div className="flex justify-end">
                      <div className="bg-purple-100 text-purple-900 rounded-lg px-4 py-2 max-w-[80%]">
                        <p className="font-medium text-sm">{insightResult.display_label}</p>
                        <button
                          onClick={() => setShowPrompt(!showPrompt)}
                          className="text-xs text-purple-600 hover:text-purple-800 mt-1"
                        >
                          {showPrompt ? 'Hide full prompt' : 'Show full prompt'}
                        </button>
                        {showPrompt && (
                          <pre className="mt-2 text-xs bg-purple-50 p-3 rounded overflow-x-auto whitespace-pre-wrap max-h-60 overflow-y-auto">
                            {insightResult.full_prompt}
                          </pre>
                        )}
                      </div>
                    </div>

                    {/* Assistant message */}
                    <div className="flex justify-start">
                      <div className="bg-gray-100 text-gray-900 rounded-lg px-4 py-3 max-w-[90%] prose prose-sm max-w-none">
                        <ReactMarkdown>{insightResult.response}</ReactMarkdown>
                      </div>
                    </div>

                    {/* Continue to Chat button */}
                    {currentInsightId && (
                      <div className="flex justify-center pt-2">
                        <button
                          onClick={async () => {
                            try {
                              const chat = await createChatFromInsight(currentInsightId);
                              navigate(`/chats/${chat.id}`);
                            } catch (err: any) {
                              setInsightError(err.message || 'Failed to create chat');
                            }
                          }}
                          className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 text-sm"
                        >
                          Continue to Chat
                        </button>
                      </div>
                    )}
                  </>
                )}
              </div>
            </div>
          </div>
        )}

        {error && <p className="text-red-600 text-sm">{error}</p>}

        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold">
              Activities ({activities.length})
            </h2>
            {!showAddForm && (
              <button
                onClick={() => setShowAddForm(true)}
                className="text-sm bg-blue-600 text-white px-3 py-1 rounded-md hover:bg-blue-700"
              >
                Add Activity
              </button>
            )}
          </div>

          {showAddForm && (
            <div className="bg-white rounded-lg shadow p-4 mb-4">
              <h3 className="text-md font-medium mb-3">Add Activity to Training</h3>
              {loadingActivities ? (
                <p className="text-gray-500 text-sm">Loading available activities...</p>
              ) : availableActivities.length === 0 ? (
                <p className="text-gray-500 text-sm">
                  No interval-tagged activities available to add. Make sure activities are tagged as "intervals".
                </p>
              ) : (
                <>
                  <select
                    value={selectedActivityId}
                    onChange={(e) => setSelectedActivityId(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 mb-3"
                  >
                    <option value="">Select an activity...</option>
                    {availableActivities.map((a) => (
                      <option key={a.id} value={a.id}>
                        {new Date(a.start_date).toLocaleDateString()} - {a.name} ({formatDistance(a.distance)})
                      </option>
                    ))}
                  </select>
                  <div className="flex gap-2">
                    <button
                      onClick={handleAddActivity}
                      disabled={!selectedActivityId}
                      className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                    >
                      Add
                    </button>
                    <button
                      onClick={() => {
                        setShowAddForm(false);
                        setSelectedActivityId('');
                      }}
                      className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
                    >
                      Cancel
                    </button>
                  </div>
                </>
              )}
            </div>
          )}

          {activities.length === 0 && !showAddForm ? (
            <p className="text-gray-500">
              No activities in this training yet. Click "Add Activity" to add interval-tagged activities.
            </p>
          ) : activities.length > 0 ? (
            <div className="bg-white rounded-lg shadow overflow-hidden">
              <table className="w-full text-sm">
                <thead className="bg-gray-50 text-gray-600">
                  <tr>
                    <th className="text-left px-4 py-3">Date</th>
                    <th className="text-left px-4 py-3">Name</th>
                    <th className="text-left px-4 py-3">Type</th>
                    <th className="text-right px-4 py-3">Distance</th>
                    <th className="text-right px-4 py-3">Duration</th>
                    <th className="text-right px-4 py-3">Pace</th>
                    <th className="text-center px-4 py-3">Tag</th>
                    <th className="text-right px-4 py-3">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {activities.map((a) => (
                    <tr key={a.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 text-gray-500">
                        {new Date(a.start_date).toLocaleDateString()}
                      </td>
                      <td className="px-4 py-3">
                        <Link
                          to={`/activities/${a.id}`}
                          className="text-blue-600 hover:underline"
                        >
                          {a.name}
                        </Link>
                      </td>
                      <td className="px-4 py-3 text-gray-500">{a.sport_type}</td>
                      <td className="px-4 py-3 text-right">
                        {formatDistance(a.distance)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        {formatDuration(a.moving_time)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        {formatPace(a.average_speed)}
                      </td>
                      <td className="px-4 py-3 text-center">
                        <TagBadge tag={a.tag} />
                      </td>
                      <td className="px-4 py-3 text-right">
                        <button
                          onClick={() => handleRemoveActivity(a.id)}
                          className="text-red-600 hover:text-red-800 text-sm"
                        >
                          Remove
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
