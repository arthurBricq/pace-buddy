import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listActivities, syncActivities, updateActivityTag } from '../api/activities';
import type { Activity, ActivityTag } from '../types';
import TagBadge from '../components/TagBadge';
import TagSelector from '../components/TagSelector';
import Navbar from '../components/Navbar';

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

export default function ActivityListPage() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState('');
  const [editingTag, setEditingTag] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const limit = 50;

  const load = async (off = 0) => {
    setLoading(true);
    try {
      const data = await listActivities(limit, off);
      setActivities(data);
      setOffset(off);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleSync = async () => {
    setSyncing(true);
    setError('');
    try {
      const result = await syncActivities();
      alert(`Synced ${result.synced} activities`);
      load();
    } catch (err: any) {
      setError(err.message);
    } finally {
      setSyncing(false);
    }
  };

  const handleTagChange = async (activityId: string, tag: ActivityTag) => {
    try {
      await updateActivityTag(activityId, tag);
      setActivities((prev) =>
        prev.map((a) => (a.id === activityId ? { ...a, tag } : a)),
      );
      setEditingTag(null);
    } catch (err: any) {
      setError(err.message);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-6xl mx-auto px-4 py-6">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-xl font-bold">Activities</h1>
          <button
            onClick={handleSync}
            disabled={syncing}
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 text-sm"
          >
            {syncing ? 'Syncing...' : 'Sync from Strava'}
          </button>
        </div>

        {error && <p className="text-red-600 text-sm mb-4">{error}</p>}

        {loading ? (
          <p className="text-gray-500">Loading activities...</p>
        ) : activities.length === 0 ? (
          <p className="text-gray-500">
            No activities yet. Sync from Strava to get started.
          </p>
        ) : (
          <>
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
                        {editingTag === a.id ? (
                          <TagSelector
                            current={a.tag}
                            onChange={(tag) => handleTagChange(a.id, tag)}
                          />
                        ) : (
                          <button onClick={() => setEditingTag(a.id)}>
                            <TagBadge tag={a.tag} />
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="flex justify-between mt-4">
              <button
                onClick={() => load(Math.max(0, offset - limit))}
                disabled={offset === 0}
                className="text-sm text-blue-600 disabled:text-gray-400"
              >
                Previous
              </button>
              <button
                onClick={() => load(offset + limit)}
                disabled={activities.length < limit}
                className="text-sm text-blue-600 disabled:text-gray-400"
              >
                Next
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
