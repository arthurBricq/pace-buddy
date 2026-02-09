import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getActivity, getIntervals, updateActivityTag } from '../api/activities';
import {
  getActivityTrainings,
  listTrainings,
  addActivityToTraining,
  removeActivityFromTraining,
} from '../api/trainings';
import type { ActivityDetail, ActivityTag, IntervalResult, Training } from '../types';
import Navbar from '../components/Navbar';
import ActivityStats from '../components/ActivityStats';
import ActivityMap from '../components/ActivityMap';
import StreamChart from '../components/StreamChart';
import TagSelector from '../components/TagSelector';
import TagBadge from '../components/TagBadge';
import IntervalRecap from '../components/IntervalRecap';

export default function ActivityDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [detail, setDetail] = useState<ActivityDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [editingTag, setEditingTag] = useState(false);
  const [intervals, setIntervals] = useState<IntervalResult | null>(null);
  const [activityTrainings, setActivityTrainings] = useState<Training[]>([]);
  const [allTrainings, setAllTrainings] = useState<Training[]>([]);
  const [showAddTraining, setShowAddTraining] = useState(false);
  const [selectedTrainingId, setSelectedTrainingId] = useState('');

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    getActivity(id)
      .then(setDetail)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [id]);

  useEffect(() => {
    if (!id || !detail) return;
    getIntervals(id)
      .then(setIntervals)
      .catch((e) => console.warn('Failed to load intervals:', e));
  }, [id, detail]);

  useEffect(() => {
    if (!id) return;
    Promise.all([getActivityTrainings(id), listTrainings()])
      .then(([at, all]) => {
        setActivityTrainings(at);
        setAllTrainings(all);
      })
      .catch((e) => console.warn('Failed to load trainings:', e));
  }, [id]);

  const handleTagChange = async (tag: ActivityTag) => {
    if (!id || !detail) return;
    try {
      await updateActivityTag(id, tag);
      setDetail({
        ...detail,
        activity: { ...detail.activity, tag },
      });
      setEditingTag(false);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleAddToTraining = async () => {
    if (!id || !selectedTrainingId) return;
    try {
      await addActivityToTraining(selectedTrainingId, id);
      const updated = await getActivityTrainings(id);
      setActivityTrainings(updated);
      setSelectedTrainingId('');
      setShowAddTraining(false);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleRemoveFromTraining = async (trainingId: string) => {
    if (!id || !confirm('Remove this activity from the training?')) return;
    try {
      await removeActivityFromTraining(trainingId, id);
      const updated = await getActivityTrainings(id);
      setActivityTrainings(updated);
    } catch (err: any) {
      setError(err.message);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-6">
          <p className="text-gray-500">Loading activity...</p>
        </div>
      </div>
    );
  }

  if (error || !detail) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-6">
          <p className="text-red-600">{error || 'Activity not found'}</p>
          <Link to="/activities" className="text-blue-600 hover:underline text-sm mt-2 inline-block">
            Back to activities
          </Link>
        </div>
      </div>
    );
  }

  const { activity, streams } = detail;
  const distanceStream = streams.find((s) => s.stream_type === 'distance');
  const timeStream = streams.find((s) => s.stream_type === 'time');

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-4xl mx-auto px-4 py-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <Link to="/activities" className="text-sm text-gray-500 hover:text-gray-700">
              &larr; Back
            </Link>
            <h1 className="text-2xl font-bold mt-1">{activity.name}</h1>
            <p className="text-sm text-gray-500">
              {new Date(activity.start_date).toLocaleDateString('en-US', {
                weekday: 'long',
                year: 'numeric',
                month: 'long',
                day: 'numeric',
              })}{' '}
              &middot; {activity.sport_type}
            </p>
          </div>
          <div>
            {editingTag ? (
              <TagSelector current={activity.tag} onChange={handleTagChange} />
            ) : (
              <button onClick={() => setEditingTag(true)}>
                <TagBadge tag={activity.tag} />
              </button>
            )}
          </div>
        </div>

        {activity.tag === 'intervals' && (
          <div className="bg-white rounded-lg shadow p-4">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-lg font-semibold">Trainings</h2>
              {!showAddTraining && (
                <button
                  onClick={() => setShowAddTraining(true)}
                  className="text-sm text-blue-600 hover:text-blue-800"
                >
                  Add to Training
                </button>
              )}
            </div>

            {showAddTraining && (
              <div className="mb-3 p-3 bg-gray-50 rounded-md">
                <select
                  value={selectedTrainingId}
                  onChange={(e) => setSelectedTrainingId(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 mb-2"
                >
                  <option value="">Select a training...</option>
                  {allTrainings
                    .filter((t) => !activityTrainings.some((at) => at.id === t.id))
                    .map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.name}
                      </option>
                    ))}
                </select>
                <div className="flex gap-2">
                  <button
                    onClick={handleAddToTraining}
                    disabled={!selectedTrainingId}
                    className="bg-blue-600 text-white px-3 py-1 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                  >
                    Add
                  </button>
                  <button
                    onClick={() => {
                      setShowAddTraining(false);
                      setSelectedTrainingId('');
                    }}
                    className="bg-gray-200 text-gray-700 px-3 py-1 rounded-md hover:bg-gray-300 text-sm"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}

            {activityTrainings.length === 0 ? (
              <p className="text-gray-500 text-sm">
                This activity is not in any trainings yet.
              </p>
            ) : (
              <ul className="space-y-2">
                {activityTrainings.map((t) => (
                  <li
                    key={t.id}
                    className="flex items-center justify-between p-2 bg-gray-50 rounded-md"
                  >
                    <Link
                      to={`/trainings/${t.id}`}
                      className="text-blue-600 hover:underline"
                    >
                      {t.name}
                    </Link>
                    <button
                      onClick={() => handleRemoveFromTraining(t.id)}
                      className="text-red-600 hover:text-red-800 text-sm"
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        <ActivityStats activity={activity} />

        {activity.summary_polyline && (
          <ActivityMap polyline={activity.summary_polyline} />
        )}

        {intervals?.is_interval_workout && (
          <IntervalRecap intervals={intervals} />
        )}

        {streams.length > 0 && (
          <StreamChart
            streams={streams}
            distanceStream={distanceStream}
            timeStream={timeStream}
            segments={intervals?.segments}
          />
        )}
      </div>
    </div>
  );
}
