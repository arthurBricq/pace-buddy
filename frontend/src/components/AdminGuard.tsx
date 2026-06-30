import { useEffect, useState } from 'react';
import { Navigate } from 'react-router-dom';
import { getAdminStats } from '../api/admin';
import { errorMessage } from '../api/client';

type GuardState = 'checking' | 'allowed' | 'denied' | 'error';

export default function AdminGuard({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<GuardState>('checking');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getAdminStats()
      .then(() => {
        if (!cancelled) {
          setState('allowed');
        }
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const message = errorMessage(err, '');
        if (
          message.includes('Forbidden') ||
          message.includes('Not an admin') ||
          message.includes('Admin access is not configured')
        ) {
          setState('denied');
          return;
        }

        if (message === 'Unauthorized') {
          setState('denied');
          return;
        }

        setError(message || 'Unable to verify admin access');
        setState('error');
      });

    return () => {
      cancelled = true;
    };
  }, []);

  if (state === 'checking') {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="theme-muted">Checking admin access...</div>
      </div>
    );
  }

  if (state === 'denied') {
    return <Navigate to="/activities" replace />;
  }

  if (state === 'error') {
    return (
      <div className="flex items-center justify-center h-screen p-4">
        <div className="theme-notice theme-notice-error max-w-lg">
          {error || 'Unable to verify admin access'}
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
