import {useEffect, useState} from 'react';
import {useParams, Link, useNavigate} from 'react-router-dom';
import {
  getTraining,
  getTrainingActivities,
  getTrainingInsight,
  listTrainingInsights,
} from '../api/trainings';
import type {Training, Activity, TrainingInsightResponse, TrainingInsightRecord} from '../types';
import ReactMarkdown from 'react-markdown';
import Navbar from '../components/Navbar';
import TagBadge from '../components/TagBadge';
import {createChatFromInsight} from '../api/chats';
import ChatSettingsModal from '../components/ChatSettingsModal';

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

function formatCost(cost: number | null | undefined): string {
  if (cost === null || cost === undefined || cost === 0) return '$0';
  return `$${cost.toFixed(4)}`;
}

export default function TrainingDetailPage() {
  const {id} = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [training, setTraining] = useState<Training | null>(null);
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // LLM Insight state
  const [insightLoading, setInsightLoading] = useState(false);
  const [insightResult, setInsightResult] = useState<TrainingInsightResponse | null>(null);
  const [showPrompt, setShowPrompt] = useState(false);
  const [insightError, setInsightError] = useState('');
  const [insightHistory, setInsightHistory] = useState<TrainingInsightRecord[]>([]);
  const [currentInsightId, setCurrentInsightId] = useState<string | null>(null);
  const [currentInsightModel, setCurrentInsightModel] = useState<string>('google/gemini-2.5-flash');
  const [showChatSettings, setShowChatSettings] = useState(false);
  const [showInsightSettings, setShowInsightSettings] = useState(false);
  const [pendingPromptType, setPendingPromptType] = useState<'overview' | 'suggestions'>('overview');

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

  const handleInsight = async (promptType: 'overview' | 'suggestions', model: string) => {
    if (!id) return;
    setInsightLoading(true);
    setInsightError('');
    setInsightResult(null);
    setShowPrompt(false);
    try {
      const result = await getTrainingInsight(id, promptType, model);
      setInsightResult(result);
      setCurrentInsightId(result.id);
      setCurrentInsightModel(model);
      // Refresh history to include the newly persisted insight
      listTrainingInsights(id).then(setInsightHistory).catch(() => {
      });
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
    setCurrentInsightModel(record.model ?? 'google/gemini-2.5-flash');
    setShowPrompt(false);
    setInsightError('');
  };

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar/>
        <div className="page-container-wide">
          <p className="text-gray-500">Loading training...</p>
        </div>
      </div>
    );
  }

  if (error || !training) {
    return (
      <div className="app-shell">
        <Navbar/>
        <div className="page-container-wide">
          <p className="text-red-600">{error || 'Training not found'}</p>
          <Link to="/trainings" className="text-blue-600 hover:underline text-sm mt-2 inline-block">
            Back to trainings
          </Link>
        </div>
      </div>
    );
  }

  const intervalActivities = activities.filter((a) => a.tag === 'intervals');
  const longRunActivities = activities.filter((a) => a.tag === 'long_run');

  return (
    <div className="app-shell">
      <Navbar/>
      <div className="page-container-wide section-stack">
        <div>
          <Link to="/trainings" className="text-sm text-gray-500 hover:text-gray-700">
            &larr; Back to Trainings
          </Link>
          <h1 className="text-2xl font-bold mt-1">{training.name}</h1>
          {training.description && (
            <p className="text-gray-600 mt-2">{training.description}</p>
          )}
          <div className="flex flex-wrap gap-4 mt-2 text-sm text-gray-500">
            {training.race_distance && (
              <span>Race Distance: <span className="font-medium text-gray-700">{training.race_distance}</span></span>
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
        <div className="card-compact">
          <h2 className="text-lg font-semibold mb-4">AI Insights</h2>
          <div className="button-row-wrap mb-4">
            <button
              onClick={() => {
                setPendingPromptType('overview');
                setShowInsightSettings(true);
              }}
              disabled={insightLoading}
              className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              {insightLoading ? 'Thinking...' : 'Critical Overview'}
            </button>
            <button
              onClick={() => {
                setPendingPromptType('suggestions');
                setShowInsightSettings(true);
              }}
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
                      <div className="flex flex-col">
                        <span className="text-sm font-medium text-gray-800">{record.display_label}</span>
                        <div className="flex items-center gap-2 mt-1">
                          {record.model && (
                            <span className="text-xs text-gray-500 font-mono">
                              {record.model.split('/').pop()}
                            </span>
                          )}
                          {record.cost !== null && record.cost !== undefined && record.cost > 0 && (
                            <span className="text-xs text-gray-500">
                              {formatCost(record.cost)}
                            </span>
                          )}
                        </div>
                      </div>
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
            <div className="modal-card max-w-2xl max-h-[80vh] flex flex-col">
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
                    <div className="animate-spin h-5 w-5 border-2 border-purple-600 border-t-transparent rounded-full"/>
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
                          <pre
                            className="mt-2 text-xs bg-purple-50 p-3 rounded overflow-x-auto whitespace-pre-wrap max-h-60 overflow-y-auto">
                            {insightResult.full_prompt}
                          </pre>
                        )}
                      </div>
                    </div>

                    {/* Assistant message */}
                    <div className="flex justify-start">
                      <div
                        className="bg-gray-100 text-gray-900 rounded-lg px-4 py-3 max-w-[90%] prose prose-sm max-w-none">
                        <ReactMarkdown>{insightResult.response}</ReactMarkdown>
                      </div>
                    </div>

                    {/* Continue to Chat button */}
                    {currentInsightId && (
                      <div className="flex justify-center pt-2">
                        <button
                          onClick={() => setShowChatSettings(true)}
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

        {/* Insight Settings Modal (model selection before generating) */}
        <ChatSettingsModal
          isOpen={showInsightSettings}
          onClose={() => setShowInsightSettings(false)}
          onConfirm={(model) => {
            setShowInsightSettings(false);
            handleInsight(pendingPromptType, model);
          }}
          hideConversationLength
          title={pendingPromptType === 'overview' ? 'Critical Overview' : 'Interval Suggestions'}
          confirmLabel="Generate"
        />

        {/* Chat Settings Modal (conversation length only, model inherited from insight) */}
        <ChatSettingsModal
          isOpen={showChatSettings}
          onClose={() => setShowChatSettings(false)}
          defaultModel={currentInsightModel}
          hideModelSelector
          title="Continue to Chat"
          confirmLabel="Start Chat"
          onConfirm={async (model, conversationLength) => {
            if (!currentInsightId) return;
            setShowChatSettings(false);
            try {
              const chat = await createChatFromInsight(currentInsightId, model, conversationLength);
              navigate(`/chats/${chat.id}`);
            } catch (err: any) {
              setInsightError(err.message || 'Failed to create chat');
            }
          }}
        />

        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold">
              Intervals and Workouts
            </h2>
          </div>

          {activities.length === 0 ? (
            <p className="text-gray-500">
              No quality activities in this training range yet. Activities are derived from the
              training time window and quality tags (intervals + long runs).
            </p>
          ) : activities.length > 0 ? (
            <div className="space-y-6">
              <div>
                <h3 className="text-md font-semibold mb-2">Intervals ({intervalActivities.length})</h3>
                {intervalActivities.length === 0 ? (
                  <p className="text-sm text-gray-500">No interval activities in this training range.</p>
                ) : (
                  <div className="data-table-wrap">
                    <table className="data-table-wide">
                      <thead className="bg-gray-50 text-gray-600">
                      <tr>
                        <th className="text-left px-4 py-3">Date</th>
                        <th className="text-left px-4 py-3">Name</th>
                        <th className="text-left px-4 py-3">Type</th>
                        <th className="text-right px-4 py-3">Distance</th>
                        <th className="text-right px-4 py-3">Duration</th>
                        <th className="text-right px-4 py-3">Pace</th>
                        <th className="text-center px-4 py-3">Tag</th>
                      </tr>
                      </thead>
                      <tbody className="divide-y divide-gray-100">
                      {intervalActivities.map((a) => (
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
                            <TagBadge tag={a.tag}/>
                          </td>
                        </tr>
                      ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>

              <div>
                <h3 className="text-md font-semibold mb-2">Long Runs</h3>
                {longRunActivities.length === 0 ? (
                  <p className="text-sm text-gray-500">No long runs in this training range.</p>
                ) : (
                  <div className="data-table-wrap">
                    <table className="data-table-wide">
                      <thead className="bg-gray-50 text-gray-600">
                      <tr>
                        <th className="text-left px-4 py-3">Date</th>
                        <th className="text-left px-4 py-3">Name</th>
                        <th className="text-left px-4 py-3">Type</th>
                        <th className="text-right px-4 py-3">Distance</th>
                        <th className="text-right px-4 py-3">Duration</th>
                        <th className="text-right px-4 py-3">Pace</th>
                        <th className="text-center px-4 py-3">Tag</th>
                      </tr>
                      </thead>
                      <tbody className="divide-y divide-gray-100">
                      {longRunActivities.map((a) => (
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
                            <TagBadge tag={a.tag}/>
                          </td>
                        </tr>
                      ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
