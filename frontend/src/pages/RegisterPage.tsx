import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { startRegistration } from '@simplewebauthn/browser';
import { registerStart, registerFinish, startStravaAuth } from '../api/auth';
import { useAuth } from '../hooks/useAuth';

export default function RegisterPage() {
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
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
      const { user_id, options } = await registerStart(username, email || undefined);
      const credential = await startRegistration({ optionsJSON: (options as any).publicKey });
      await registerFinish(user_id, credential);
      await refresh();
      navigate('/activities');
    } catch (err: any) {
      setError(err.message || 'Registration failed');
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
    <div className="auth-shell">
      <div className="auth-card">
        <h1 className="text-2xl font-bold text-center mb-6">Create Account</h1>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Username
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Email{' '}
              <span className="text-xs font-semibold text-amber-700 bg-amber-100 px-2 py-0.5 rounded">
                Optional
              </span>
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <p className="mt-1 text-xs text-gray-500">Optional, but recommended so we can contact you later.</p>
          </div>
          {error && <p className="text-red-600 text-sm">{error}</p>}
          <button
            type="submit"
            disabled={loading || stravaLoading}
            className="w-full bg-blue-600 text-white py-2 rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Registering...' : 'Register with Passkey'}
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
          Already have an account?{' '}
          <Link to="/login" className="text-blue-600 hover:underline">
            Log in
          </Link>
        </p>
      </div>
    </div>
  );
}
