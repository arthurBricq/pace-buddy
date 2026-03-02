import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listActivities, updateActivityTag } from '../api/activities';
import { getMAS, updateMAS } from '../api/auth';
import type { Activity, MASEstimate } from '../types';
import { calculateMAS, masToKmh } from '../utils/mas';
import Navbar from '../components/Navbar';
import MASChart from '../components/MASChart';

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h${m.toString().padStart(2, '0')}m${s.toString().padStart(2, '0')}s`;
  return `${m}m${s.toString().padStart(2, '0')}s`;
}

function formatDistance(meters: number): string {
  return (meters / 1000).toFixed(2) + ' km';
}

export default function RacesPage() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const [allActivities, setAllActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showAddForm, setShowAddForm] = useState(false);
  const [currentMAS, setCurrentMAS] = useState<number | null>(null);
  const [showManualOverride, setShowManualOverride] = useState(false);
  const [manualMAS, setManualMAS] = useState('');
  const [updatingMAS, setUpdatingMAS] = useState(false);

  const loadRaces = async () => {
    setLoading(true);
    try {
      // Fetch activities and filter for races
      const fetched: Activity[] = [];
      let offset = 0;
      const limit = 100;

      // Fetch up to 500 activities to find all races
      for (let i = 0; i < 5; i++) {
        const page = await listActivities(limit, offset);
        if (page.length === 0) break;
        fetched.push(...page);
        offset += limit;
        if (page.length < limit) break;
      }

      setAllActivities(fetched);
      const races = fetched.filter((a) => a.tag === 'race');
      setActivities(races);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadRaces();
    loadCurrentMAS();
  }, []);

  const loadCurrentMAS = async () => {
    try {
      const response = await getMAS();
      setCurrentMAS(response.mas_mps);
    } catch (err: any) {
      console.error('Failed to load current MAS:', err);
    }
  };

  const handleRemoveRace = async (activityId: string) => {
    if (!confirm('Remove this activity from races? It will be untagged as a race.')) {
      return;
    }

    try {
      await updateActivityTag(activityId, 'normal');
      await loadRaces();
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleAddRace = async (activityId: string) => {
    try {
      await updateActivityTag(activityId, 'race');
      await loadRaces();
      setShowAddForm(false);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleRecomputeMAS = async () => {
    if (activities.length === 0) {
      setError('No races available to compute MAS');
      return;
    }

    setUpdatingMAS(true);
    try {
      // Get the latest race (most recent by date)
      const latestRace = [...activities].sort(
        (a, b) => new Date(b.start_date).getTime() - new Date(a.start_date).getTime(),
      )[0];

      const mas_ms = calculateMAS(latestRace.distance, latestRace.moving_time);
      await updateMAS(mas_ms);
      setCurrentMAS(mas_ms);
      setError('');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setUpdatingMAS(false);
    }
  };

  const handleManualOverride = async () => {
    const masValue = parseFloat(manualMAS);
    if (isNaN(masValue) || masValue <= 0) {
      setError('Please enter a valid MAS value (m/s)');
      return;
    }

    setUpdatingMAS(true);
    try {
      await updateMAS(masValue);
      setCurrentMAS(masValue);
      setManualMAS('');
      setShowManualOverride(false);
      setError('');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setUpdatingMAS(false);
    }
  };

  // Calculate MAS estimates
  const masEstimates: MASEstimate[] = activities
    .map((activity) => {
      const mas_ms = calculateMAS(activity.distance, activity.moving_time);
      const mas_kmh = masToKmh(mas_ms);
      return {
        date: activity.start_date,
        mas_ms,
        mas_kmh,
        activity_id: activity.id,
        activity_name: activity.name,
        distance_m: activity.distance,
        time_s: activity.moving_time,
      };
    })
    .sort((a, b) => new Date(a.date).getTime() - new Date(b.date).getTime());

  // Get available activities to add (not already tagged as race)
  const availableActivities = allActivities.filter(
    (a) => a.tag !== 'race',
  ).sort(
    (a, b) => new Date(b.start_date).getTime() - new Date(a.start_date).getTime(),
  );

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-wide">
          <p className="text-gray-500">Loading races...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide section-stack">
        <div>
          <h1 className="text-2xl font-bold">Races (estimators)</h1>
          <p className="text-sm text-gray-500 mt-1">
            Activities tagged as races used to estimate Maximum Aerobic Speed (MAS)
          </p>
        </div>

        {error && <p className="text-red-600 text-sm">{error}</p>}

        {/* Current MAS Display */}
        <div className="card">
          <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-lg font-semibold">Current MAS Estimate</h2>
              <p className="text-sm text-gray-500 mt-1">
                Maximum Aerobic Speed used for performance analysis
              </p>
            </div>
            <div className="button-row-wrap">
              <button
                onClick={handleRecomputeMAS}
                disabled={updatingMAS || activities.length === 0}
                className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
              >
                {updatingMAS ? 'Updating...' : 'Recompute'}
              </button>
              {!showManualOverride ? (
                <button
                  onClick={() => setShowManualOverride(true)}
                  className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
                >
                  Manual Override
                </button>
              ) : (
                <button
                  onClick={() => {
                    setShowManualOverride(false);
                    setManualMAS('');
                  }}
                  className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
                >
                  Cancel
                </button>
              )}
            </div>
          </div>

          {showManualOverride ? (
            <div className="border-t pt-4">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Enter MAS value (m/s)
              </label>
              <div className="flex gap-2">
                <input
                  type="number"
                  step="0.01"
                  value={manualMAS}
                  onChange={(e) => setManualMAS(e.target.value)}
                  placeholder="e.g., 4.5"
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button
                  onClick={handleManualOverride}
                  disabled={updatingMAS}
                  className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                >
                  {updatingMAS ? 'Saving...' : 'Save'}
                </button>
              </div>
            </div>
          ) : (
            <div className="border-t pt-4">
              {currentMAS === null ? (
                <p className="text-gray-500 text-sm">
                  No MAS estimate set. Click "Recompute" to calculate from your latest race.
                </p>
              ) : (
                <div className="flex flex-col items-start gap-2 sm:flex-row sm:items-baseline sm:gap-4">
                  <div>
                    <span className="text-sm text-gray-600">MAS:</span>
                    <span className="text-2xl font-bold text-blue-600 ml-2">
                      {masToKmh(currentMAS).toFixed(2)} km/h
                    </span>
                  </div>
                  <div>
                    <span className="text-sm text-gray-500">
                      ({currentMAS.toFixed(2)} m/s)
                    </span>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* MAS Chart */}
        {masEstimates.length > 0 && (
          <MASChart estimates={masEstimates} />
        )}

        {/* Activities Section */}
        <div>
          <div className="mb-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <h2 className="text-lg font-semibold">
              Race Activities ({activities.length})
            </h2>
            {!showAddForm && (
              <button
                onClick={() => setShowAddForm(true)}
                className="text-sm bg-blue-600 text-white px-3 py-1 rounded-md hover:bg-blue-700"
              >
                Add Race Activity
              </button>
            )}
          </div>

          {showAddForm && (
            <div className="card-compact mb-4">
              <h3 className="text-md font-medium mb-3">Add Activity as Race</h3>
              {availableActivities.length === 0 ? (
                <p className="text-gray-500 text-sm">
                  No activities available to add. All activities are already tagged as races.
                </p>
              ) : (
                <>
                  <select
                    onChange={(e) => {
                      if (e.target.value) {
                        handleAddRace(e.target.value);
                      }
                    }}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 mb-3"
                    defaultValue=""
                  >
                    <option value="">Select an activity to tag as race...</option>
                    {availableActivities.map((a) => (
                      <option key={a.id} value={a.id}>
                        {new Date(a.start_date).toLocaleDateString()} - {a.name} ({formatDistance(a.distance)})
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => setShowAddForm(false)}
                    className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
                  >
                    Cancel
                  </button>
                </>
              )}
            </div>
          )}

          {activities.length === 0 ? (
            <p className="text-gray-500">
              No race activities yet. Tag activities as "race" to use them for MAS estimation.
            </p>
          ) : (
            <div className="data-table-wrap">
              <table className="data-table-wide">
                <thead className="bg-gray-50 text-gray-600">
                  <tr>
                    <th className="text-left px-4 py-3">Date</th>
                    <th className="text-left px-4 py-3">Name</th>
                    <th className="text-left px-4 py-3">Distance</th>
                    <th className="text-right px-4 py-3">Time</th>
                    <th className="text-right px-4 py-3">MAS (km/h)</th>
                    <th className="text-right px-4 py-3">MAS (m/s)</th>
                    <th className="text-right px-4 py-3">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {activities
                    .sort((a, b) => new Date(b.start_date).getTime() - new Date(a.start_date).getTime())
                    .map((a) => {
                      const mas_ms = calculateMAS(a.distance, a.moving_time);
                      const mas_kmh = masToKmh(mas_ms);
                      return (
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
                          <td className="px-4 py-3">{formatDistance(a.distance)}</td>
                          <td className="px-4 py-3 text-right">
                            {formatDuration(a.moving_time)}
                          </td>
                          <td className="px-4 py-3 text-right font-medium text-blue-600">
                            {mas_kmh.toFixed(2)}
                          </td>
                          <td className="px-4 py-3 text-right text-gray-600">
                            {mas_ms.toFixed(2)}
                          </td>
                          <td className="px-4 py-3 text-right">
                            <button
                              onClick={() => handleRemoveRace(a.id)}
                              className="text-red-600 hover:text-red-800 text-sm"
                            >
                              Remove
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
