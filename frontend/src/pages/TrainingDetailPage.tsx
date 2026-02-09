import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import {
  getTraining,
  getTrainingActivities,
  removeActivityFromTraining,
  addActivityToTraining,
} from '../api/trainings';
import { listActivities } from '../api/activities';
import type { Training, Activity } from '../types';
import Navbar from '../components/Navbar';
import TagBadge from '../components/TagBadge';

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
  const [training, setTraining] = useState<Training | null>(null);
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showAddForm, setShowAddForm] = useState(false);
  const [availableActivities, setAvailableActivities] = useState<Activity[]>([]);
  const [loadingActivities, setLoadingActivities] = useState(false);
  const [selectedActivityId, setSelectedActivityId] = useState('');

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    Promise.all([getTraining(id), getTrainingActivities(id)])
      .then(([t, a]) => {
        setTraining(t);
        setActivities(a);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [id]);

  const loadAvailableActivities = async (currentActivities: Activity[] = activities) => {
    if (!id) return;
    setLoadingActivities(true);
    try {
      // Fetch a reasonable number of activities to find interval-tagged ones
      // We'll fetch multiple pages if needed
      const allActivities: Activity[] = [];
      let offset = 0;
      const limit = 100;
      
      // Fetch up to 500 activities (5 pages) to find interval activities
      for (let i = 0; i < 5; i++) {
        const page = await listActivities(limit, offset);
        if (page.length === 0) break;
        allActivities.push(...page);
        offset += limit;
        if (page.length < limit) break;
      }

      // Filter for interval-tagged activities that aren't already in the training
      const activityIds = new Set(currentActivities.map((a) => a.id));
      const intervalActivities = allActivities.filter(
        (a) => a.tag === 'intervals' && !activityIds.has(a.id),
      );

      // Sort by date (most recent first)
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
      // Reload activities
      const updated = await getTrainingActivities(id);
      setActivities(updated);
      setSelectedActivityId('');
      // Reload available activities with updated list
      loadAvailableActivities(updated);
    } catch (err: any) {
      setError(err.message);
    }
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
          <p className="text-sm text-gray-500 mt-2">
            Created {new Date(training.created_at).toLocaleDateString()}
          </p>
        </div>

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
