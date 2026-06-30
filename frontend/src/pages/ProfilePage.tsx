import { useEffect, useState } from 'react';
import { useSearchParams, Link } from 'react-router-dom';
import { getProfile, getQuotaStatus, requestQuota } from '../api/auth';
import { syncActivities } from '../api/activities';
import { getStravaStatus, getStravaLink, disconnectStrava } from '../api/strava';
import type { ProfileResponse, QuotaStatus, RunningStats, StravaStatus } from '../types';
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

function formatGoalTargetTime(seconds: number | null): string {
  if (!seconds || seconds <= 0) return '-';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s
    .toString()
    .padStart(2, '0')}`;
}

function formatGoalSportType(value: string | null): string {
  if (!value) return '-';
  if (value === 'trail_running') return 'Trail running';
  if (value === 'running') return 'Running';
  return value;
}

function formatQuotaAmount(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

function errorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return fallback;
}

export default function ProfilePage() {
  const [searchParams] = useSearchParams();
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [stravaStatus, setStravaStatus] = useState<StravaStatus | null>(null);
  const [quotaStatus, setQuotaStatus] = useState<QuotaStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(searchParams.get('error') || '');
  const [resyncing, setResyncing] = useState(false);
  const [resyncNotice, setResyncNotice] = useState('');
  const [quotaRequesting, setQuotaRequesting] = useState(false);
  const [quotaNotice, setQuotaNotice] = useState('');
  const [quotaError, setQuotaError] = useState('');

  useEffect(() => {
    Promise.all([
      getProfile().catch((err: unknown) => {
        setError(errorMessage(err, 'Failed to load profile'));
        return null;
      }),
      getStravaStatus().catch(() => null),
      getQuotaStatus().catch(() => null),
    ]).then(([p, s, q]) => {
      setProfile(p);
      setStravaStatus(s);
      setQuotaStatus(q);
    }).finally(() => setLoading(false));
  }, []);

  const handleLinkStrava = async () => {
    try {
      const { url } = await getStravaLink();
      window.location.href = url;
    } catch (err) {
      setError(errorMessage(err, 'Failed to start Strava linking'));
    }
  };

  const handleDisconnectStrava = async () => {
    if (!confirm('Disconnect Strava? This will delete all synced activities and streams.')) return;
    try {
      await disconnectStrava();
      setStravaStatus({ linked: false });
    } catch (err) {
      setError(errorMessage(err, 'Failed to disconnect Strava'));
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
    } catch (err) {
      setError(errorMessage(err, 'Failed to request Strava resync'));
    } finally {
      setResyncing(false);
    }
  };

  const handleRequestQuota = async () => {
    setQuotaError('');
    setQuotaNotice('');
    setQuotaRequesting(true);
    try {
      const request = await requestQuota();
      setQuotaStatus((current) => current
        ? {
            ...current,
            has_pending_request: true,
            requests: [request, ...current.requests],
          }
        : current
      );
      setQuotaNotice('Quota request sent. An admin can now review it.');
    } catch (err) {
      setQuotaError(errorMessage(err, 'Failed to request more AI quota'));
    } finally {
      setQuotaRequesting(false);
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

  const { user, stats, identity_profile: identityProfile, athlete_profile: athleteProfile } = profile;

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
          <div className="mt-4">
            <Link
              to="/runner-profile?returnTo=/profile"
              className="text-sm text-purple-700 hover:text-purple-900 underline"
            >
              Edit Runner Profile
            </Link>
          </div>
        </div>

        <div className="grid gap-6 sm:grid-cols-2">
          <div className="card">
            <h2 className="text-lg font-semibold mb-4">About You</h2>
            {identityProfile ? (
              <div className="space-y-3">
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Name</span>
                  <span className="text-sm font-medium text-right">{identityProfile.name || '-'}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Age</span>
                  <span className="text-sm font-medium text-right">
                    {identityProfile.age != null ? identityProfile.age : '-'}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Email</span>
                  <span className="text-sm font-medium text-right">{identityProfile.email || '-'}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Gender</span>
                  <span className="text-sm font-medium text-right">{identityProfile.gender || '-'}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Height</span>
                  <span className="text-sm font-medium text-right">
                    {identityProfile.height_cm != null ? `${identityProfile.height_cm} cm` : '-'}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Weight</span>
                  <span className="text-sm font-medium text-right">
                    {identityProfile.weight_kg != null ? `${identityProfile.weight_kg} kg` : '-'}
                  </span>
                </div>
              </div>
            ) : (
              <p className="text-sm text-gray-500">Not configured yet.</p>
            )}
          </div>

          <div className="card">
            <h2 className="text-lg font-semibold mb-4">Running Goals</h2>
            {athleteProfile ? (
              <div className="space-y-3">
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Goal date</span>
                  <span className="text-sm font-medium text-right">{athleteProfile.goal_date || '-'}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Goal distance</span>
                  <span className="text-sm font-medium text-right">
                    {athleteProfile.goal_distance_km != null ? `${athleteProfile.goal_distance_km} km` : '-'}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Goal target time</span>
                  <span className="text-sm font-medium text-right">
                    {formatGoalTargetTime(athleteProfile.goal_target_time_seconds)}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Goal sport</span>
                  <span className="text-sm font-medium text-right">
                    {formatGoalSportType(athleteProfile.goal_sport_type)}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-sm text-gray-500">Goal elevation</span>
                  <span className="text-sm font-medium text-right">
                    {athleteProfile.goal_elevation_gain_m != null
                      ? `${Math.round(athleteProfile.goal_elevation_gain_m)} m`
                      : '-'}
                  </span>
                </div>
                <div>
                  <p className="text-sm text-gray-500">Goal description</p>
                  <p className="text-sm font-medium mt-1 whitespace-pre-wrap">
                    {athleteProfile.goal_description || '-'}
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-500">Additional information</p>
                  <p className="text-sm font-medium mt-1 whitespace-pre-wrap">
                    {athleteProfile.additional_info || '-'}
                  </p>
                </div>
              </div>
            ) : (
              <p className="text-sm text-gray-500">Not configured yet.</p>
            )}
          </div>
        </div>

        <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          <StatsCard title="Year to Date" stats={stats.ytd} />
          <StatsCard title="Last Year" stats={stats.last_year} />
          <StatsCard title="All Time" stats={stats.all_time} />
        </div>

        <div className="card">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-lg font-semibold">AI quota</h2>
              {quotaStatus ? (
                <p className="text-sm text-gray-500 mt-1">
                  Remaining budget:{' '}
                  <span className="font-semibold text-green-700">
                    {formatQuotaAmount(quotaStatus.balance_usd)}
                  </span>
                </p>
              ) : (
                <p className="text-sm text-gray-500 mt-1">Quota status unavailable.</p>
              )}
            </div>

            {quotaStatus && (
              quotaStatus.has_pending_request ? (
                <span className="text-sm text-amber-600 font-medium">Request pending...</span>
              ) : (
                <button
                  onClick={handleRequestQuota}
                  disabled={quotaRequesting}
                  className="text-sm bg-purple-600 text-white px-3 py-1.5 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {quotaRequesting ? 'Requesting...' : 'Request More Tokens'}
                </button>
              )
            )}
          </div>
          {quotaNotice && <p className="text-green-700 text-sm mt-3">{quotaNotice}</p>}
          {quotaError && <p className="text-red-600 text-sm mt-3">{quotaError}</p>}
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

      </div>
    </div>
  );
}
