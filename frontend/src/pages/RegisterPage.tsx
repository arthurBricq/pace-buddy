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
        <Link to="/login" className="auth-logo-link" aria-label="Back to PaceBuddy landing">
          <img src="/pace-buddy-logo.svg" alt="PaceBuddy" className="auth-logo" />
        </Link>
        <h1 className="auth-title">Create your account</h1>
        <p className="auth-subtitle">
          Use a passkey to register, then sync your activities from Strava.
        </p>

        <form onSubmit={handleSubmit} className="auth-form">
          <div className="theme-field">
            <label className="theme-label">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              className="theme-input"
            />
          </div>
          <div className="theme-field">
            <label className="theme-label">
              Email{' '}
              <span className="theme-optional-pill">Optional</span>
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="theme-input"
            />
            <p className="theme-help">Recommended so we can contact you later if needed.</p>
          </div>
          {error && <p className="theme-error-text">{error}</p>}
          <button
            type="submit"
            disabled={loading || stravaLoading}
            className="theme-btn theme-btn-primary w-full"
          >
            {loading ? 'Registering...' : 'Register with Passkey'}
          </button>
        </form>

        <div className="theme-divider">or</div>

        <button
          type="button"
          onClick={handleStravaLogin}
          disabled={loading || stravaLoading}
          className="brand-btn brand-strava-btn w-full"
        >
          {stravaLoading ? (
            'Redirecting to Strava...'
          ) : (
            <img
              src="/btn_strava_connect_with_orange.svg"
              alt="Connect with Strava"
              className="strava-connect-img"
            />
          )}
        </button>

        <p className="auth-footer-text">
          Already have an account?{' '}
          <Link to="/login" className="theme-link">
            Log in
          </Link>
        </p>
      </div>
    </div>
  );
}
