import { useEffect, useState } from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import { getProfile, getAiCostSummary } from '../api/auth';
import { getStravaStatus, getStravaLink, disconnectStrava } from '../api/strava';
import type { ProfileResponse, RunningStats, StravaStatus, ExpensiveRequest, AiCostSummary } from '../types';
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

function formatCost(cost: number): string {
  if (cost === 0) return '$0';
  return `$${cost.toFixed(4)}`;
}

export default function ProfilePage() {
  const [searchParams] = useSearchParams();
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [stravaStatus, setStravaStatus] = useState<StravaStatus | null>(null);
  const [aiCostSummary, setAiCostSummary] = useState<AiCostSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(searchParams.get('error') || '');

  useEffect(() => {
    Promise.all([
      getProfile().catch((e) => {
        setError(e.message);
        return null;
      }),
      getStravaStatus().catch(() => null),
      getAiCostSummary().catch(() => null),
    ]).then(([p, s, a]) => {
      setProfile(p);
      setStravaStatus(s);
      setAiCostSummary(a);
    }).finally(() => setLoading(false));
  }, []);

  const handleLinkStrava = async () => {
    try {
      const { url } = await getStravaLink();
      window.location.href = url;
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleDisconnectStrava = async () => {
    if (!confirm('Disconnect Strava? This will delete all synced activities and streams.')) return;
    try {
      await disconnectStrava();
      setStravaStatus({ linked: false });
    } catch (err: any) {
      setError(err.message);
    }
  };

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

        {/* Strava Connection Section */}
        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold mb-4">Strava Connection</h2>
          {error && <p className="text-red-600 text-sm mb-2">{error}</p>}
          {stravaStatus && stravaStatus.linked ? (
            <div className="space-y-3">
              <p className="text-green-600 font-medium">Strava connected</p>
              <p className="text-sm text-gray-500">
                Athlete ID: {stravaStatus.athlete_id}
              </p>
              <div className="flex items-center gap-3">
                <a
                  href="https://www.strava.com/settings/apps"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-orange-600 hover:text-orange-800 underline"
                >
                  Manage on Strava
                </a>
                <button
                  onClick={handleDisconnectStrava}
                  className="text-sm text-red-600 hover:text-red-800"
                >
                  Disconnect
                </button>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <p className="text-gray-600">
                Connect your Strava account to sync activities.
              </p>
              <button
                onClick={handleLinkStrava}
                className="bg-orange-500 text-white px-4 py-2 rounded-md hover:bg-orange-600"
              >
                Connect Strava
              </button>
            </div>
          )}
        </div>

        {/* AI Cost Summary Section */}
        {aiCostSummary && (
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4">AI Usage</h2>
            <div className="mb-6">
              <div className="flex items-baseline gap-2">
                <span className="text-sm text-gray-500">Total Cost:</span>
                <span className="text-2xl font-bold text-purple-600">
                  {formatCost(aiCostSummary.total_cost)}
                </span>
              </div>
            </div>

            {aiCostSummary.expensive_requests.length > 0 && (
              <div>
                <h3 className="text-md font-medium text-gray-700 mb-3">
                  Most Expensive Requests
                </h3>
                <div className="space-y-2">
                  {aiCostSummary.expensive_requests.slice(0, 10).map((req) => (
                    <div
                      key={req.id}
                      className="flex items-center justify-between p-3 bg-gray-50 rounded-md hover:bg-gray-100 transition-colors"
                    >
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <span
                            className={`text-xs px-2 py-1 rounded ${
                              req.type === 'insight'
                                ? 'bg-purple-100 text-purple-700'
                                : 'bg-blue-100 text-blue-700'
                            }`}
                          >
                            {req.type === 'insight' ? 'Insight' : 'Chat'}
                          </span>
                          {req.type === 'insight' && req.training_id ? (
                            <Link
                              to={`/trainings/${req.training_id}`}
                              className="text-sm font-medium text-gray-800 hover:text-purple-600"
                            >
                              {req.title}
                            </Link>
                          ) : req.type === 'chat' ? (
                            <Link
                              to={`/chats/${req.id}`}
                              className="text-sm font-medium text-gray-800 hover:text-purple-600"
                            >
                              {req.title}
                            </Link>
                          ) : (
                            <span className="text-sm font-medium text-gray-800">
                              {req.title}
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-3 mt-1 text-xs text-gray-500">
                          {req.model && (
                            <span className="font-mono">{req.model.split('/').pop()}</span>
                          )}
                          <span>
                            {new Date(req.created_at).toLocaleDateString(undefined, {
                              month: 'short',
                              day: 'numeric',
                              year: 'numeric',
                              hour: '2-digit',
                              minute: '2-digit',
                            })}
                          </span>
                        </div>
                      </div>
                      <div className="text-sm font-semibold text-gray-700 ml-4">
                        {formatCost(req.cost)}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
