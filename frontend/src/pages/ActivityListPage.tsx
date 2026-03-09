import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { getActivitiesSyncStatus, listActivities, updateActivityTag } from '../api/activities';
import { getStravaStatus } from '../api/strava';
import type { Activity, ActivityTag } from '../types';
import TagBadge from '../components/TagBadge';
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

function pad2(value: number): string {
  return value.toString().padStart(2, '0');
}

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
}

function startOfWeek(date: Date): Date {
  const copy = new Date(date);
  copy.setHours(0, 0, 0, 0);
  const day = (copy.getDay() + 6) % 7;
  copy.setDate(copy.getDate() - day);
  return copy;
}

function addDays(date: Date, days: number): Date {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + days);
  return copy;
}

const TAG_OPTIONS: ActivityTag[] = ['normal', 'intervals', 'long_run', 'race'];

function formatTagLabel(tag: ActivityTag): string {
  return tag.replace('_', ' ');
}

export default function ActivityListPage() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [stravaLinked, setStravaLinked] = useState(false);
  const [syncStatus, setSyncStatus] = useState<'idle' | 'running' | 'finished' | 'failed' | null>(null);
  const [syncStatusError, setSyncStatusError] = useState('');
  const [syncStatusHandled, setSyncStatusHandled] = useState(false);
  const [error, setError] = useState('');
  const [editingTag, setEditingTag] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [tagFilter, setTagFilter] = useState<ActivityTag | 'all'>('all');
  const [qualityOnly, setQualityOnly] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'calendar'>('list');
  const limit = 50;
  const editingActivity = editingTag ? activities.find((a) => a.id === editingTag) ?? null : null;

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

  const filteredActivities = activities
    .filter((a) => (tagFilter === 'all' ? true : a.tag === tagFilter))
    .filter((a) => (qualityOnly ? a.tag === 'intervals' || a.tag === 'long_run' || a.tag === 'race' : true));

  const qualityTooltip = 'Quality sessions are intervals, long runs, and races.';
  const showSyncBanner =
    !loading &&
    activities.length === 0 &&
    stravaLinked &&
    !syncStatusHandled &&
    (syncStatus === null || syncStatus === 'running');

  const getHighlightClasses = (tag: ActivityTag) => {
    if (tag === 'intervals') {
      return 'activity-highlight-intervals';
    }
    if (tag === 'long_run') {
      return 'activity-highlight-long_run';
    }
    if (tag === 'race') {
      return 'activity-highlight-race';
    }
    return '';
  };

  const calendarWeeks = useMemo(() => {
    if (filteredActivities.length === 0) return [];
    const sorted = [...filteredActivities].sort(
      (a, b) => new Date(b.start_date).getTime() - new Date(a.start_date).getTime(),
    );
    const byDay = new Map<string, Activity[]>();
    sorted.forEach((activity) => {
      const day = new Date(activity.start_date);
      const key = dateKey(day);
      const existing = byDay.get(key);
      if (existing) {
        existing.push(activity);
      } else {
        byDay.set(key, [activity]);
      }
    });

    const weekKeys = new Map<string, Date>();
    sorted.forEach((activity) => {
      const weekStart = startOfWeek(new Date(activity.start_date));
      weekKeys.set(dateKey(weekStart), weekStart);
    });

    const weekStarts = Array.from(weekKeys.values()).sort(
      (a, b) => b.getTime() - a.getTime(),
    );

    return weekStarts.map((weekStart) => {
      const days = Array.from({ length: 7 }, (_, index) => {
        const date = addDays(weekStart, index);
        const key = dateKey(date);
        return {
          date,
          key,
          activities: byDay.get(key) ?? [],
        };
      });
      const weekActivities = days.flatMap((day) => day.activities);
      const totalDistance = weekActivities.reduce((sum, activity) => sum + activity.distance, 0);
      const totalTime = weekActivities.reduce((sum, activity) => sum + activity.moving_time, 0);
      return {
        weekStart,
        days,
        totalDistance,
        totalTime,
      };
    });
  }, [filteredActivities]);

  useEffect(() => {
    load();
    getStravaStatus()
      .then((status) => setStravaLinked(Boolean(status.linked)))
      .catch(() => setStravaLinked(false));
  }, []);

  useEffect(() => {
    if (loading || activities.length > 0 || !stravaLinked || syncStatusHandled) {
      return;
    }

    let stopped = false;
    let intervalId: number | null = null;

    const poll = async () => {
      try {
        const status = await getActivitiesSyncStatus();
        if (stopped) return;

        setSyncStatus(status.status);
        setSyncStatusError(status.error || '');

        if (status.status === 'finished') {
          setSyncStatusHandled(true);
          load();
          return;
        }

        if (status.status === 'failed') {
          setSyncStatusHandled(true);
        }
      } catch {
        // Ignore transient polling errors.
      }
    };

    poll();
    intervalId = window.setInterval(poll, 1500);

    return () => {
      stopped = true;
      if (intervalId !== null) {
        window.clearInterval(intervalId);
      }
    };
  }, [loading, activities.length, stravaLinked, syncStatusHandled]);

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

  const openTagEditor = (activityId: string) => {
    setEditingTag(activityId);
  };

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide">
        <div className="page-title-row">
          <div className="flex flex-wrap items-center gap-3 sm:gap-4">
            <h1 className="text-xl font-bold">Activities</h1>
            <select
              value={tagFilter}
              onChange={(e) => setTagFilter(e.target.value as ActivityTag | 'all')}
              className="theme-select w-full sm:w-auto text-sm"
            >
              <option value="all">All Activities</option>
              <option value="intervals">Intervals</option>
              <option value="long_run">Long run</option>
              <option value="race">Race</option>
              <option value="normal">Normal</option>
            </select>
          </div>
          <div className="button-row-wrap">
            <button
              onClick={() => setViewMode(viewMode === 'list' ? 'calendar' : 'list')}
              className={`theme-toggle-btn sm:w-[18rem] ${
                viewMode === 'calendar'
                  ? 'theme-toggle-btn-active'
                  : ''
              }`}
            >
              {viewMode === 'list' ? 'Switch to calendar view' : 'Switch to list view'}
            </button>
            <button
              type="button"
              title={qualityTooltip}
              onClick={() => setQualityOnly((prev) => !prev)}
              className={`theme-toggle-btn sm:w-[18rem] ${
                qualityOnly
                  ? 'theme-toggle-btn-active'
                  : ''
              }`}
            >
              {qualityOnly ? 'Showing quality sessions only' : 'Filter only quality sessions'}
            </button>
          </div>
        </div>

        {error && <p className="theme-error-text mb-4">{error}</p>}

        {showSyncBanner && (
          <div className="theme-notice theme-notice-info mb-4 flex items-center gap-2">
            <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
            Importing activities from Strava...
          </div>
        )}

        {!loading && activities.length === 0 && syncStatus === 'failed' && syncStatusError && (
          <div className="theme-notice theme-notice-error mb-4">
            Automatic Strava import failed: {syncStatusError}
          </div>
        )}

        {loading ? (
          <p className="theme-muted">Loading activities...</p>
        ) : activities.length === 0 ? (
          <p className="theme-muted">
            No activities yet. If Strava is linked, imports happen automatically. You can request
            a manual resync from your profile.
          </p>
        ) : viewMode === 'calendar' ? (
          calendarWeeks.length === 0 ? (
            <p className="theme-muted">No activities for this filter.</p>
          ) : (
            <div className="space-y-4">
              {calendarWeeks.map((week) => (
                <div
                  key={dateKey(week.weekStart)}
                  className="calendar-week-card"
                >
                  <div className="calendar-week-grid grid gap-3 items-stretch">
                  <div className="calendar-recap-card flex flex-col justify-between">
                    <div>
                      <p className="text-xs theme-muted uppercase tracking-wide">Weekly recap</p>
                      <p className="text-sm font-semibold mt-1">
                        {week.weekStart.toLocaleDateString(undefined, {
                          month: 'short',
                          day: 'numeric',
                        })}
                      </p>
                    </div>
                    <div className="mt-3">
                      <p className="text-lg font-semibold">
                        {formatDistance(week.totalDistance)}
                      </p>
                      <p className="text-sm theme-muted">
                        {formatDuration(week.totalTime)}
                      </p>
                    </div>
                  </div>
                  {week.days.map((day) => (
                    <div
                      key={day.key}
                      className="calendar-day-card"
                    >
                      <div className="text-xs theme-muted">
                        {day.date.toLocaleDateString(undefined, {
                          weekday: 'short',
                          month: 'short',
                          day: 'numeric',
                        })}
                      </div>
                      <div className="mt-2 space-y-2">
                        {day.activities.length === 0 ? (
                          <p className="text-xs theme-muted">No activity</p>
                        ) : (
                          day.activities.map((activity) => (
                            <div key={activity.id} className="text-xs">
                              <div className="flex items-center gap-2">
                                <span className="font-semibold">
                                  {formatDistance(activity.distance)}
                                </span>
                                <TagBadge tag={activity.tag} />
                              </div>
                              <Link
                                to={`/activities/${activity.id}`}
                                className="theme-link"
                              >
                                {activity.name}
                              </Link>
                            </div>
                          ))
                        )}
                      </div>
                    </div>
                  ))}
                  </div>
                </div>
              ))}
            </div>
          )
        ) : (
          <>
            {filteredActivities.length === 0 ? (
              <p className="theme-muted">No activities for this filter.</p>
            ) : (
              <>
                <div className="space-y-3 sm:hidden">
                  {filteredActivities.map((a) => (
                    <article key={a.id} className={`card-compact ${getHighlightClasses(a.tag)}`}>
                      <div className="mb-2 flex items-start justify-between gap-2">
                        <p className="text-xs theme-muted">
                          {new Date(a.start_date).toLocaleDateString()}
                        </p>
                        <div>
                          <button
                            onClick={() => openTagEditor(a.id)}
                            className="tag-edit-trigger"
                            title="Edit activity type"
                          >
                            <TagBadge tag={a.tag} />
                          </button>
                        </div>
                      </div>
                      <Link
                        to={`/activities/${a.id}`}
                        className="mb-3 block text-base font-medium theme-link"
                      >
                        {a.name}
                      </Link>
                      <dl className="grid grid-cols-2 gap-x-3 gap-y-2 text-sm">
                        <div>
                          <dt className="text-xs uppercase tracking-wide theme-muted">Type</dt>
                          <dd>{a.sport_type}</dd>
                        </div>
                        <div>
                          <dt className="text-xs uppercase tracking-wide theme-muted">Distance</dt>
                          <dd className="font-medium">{formatDistance(a.distance)}</dd>
                        </div>
                        <div>
                          <dt className="text-xs uppercase tracking-wide theme-muted">Duration</dt>
                          <dd>{formatDuration(a.moving_time)}</dd>
                        </div>
                        <div>
                          <dt className="text-xs uppercase tracking-wide theme-muted">Pace</dt>
                          <dd>{formatPace(a.average_speed)}</dd>
                        </div>
                      </dl>
                    </article>
                  ))}
                </div>

                <div className="hidden sm:block data-table-wrap">
                  <table className="data-table">
                    <thead className="bg-[#fff6df] text-[#6b4f5d]">
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
                    <tbody className="divide-y divide-[#f0e3d7]">
                      {filteredActivities.map((a) => (
                        <tr key={a.id} className={`hover:bg-[#fff9ef] ${getHighlightClasses(a.tag)}`}>
                          <td className="px-4 py-3 theme-muted">
                            {new Date(a.start_date).toLocaleDateString()}
                          </td>
                          <td className="px-4 py-3">
                            <Link
                              to={`/activities/${a.id}`}
                              className="theme-link"
                            >
                              {a.name}
                            </Link>
                          </td>
                          <td className="px-4 py-3 theme-muted">{a.sport_type}</td>
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
                            <button
                              onClick={() => openTagEditor(a.id)}
                              className="tag-edit-trigger"
                              title="Edit activity type"
                            >
                              <TagBadge tag={a.tag} />
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}

            <div className="mt-4 flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-between">
              <div className="flex items-center justify-between gap-3 sm:contents">
                <button
                  onClick={() => load(Math.max(0, offset - limit))}
                  disabled={offset === 0}
                  className="theme-status-link"
                >
                  Previous
                </button>
                <button
                  onClick={() => load(offset + limit)}
                  disabled={activities.length < limit}
                  className="theme-status-link"
                >
                  Next
                </button>
              </div>
              <span className="text-sm theme-muted sm:order-none">
                Showing {filteredActivities.length} of {activities.length}
              </span>
            </div>
          </>
        )}
      </div>

      {editingActivity && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40"
          onClick={() => setEditingTag(null)}
        >
          <div
            className="card w-full max-w-sm"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="mb-3">
              <h2 className="text-base font-semibold">Edit activity type</h2>
              <p className="text-xs theme-muted mt-1 truncate">{editingActivity.name}</p>
            </div>
            <div className="grid grid-cols-2 gap-2">
              {TAG_OPTIONS.map((tag) => (
                <button
                  key={tag}
                  onClick={() => handleTagChange(editingActivity.id, tag)}
                  className={`rounded-md border px-3 py-2 text-sm font-medium capitalize transition-colors ${
                    tag === editingActivity.tag
                      ? 'bg-[#4c102a] text-white border-[#4c102a]'
                      : 'bg-white text-[#5b4451] border-[#dccab9] hover:border-[#761941]'
                  }`}
                >
                  {formatTagLabel(tag)}
                </button>
              ))}
            </div>
            <div className="mt-3 flex justify-end">
              <button
                onClick={() => setEditingTag(null)}
                className="theme-status-link"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
