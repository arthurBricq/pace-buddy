import { useEffect, useState } from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import { getProfile, getAiCostSummary, getQuotaStatus, requestQuota } from '../api/auth';
import { syncActivities } from '../api/activities';
import { getStravaStatus, getStravaLink, disconnectStrava } from '../api/strava';
import type { ProfileResponse, RunningStats, StravaStatus, AiCostSummary, QuotaStatus } from '../types';
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
    <div className="card">
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
  const [quotaStatus, setQuotaStatus] = useState<QuotaStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(searchParams.get('error') || '');
  const [resyncing, setResyncing] = useState(false);
  const [resyncNotice, setResyncNotice] = useState('');

  useEffect(() => {
    Promise.all([
      getProfile().catch((e) => {
        setError(e.message);
        return null;
      }),
      getStravaStatus().catch(() => null),
      getAiCostSummary().catch(() => null),
      getQuotaStatus().catch(() => null),
    ]).then(([p, s, a, q]) => {
      setProfile(p);
      setStravaStatus(s);
      setAiCostSummary(a);
      setQuotaStatus(q);
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

  const handleRequestResync = async () => {
    setError('');
    setResyncNotice('');
    setResyncing(true);
    try {
      const result = await syncActivities();
      if (result.already_running) {
        setResyncNotice('A Strava sync is already running in the background.');
      } else {
        setResyncNotice(`Strava resync requested. ${result.synced} activity(ies) synchronized.`);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to request Strava resync');
    } finally {
      setResyncing(false);
    }
  };

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-wide">
          <p className="text-gray-500">Loading profile...</p>
        </div>
      </div>
    );
  }

  if (error || !profile) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-wide">
          <p className="text-red-600">{error || 'Failed to load profile'}</p>
        </div>
      </div>
    );
  }

  const { user, stats } = profile;

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide section-stack">
        <div className="card">
          <h1 className="text-2xl font-bold">{user.display_name}</h1>
          <p className="text-sm text-gray-500 mt-1">@{user.username}</p>
          <p className="text-sm text-gray-400 mt-1">
            Member since {new Date(user.created_at).toLocaleDateString()}
          </p>
        </div>

        <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          <StatsCard title="Year to Date" stats={stats.ytd} />
          <StatsCard title="Last Year" stats={stats.last_year} />
          <StatsCard title="All Time" stats={stats.all_time} />
        </div>

        {/* Strava Connection Section */}
        <div className="card">
          {error && <p className="text-red-600 text-sm mb-2">{error}</p>}
          {resyncNotice && <p className="text-green-700 text-sm mb-2">{resyncNotice}</p>}
          {stravaStatus && stravaStatus.linked ? (
            <div className="space-y-3">
              <img src="/strava_pwrdby_horiz_orange.svg" alt="Powered by Strava" className="h-8" />
              <div className="flex flex-wrap items-center gap-3">
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
                <button
                  onClick={handleRequestResync}
                  disabled={resyncing}
                  title="Use this if your latest Strava activities are missing or outdated. It manually asks the backend to pull updates from Strava."
                  className="text-sm bg-blue-600 text-white px-3 py-1.5 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {resyncing ? 'Requesting resync...' : 'Request Strava Resync'}
                </button>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <p className="text-gray-600">
                Connect your Strava account to sync activities.
              </p>
              <button onClick={handleLinkStrava} className="hover:opacity-80 transition-opacity">
                <img src="/btn_strava_connect_with_orange.svg" alt="Connect with Strava" className="h-12" />
              </button>
            </div>
          )}
        </div>

        {/* AI Cost Summary Section */}
        {aiCostSummary && (
          <div className="card">
            <h2 className="text-lg font-semibold mb-4">AI Usage</h2>

            {/* Quota */}
            {quotaStatus && (
              <div className="mb-6 p-4 bg-gray-50 rounded-lg">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <span className="text-sm text-gray-500">AI Budget Remaining:</span>
                    <span className="ml-2 text-lg font-bold text-green-600">
                      ${quotaStatus.balance_usd.toFixed(2)}
                    </span>
                  </div>
                  {quotaStatus.has_pending_request ? (
                    <span className="text-sm text-amber-600 font-medium">Request pending...</span>
                  ) : (
                    <button
                      onClick={async () => {
                        try {
                          await requestQuota();
                          setQuotaStatus({ ...quotaStatus, has_pending_request: true });
                        } catch (err: any) {
                          setError(err.message);
                        }
                      }}
                      className="px-3 py-1.5 text-sm bg-purple-600 text-white rounded-md hover:bg-purple-700"
                    >
                      Request More Tokens
                    </button>
                  )}
                </div>
              </div>
            )}

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
                      className="flex flex-col gap-2 p-3 bg-gray-50 rounded-md hover:bg-gray-100 transition-colors sm:flex-row sm:items-center sm:justify-between"
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
                      <div className="text-sm font-semibold text-gray-700 sm:ml-4">
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
