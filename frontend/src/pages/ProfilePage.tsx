import { useEffect, useState } from 'react';
import { getProfile } from '../api/auth';
import type { ProfileResponse, RunningStats } from '../types';
import Navbar from '../components/Navbar';

function formatDistance(meters: number): string {
  return (meters / 1000).toFixed(1) + ' km';
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatPace(speedMps: number | null): string {
  if (!speedMps || speedMps <= 0) return '-';
  const paceSecsPerKm = 1000 / speedMps;
  const mins = Math.floor(paceSecsPerKm / 60);
  const secs = Math.round(paceSecsPerKm % 60);
  return `${mins}:${secs.toString().padStart(2, '0')} /km`;
}

function StatsCard({ title, stats }: { title: string; stats: RunningStats }) {
  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-semibold mb-4">{title}</h3>
      <div className="space-y-3">
        <div className="flex justify-between">
          <span className="text-sm text-gray-500">Distance</span>
          <span className="text-sm font-medium">{formatDistance(stats.total_distance_m)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-sm text-gray-500">Time</span>
          <span className="text-sm font-medium">{formatDuration(stats.total_time_s)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-sm text-gray-500">Elevation</span>
          <span className="text-sm font-medium">{Math.round(stats.total_elevation_m)} m</span>
        </div>
        <div className="flex justify-between">
          <span className="text-sm text-gray-500">Avg Pace</span>
          <span className="text-sm font-medium">{formatPace(stats.avg_speed_mps)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-sm text-gray-500">Runs</span>
          <span className="text-sm font-medium">{stats.activity_count}</span>
        </div>
        {stats.interval_count != null && (
          <div className="flex justify-between">
            <span className="text-sm text-gray-500">Interval sessions</span>
            <span className="text-sm font-medium">{stats.interval_count}</span>
          </div>
        )}
      </div>
    </div>
  );
}

export default function ProfilePage() {
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    getProfile()
      .then(setProfile)
      .catch((err: any) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-6xl mx-auto px-4 py-6">
          <p className="text-gray-500">Loading profile...</p>
        </div>
      </div>
    );
  }

  if (error || !profile) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-6xl mx-auto px-4 py-6">
          <p className="text-red-600">{error || 'Failed to load profile'}</p>
        </div>
      </div>
    );
  }

  const { user, stats } = profile;

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-6xl mx-auto px-4 py-6 space-y-6">
        <div className="bg-white rounded-lg shadow p-6">
          <h1 className="text-2xl font-bold">{user.display_name}</h1>
          <p className="text-sm text-gray-500 mt-1">@{user.username}</p>
          <p className="text-sm text-gray-400 mt-1">
            Member since {new Date(user.created_at).toLocaleDateString()}
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-3">
          <StatsCard title="Year to Date" stats={stats.ytd} />
          <StatsCard title="Last Year" stats={stats.last_year} />
          <StatsCard title="All Time" stats={stats.all_time} />
        </div>
      </div>
    </div>
  );
}
