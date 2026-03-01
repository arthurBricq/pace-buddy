import { useEffect, useState } from 'react';
import {
  getAdminStats,
  getQuotaRequests,
  approveQuotaRequest,
  rejectQuotaRequest,
  deleteAllData,
  type AdminStats,
} from '../api/admin';
import type { QuotaRequestRecord } from '../types';
import Navbar from '../components/Navbar';

export default function AdminDashboardPage() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [requests, setRequests] = useState<QuotaRequestRecord[]>([]);
  const [amounts, setAmounts] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const loadData = () => {
    setNotice(null);
    getAdminStats().then(setStats).catch((e) => setError(e.message));
    getQuotaRequests().then(setRequests).catch(() => {});
  };

  useEffect(loadData, []);

  const handleApprove = async (id: string) => {
    const val = parseFloat(amounts[id] || '');
    if (isNaN(val) || val <= 0) return;
    try {
      await approveQuotaRequest(id, val);
      loadData();
    } catch (e: any) {
      setError(e.message);
    }
  };

  const handleReject = async (id: string) => {
    try {
      await rejectQuotaRequest(id);
      loadData();
    } catch (e: any) {
      setError(e.message);
    }
  };

  const handleDeleteAllData = async () => {
    const confirmation = window.prompt(
      'This will permanently delete all database data. Type DELETE ALL to confirm.'
    );
    if (confirmation !== 'DELETE ALL') {
      return;
    }

    try {
      setIsDeleting(true);
      await deleteAllData();
      setAmounts({});
      loadData();
      setNotice('All database data has been deleted.');
    } catch (e: any) {
      setError(e.message);
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="max-w-4xl mx-auto px-4 py-8 space-y-6">
        <h1 className="text-2xl font-bold">Admin Dashboard</h1>

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
            {error === 'Unauthorized' ? 'You must be logged in.' : `Access denied: ${error}`}
          </div>
        )}
        {notice && (
          <div className="bg-green-50 border border-green-200 rounded-lg p-4 text-green-700">
            {notice}
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

        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold mb-4">Pending Quota Requests</h3>
          {requests.length === 0 ? (
            <p className="text-sm text-gray-500">No pending requests.</p>
          ) : (
            <div className="space-y-3">
              {requests.map((req) => (
                <div key={req.id} className="flex items-center justify-between p-3 bg-gray-50 rounded-md">
                  <div>
                    <p className="text-sm font-medium text-gray-800">
                      User: <span className="font-mono text-xs">{req.user_id.slice(0, 8)}...</span>
                    </p>
                    <p className="text-xs text-gray-500">
                      Requested {new Date(req.requested_at).toLocaleDateString(undefined, {
                        month: 'short', day: 'numeric', year: 'numeric',
                        hour: '2-digit', minute: '2-digit',
                      })}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-gray-500">$</span>
                    <input
                      type="number"
                      step="0.5"
                      min="0"
                      placeholder="Amount"
                      value={amounts[req.id] || ''}
                      onChange={(e) => setAmounts({ ...amounts, [req.id]: e.target.value })}
                      className="w-20 px-2 py-1 text-sm border rounded"
                    />
                    <button
                      onClick={() => handleApprove(req.id)}
                      className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700"
                    >
                      Approve
                    </button>
                    <button
                      onClick={() => handleReject(req.id)}
                      className="px-3 py-1 text-sm bg-red-600 text-white rounded hover:bg-red-700"
                    >
                      Reject
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="bg-white rounded-lg shadow p-6 border border-red-200">
          <h3 className="text-lg font-semibold text-red-700 mb-2">Danger Zone</h3>
          <p className="text-sm text-gray-600 mb-4">
            Delete all data in the database (users, activities, trainings, chats, quota requests).
            This is intended for development only.
          </p>
          <button
            onClick={handleDeleteAllData}
            disabled={isDeleting}
            className="px-4 py-2 text-sm bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-60"
          >
            {isDeleting ? 'Deleting...' : 'Delete All Database Data'}
          </button>
        </div>

        {!stats && !error && (
          <p className="text-gray-500">Loading...</p>
        )}
      </div>
    </div>
  );
}
