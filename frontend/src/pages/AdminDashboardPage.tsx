import { useEffect, useState } from 'react';
import { getAdminStats, type AdminStats } from '../api/admin';
import Navbar from '../components/Navbar';

export default function AdminDashboardPage() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAdminStats()
      .then(setStats)
      .catch((e) => setError(e.message));
  }, []);

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-4xl mx-auto px-4 py-8">
        <h1 className="text-2xl font-bold mb-6">Admin Dashboard</h1>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
            {error === 'Unauthorized' ? 'You must be logged in.' : `Access denied: ${error}`}
          </div>
        )}

        {stats && (
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold mb-4">Platform Stats</h3>
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Registered users</span>
              <span className="text-sm font-medium">{stats.user_count}</span>
            </div>
          </div>
        )}

        {!stats && !error && (
          <p className="text-gray-500">Loading...</p>
        )}
      </div>
    </div>
  );
}
