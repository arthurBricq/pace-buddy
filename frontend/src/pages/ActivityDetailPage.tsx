import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import {
  getActivity,
  getIntervals,
  type IntervalAlgorithm,
  updateActivityTag,
} from '../api/activities';
import type { ActivityDetail, ActivityTag, IntervalResult } from '../types';
import { useAuth } from '../hooks/useAuth';
import Navbar from '../components/Navbar';
import ActivityStats from '../components/ActivityStats';
import ActivityMap from '../components/ActivityMap';
import StreamChart from '../components/StreamChart';
import TagSelector from '../components/TagSelector';
import TagBadge from '../components/TagBadge';
import IntervalRecap from '../components/IntervalRecap';

export default function ActivityDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { user } = useAuth();
  const [detail, setDetail] = useState<ActivityDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [editingTag, setEditingTag] = useState(false);
  const [intervals, setIntervals] = useState<IntervalResult | null>(null);
  const [intervalAlgorithm, setIntervalAlgorithm] = useState<IntervalAlgorithm>('speed_based');

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
    
    // Only run interval parsing algorithm if activity is tagged as intervals
    if (detail.activity.tag === 'intervals') {
      getIntervals(id, intervalAlgorithm)
        .then(setIntervals)
        .catch((e) => console.warn('Failed to load intervals:', e));
    } else {
      // Clear intervals if activity is not tagged as intervals
      setIntervals(null);
    }
  }, [id, detail, intervalAlgorithm]);

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

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-narrow">
          <p className="text-gray-500">Loading activity...</p>
        </div>
      </div>
    );
  }

  if (error || !detail) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-narrow">
          <p className="text-red-600">{error || 'Activity not found'}</p>
          <Link to="/activities" className="text-blue-600 hover:underline text-sm mt-2 inline-block">
            Back to activities
          </Link>
        </div>
      </div>
    );
  }

  const { activity, streams, laps } = detail;
  const distanceStream = streams.find((s) => s.stream_type === 'distance');
  const timeStream = streams.find((s) => s.stream_type === 'time');

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-narrow section-stack">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
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

        <ActivityStats activity={activity} />

        {activity.summary_polyline && (
          <ActivityMap polyline={activity.summary_polyline} />
        )}

        {intervals?.is_interval_workout && (
          <IntervalRecap intervals={intervals} masCurrent={user?.mas_current ?? null} />
        )}

        {activity.tag === 'intervals' && (
          <div className="bg-white rounded-lg shadow p-4">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Interval Algorithm
            </label>
            <select
              value={intervalAlgorithm}
              onChange={(e) => setIntervalAlgorithm(e.target.value as IntervalAlgorithm)}
              className="w-full sm:w-auto rounded border border-gray-300 px-3 py-2 text-sm bg-white"
            >
              <option value="speed_based">Speed based</option>
              <option value="manual_laps">Manual laps</option>
            </select>
          </div>
        )}

        {(streams.length > 0 || laps.length > 0) && (
          <StreamChart
            streams={streams}
            distanceStream={distanceStream}
            timeStream={timeStream}
            segments={intervals?.segments}
            laps={laps}
          />
        )}
      </div>
    </div>
  );
}
