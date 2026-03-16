import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import {
  getAdminStats,
  getQuotaRequests,
  approveQuotaRequest,
  rejectQuotaRequest,
  deleteAllData,
  listInviteCodes,
  createInviteCode,
  revokeInviteCode,
  type AdminStats,
  type AdminInviteCode,
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
  const [inviteCodes, setInviteCodes] = useState<AdminInviteCode[]>([]);
  const [inviteFor, setInviteFor] = useState('');
  const [inviteExpiresDays, setInviteExpiresDays] = useState('30');
  const [inviteCustomCode, setInviteCustomCode] = useState('');
  const [createdInviteCode, setCreatedInviteCode] = useState<string | null>(null);
  const [isCreatingInvite, setIsCreatingInvite] = useState(false);

  const loadData = () => {
    setNotice(null);
    getAdminStats().then(setStats).catch((e) => setError(e.message));
    getQuotaRequests().then(setRequests).catch(() => {});
    listInviteCodes().then(setInviteCodes).catch(() => {});
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

  const handleCreateInvite = async () => {
    const expiresInDays = parseInt(inviteExpiresDays, 10);
    const payload = {
      created_for: inviteFor.trim() || undefined,
      expires_in_days: Number.isNaN(expiresInDays) ? undefined : expiresInDays,
      code: inviteCustomCode.trim() || undefined,
    };

    try {
      setIsCreatingInvite(true);
      const result = await createInviteCode(payload);
      setCreatedInviteCode(result.code);
      setInviteFor('');
      setInviteCustomCode('');
      loadData();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setIsCreatingInvite(false);
    }
  };

  const handleRevokeInvite = async (id: string) => {
    try {
      await revokeInviteCode(id);
      loadData();
    } catch (e: any) {
      setError(e.message);
    }
  };

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-narrow section-stack">
        <div className="flex items-center justify-between gap-3">
          <h1 className="text-2xl font-bold">Admin Dashboard</h1>
          <Link to="/admin/users" className="text-sm text-blue-600 hover:underline">
            Users by quota spent
          </Link>
        </div>

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
          <div className="card">
            <h3 className="text-lg font-semibold mb-4">Platform Stats</h3>
            <div className="flex justify-between">
              <span className="text-sm text-gray-500">Registered users</span>
              <span className="text-sm font-medium">{stats.user_count}</span>
            </div>
          </div>
        )}

        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Pending Quota Requests</h3>
          {requests.length === 0 ? (
            <p className="text-sm text-gray-500">No pending requests.</p>
          ) : (
            <div className="space-y-3">
              {requests.map((req) => (
                <div key={req.id} className="flex flex-col gap-3 p-3 bg-gray-50 rounded-md sm:flex-row sm:items-center sm:justify-between">
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
                  <div className="button-row-wrap">
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

        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Invite Codes</h3>

          {createdInviteCode && (
            <div className="mb-4 rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-800">
              New invite code: <span className="font-mono font-semibold">{createdInviteCode}</span>
            </div>
          )}

          <div className="mb-4 grid gap-2 sm:grid-cols-4">
            <input
              type="text"
              placeholder="For (optional)"
              value={inviteFor}
              onChange={(e) => setInviteFor(e.target.value)}
              className="rounded border px-3 py-2 text-sm"
            />
            <input
              type="number"
              min="1"
              placeholder="Expires in days"
              value={inviteExpiresDays}
              onChange={(e) => setInviteExpiresDays(e.target.value)}
              className="rounded border px-3 py-2 text-sm"
            />
            <input
              type="text"
              placeholder="Custom code (optional)"
              value={inviteCustomCode}
              onChange={(e) => setInviteCustomCode(e.target.value)}
              className="rounded border px-3 py-2 text-sm"
            />
            <button
              onClick={handleCreateInvite}
              disabled={isCreatingInvite}
              className="rounded bg-blue-600 px-3 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-60"
            >
              {isCreatingInvite ? 'Creating...' : 'Create Invite Code'}
            </button>
          </div>

          {inviteCodes.length === 0 ? (
            <p className="text-sm text-gray-500">No invite codes yet.</p>
          ) : (
            <div className="space-y-2">
              {inviteCodes.map((invite) => (
                <div
                  key={invite.id}
                  className="rounded border border-gray-200 bg-gray-50 p-3 text-sm"
                >
                  <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                      <p className="font-medium text-gray-800">
                        {invite.created_for || 'Unnamed invite'}
                      </p>
                      <p className="text-xs text-gray-500">
                        Created {new Date(invite.created_at).toLocaleString()}
                        {invite.expires_at ? ` • Expires ${new Date(invite.expires_at).toLocaleString()}` : ''}
                      </p>
                      {invite.used_at && (
                        <p className="text-xs text-gray-500">
                          Used {new Date(invite.used_at).toLocaleString()}
                          {invite.used_by_strava_athlete_id
                            ? ` by athlete ${invite.used_by_strava_athlete_id}`
                            : ''}
                        </p>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`rounded px-2 py-1 text-xs font-semibold ${
                          invite.is_redeemable
                            ? 'bg-green-100 text-green-700'
                            : invite.revoked_at
                              ? 'bg-gray-200 text-gray-700'
                              : 'bg-amber-100 text-amber-700'
                        }`}
                      >
                        {invite.is_redeemable
                          ? 'active'
                          : invite.revoked_at
                            ? 'revoked'
                            : 'used/expired'}
                      </span>
                      {invite.is_redeemable && (
                        <button
                          onClick={() => handleRevokeInvite(invite.id)}
                          className="rounded bg-red-600 px-3 py-1 text-xs text-white hover:bg-red-700"
                        >
                          Revoke
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="card border border-red-200">
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
