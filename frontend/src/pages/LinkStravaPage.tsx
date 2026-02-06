import { useEffect, useState } from 'react';
import { getStravaStatus, getStravaLink } from '../api/strava';
import type { StravaStatus } from '../types';
import Navbar from '../components/Navbar';

export default function LinkStravaPage() {
  const [status, setStatus] = useState<StravaStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    getStravaStatus()
      .then(setStatus)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  const handleLink = async () => {
    try {
      const { url } = await getStravaLink();
      window.location.href = url;
    } catch (err: any) {
      setError(err.message);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-lg mx-auto mt-12 bg-white rounded-lg shadow p-8">
        <h1 className="text-xl font-bold mb-4">Strava Connection</h1>

        {loading && <p className="text-gray-500">Checking status...</p>}
        {error && <p className="text-red-600 text-sm">{error}</p>}

        {status && status.linked ? (
          <div className="space-y-2">
            <p className="text-green-600 font-medium">Strava connected</p>
            <p className="text-sm text-gray-500">
              Athlete ID: {status.athlete_id}
            </p>
          </div>
        ) : status && !status.linked ? (
          <div className="space-y-4">
            <p className="text-gray-600">
              Connect your Strava account to sync activities.
            </p>
            <button
              onClick={handleLink}
              className="bg-orange-500 text-white px-4 py-2 rounded-md hover:bg-orange-600"
            >
              Connect Strava
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
