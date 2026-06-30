import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { getAdminCoachContext, type AdminCoachContextRow } from '../api/admin';
import Navbar from '../components/Navbar';
import { errorMessage } from '../api/client';

export default function AdminCoachContextPage() {
  const { userId } = useParams<{ userId: string }>();
  const [context, setContext] = useState<AdminCoachContextRow | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!userId) return;

    getAdminCoachContext(userId)
      .then(setContext)
      .catch((err: unknown) => setError(errorMessage(err, 'Failed to load coach context')))
      .finally(() => setLoading(false));
  }, [userId]);

  if (!userId) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-wide section-stack">
          <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
            Missing user id
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide section-stack">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl font-bold">AI Coach Context</h1>
            {context && (
              <p className="text-sm text-gray-500 mt-1">
                {context.display_name || context.username} ({context.username})
              </p>
            )}
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
        ) : context ? (
          <>
            <div className="card">
              <h3 className="text-lg font-semibold mb-4">Coach State</h3>
              <div className="grid gap-2 text-sm sm:grid-cols-2">
                <p><span className="font-semibold">Model:</span> <span className="font-mono text-xs">{context.model}</span></p>
                <p><span className="font-semibold">Last interaction:</span> {context.last_interaction_at ? new Date(context.last_interaction_at).toLocaleString() : 'Never'}</p>
                <p><span className="font-semibold">Last seen activity:</span> {context.last_seen_activity_start_date ? new Date(context.last_seen_activity_start_date).toLocaleString() : 'None'}</p>
                <p><span className="font-semibold">Normalization counter:</span> {context.message_count_since_normalization}</p>
                <p><span className="font-semibold">Pinned facts:</span> {context.pinned_facts_count}</p>
                <p><span className="font-semibold">Episodic memories:</span> {context.episodic_memory_count}</p>
              </div>
            </div>

            <div className="card">
              <h3 className="text-lg font-semibold mb-2">Coach Personality</h3>
              <p className="text-sm text-gray-700 whitespace-pre-wrap">{context.personality || '-'}</p>
            </div>

            <div className="card">
              <h3 className="text-lg font-semibold mb-2">Active Plan</h3>
              <p className="text-sm text-gray-700 whitespace-pre-wrap">{context.active_coaching_plan || '-'}</p>
              <h3 className="text-lg font-semibold mt-4 mb-2">Rolling Summary</h3>
              <p className="text-sm text-gray-700 whitespace-pre-wrap">{context.rolling_summary || '-'}</p>
            </div>

            <div className="card">
              <h3 className="text-lg font-semibold mb-4">Context Snapshot</h3>
              <pre className="max-h-[70vh] overflow-auto rounded border border-gray-200 bg-gray-50 p-3 text-xs whitespace-pre-wrap">
                {context.context_snapshot || 'No context snapshot available.'}
              </pre>
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}
