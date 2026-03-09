import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getAdminUsersByQuotaSpent, type AdminUserQuotaSpending } from '../api/admin';
import Navbar from '../components/Navbar';

function usd(value: number): string {
  return `$${value.toFixed(2)}`;
}

export default function AdminUsersPage() {
  const [users, setUsers] = useState<AdminUserQuotaSpending[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAdminUsersByQuotaSpent()
      .then(setUsers)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide section-stack">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl font-bold">Admin Users</h1>
            <p className="text-sm text-gray-500 mt-1">
              Sorted by total quota spent (highest first)
            </p>
          </div>
          <Link to="/admin" className="text-sm text-blue-600 hover:underline">
            Back to dashboard
          </Link>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
            {error === 'Unauthorized' ? 'You must be logged in.' : `Access denied: ${error}`}
          </div>
        )}

        {loading ? (
          <p className="text-gray-500">Loading...</p>
        ) : (
          <div className="data-table-wrap">
            <table className="data-table-wide">
              <thead className="bg-gray-50 text-gray-600">
                <tr>
                  <th className="text-left px-4 py-3">#</th>
                  <th className="text-left px-4 py-3">Username</th>
                  <th className="text-left px-4 py-3">Display Name</th>
                  <th className="text-left px-4 py-3">Email</th>
                  <th className="text-right px-4 py-3">Spent</th>
                  <th className="text-right px-4 py-3">Granted</th>
                  <th className="text-right px-4 py-3">Balance</th>
                  <th className="text-left px-4 py-3">Created</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {users.map((u, index) => (
                  <tr key={u.user_id} className="hover:bg-gray-50">
                    <td className="px-4 py-3">{index + 1}</td>
                    <td className="px-4 py-3 font-medium">{u.username}</td>
                    <td className="px-4 py-3">{u.display_name}</td>
                    <td className="px-4 py-3 text-gray-600">{u.email || '-'}</td>
                    <td className="px-4 py-3 text-right font-semibold text-red-700">
                      {usd(u.total_spent_usd)}
                    </td>
                    <td className="px-4 py-3 text-right">{usd(u.total_granted_usd)}</td>
                    <td className="px-4 py-3 text-right">{usd(u.quota_balance_usd)}</td>
                    <td className="px-4 py-3 text-gray-500">
                      {new Date(u.created_at).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
