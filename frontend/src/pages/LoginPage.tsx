import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { startAuthentication } from '@simplewebauthn/browser';
import { loginStart, loginFinish, startStravaAuth } from '../api/auth';
import { useAuth } from '../hooks/useAuth';

export default function LoginPage() {
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [stravaLoading, setStravaLoading] = useState(false);
  const navigate = useNavigate();
  const { refresh } = useAuth();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const { auth_id, options } = await loginStart();
      const credential = await startAuthentication({ optionsJSON: (options as any).publicKey });
      await loginFinish(auth_id, credential);
      await refresh();
      navigate('/activities');
    } catch (err: any) {
      setError(err.message || 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  const handleStravaLogin = async () => {
    setError('');
    setStravaLoading(true);
    try {
      const { url } = await startStravaAuth();
      window.location.href = url;
    } catch (err: any) {
      setError(err.message || 'Strava login failed');
      setStravaLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="max-w-md w-full bg-white rounded-lg shadow p-8">
        <h1 className="text-2xl font-bold text-center mb-6">Log In</h1>
        <form onSubmit={handleSubmit} className="space-y-4">
          <p className="text-sm text-gray-600">
            Use your saved passkey to sign in. No username is required.
          </p>
          {error && <p className="text-red-600 text-sm">{error}</p>}
          <button
            type="submit"
            disabled={loading || stravaLoading}
            className="w-full bg-blue-600 text-white py-2 rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Authenticating...' : 'Log in with Passkey'}
          </button>
        </form>
        <div className="my-4 text-center text-xs text-gray-400">OR</div>
        <button
          type="button"
          onClick={handleStravaLogin}
          disabled={loading || stravaLoading}
          className="w-full border border-orange-500 text-orange-700 py-2 rounded-md hover:bg-orange-50 disabled:opacity-50"
        >
          {stravaLoading ? 'Redirecting to Strava...' : 'Log in with Strava'}
        </button>
        <p className="mt-4 text-center text-sm text-gray-500">
          No account?{' '}
          <Link to="/register" className="text-blue-600 hover:underline">
            Register
          </Link>
        </p>
      </div>
    </div>
  );
}
