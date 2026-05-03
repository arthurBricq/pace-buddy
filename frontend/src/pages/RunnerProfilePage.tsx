import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  getAthleteProfile,
  getIdentityProfile,
  upsertAthleteProfile,
  upsertIdentityProfile,
} from '../api/auth';
import Navbar from '../components/Navbar';
import { useAuth } from '../hooks/useAuth';

interface IdentityFormState {
  name: string;
  age: string;
  email: string;
  gender: string;
  height_cm: string;
  weight_kg: string;
}

interface AthleteFormState {
  goal_description: string;
  goal_date: string;
  goal_distance_km: string;
  goal_target_time: string;
  goal_sport_type: string;
  goal_elevation_gain_m: string;
  additional_info: string;
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function parseOptionalNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseOptionalInt(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number.parseInt(trimmed, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseDurationToSeconds(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const match = /^(\d{1,2}):([0-5]\d):([0-5]\d)$/.exec(trimmed);
  if (!match) return null;
  const hours = Number.parseInt(match[1], 10);
  const minutes = Number.parseInt(match[2], 10);
  const seconds = Number.parseInt(match[3], 10);
  return hours * 3600 + minutes * 60 + seconds;
}

function formatDurationFromSeconds(value: number | null): string {
  if (value == null || value <= 0) return '';
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  const seconds = value % 60;
  return `${hours.toString().padStart(2, '0')}:${minutes
    .toString()
    .padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

function errorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return fallback;
}

function safeReturnTo(value: string | null): string {
  if (!value || !value.startsWith('/') || value.startsWith('//')) return '/profile';
  return value;
}

export default function RunnerProfilePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { refresh } = useAuth();
  const returnTo = safeReturnTo(searchParams.get('returnTo'));

  const [step, setStep] = useState(1);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const [identity, setIdentity] = useState<IdentityFormState>({
    name: '',
    age: '',
    email: '',
    gender: '',
    height_cm: '',
    weight_kg: '',
  });
  const [athlete, setAthlete] = useState<AthleteFormState>({
    goal_description: '',
    goal_date: '',
    goal_distance_km: '',
    goal_target_time: '',
    goal_sport_type: '',
    goal_elevation_gain_m: '',
    additional_info: '',
  });

  useEffect(() => {
    Promise.all([getIdentityProfile(), getAthleteProfile()])
      .then(([identityProfile, athleteProfile]) => {
        if (identityProfile) {
          setIdentity({
            name: identityProfile.name ?? '',
            age: identityProfile.age != null ? String(identityProfile.age) : '',
            email: identityProfile.email ?? '',
            gender: identityProfile.gender ?? '',
            height_cm: identityProfile.height_cm != null ? String(identityProfile.height_cm) : '',
            weight_kg: identityProfile.weight_kg != null ? String(identityProfile.weight_kg) : '',
          });
        }
        if (athleteProfile) {
          setAthlete({
            goal_description: athleteProfile.goal_description ?? '',
            goal_date: athleteProfile.goal_date ?? '',
            goal_distance_km:
              athleteProfile.goal_distance_km != null ? String(athleteProfile.goal_distance_km) : '',
            goal_target_time: formatDurationFromSeconds(athleteProfile.goal_target_time_seconds),
            goal_sport_type: athleteProfile.goal_sport_type ?? '',
            goal_elevation_gain_m:
              athleteProfile.goal_elevation_gain_m != null
                ? String(athleteProfile.goal_elevation_gain_m)
                : '',
            additional_info: athleteProfile.additional_info ?? '',
          });
        }
      })
      .catch((err: unknown) => {
        setError(errorMessage(err, 'Failed to load runner profile'));
      })
      .finally(() => setLoading(false));
  }, []);

  const stepTitle = useMemo(() => {
    return step === 1 ? 'About You' : 'Running Goals';
  }, [step]);

  const handleContinue = () => {
    setError('');
    setStep(2);
  };

  const handleSubmit = async () => {
    setError('');
    const parsedGoalTargetTime = parseDurationToSeconds(athlete.goal_target_time);
    if (athlete.goal_target_time.trim() && parsedGoalTargetTime == null) {
      setError('Goal target time must use HH:MM:SS format.');
      return;
    }

    setSaving(true);
    try {
      await upsertIdentityProfile({
        name: emptyToNull(identity.name),
        age: parseOptionalInt(identity.age),
        email: emptyToNull(identity.email),
        gender: emptyToNull(identity.gender),
        height_cm: parseOptionalNumber(identity.height_cm),
        weight_kg: parseOptionalNumber(identity.weight_kg),
      });

      await upsertAthleteProfile({
        goal_description: emptyToNull(athlete.goal_description),
        goal_date: emptyToNull(athlete.goal_date),
        goal_distance_km: parseOptionalNumber(athlete.goal_distance_km),
        goal_target_time_seconds: parsedGoalTargetTime,
        goal_sport_type: emptyToNull(athlete.goal_sport_type),
        goal_elevation_gain_m: parseOptionalNumber(athlete.goal_elevation_gain_m),
        additional_info: emptyToNull(athlete.additional_info),
      });

      await refresh();
      navigate(returnTo, { replace: true });
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to save runner profile'));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="app-shell">
        <Navbar />
        <div className="page-container-compact">
          <p className="text-gray-500">Loading runner profile...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-compact section-stack">
        <div className="card">
          <p className="text-xs text-gray-500">Step {step} / 2</p>
          <h1 className="text-2xl font-bold mt-1">Runner Profile</h1>
          <p className="text-sm text-gray-600 mt-2">{stepTitle}</p>
          <p className="text-sm text-gray-600 mt-3">
            Every field is optional. Blank fields are ignored by your coach.
          </p>
        </div>

        {error && (
          <div className="card">
            <p className="text-sm text-red-600">{error}</p>
          </div>
        )}

        {step === 1 && (
          <div className="card">
            <div className="space-y-3">
              <label className="theme-field">
                <span className="theme-label">Name</span>
                <input
                  className="theme-input"
                  value={identity.name}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, name: e.target.value }))}
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Age</span>
                <input
                  className="theme-input"
                  type="number"
                  min={1}
                  max={120}
                  value={identity.age}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, age: e.target.value }))}
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Email</span>
                <input
                  className="theme-input"
                  type="email"
                  value={identity.email}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, email: e.target.value }))}
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Gender</span>
                <select
                  className="theme-select"
                  value={identity.gender}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, gender: e.target.value }))}
                >
                  <option value="">Prefer not to say</option>
                  <option value="female">Female</option>
                  <option value="male">Male</option>
                  <option value="non_binary">Non-binary</option>
                  <option value="other">Other</option>
                </select>
              </label>
              <label className="theme-field">
                <span className="theme-label">Height (cm)</span>
                <input
                  className="theme-input"
                  type="number"
                  min={50}
                  max={260}
                  step="0.1"
                  value={identity.height_cm}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, height_cm: e.target.value }))}
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Weight (kg)</span>
                <input
                  className="theme-input"
                  type="number"
                  min={20}
                  max={250}
                  step="0.1"
                  value={identity.weight_kg}
                  onChange={(e) => setIdentity((prev) => ({ ...prev, weight_kg: e.target.value }))}
                />
              </label>
            </div>
            <div className="mt-6">
              <button type="button" className="theme-btn theme-btn-primary w-full" onClick={handleContinue}>
                Continue
              </button>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="card">
            <div className="space-y-3">
              <label className="theme-field">
                <span className="theme-label">Goal description</span>
                <textarea
                  className="theme-textarea"
                  rows={3}
                  placeholder="Describe your current goal in your own words."
                  value={athlete.goal_description}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, goal_description: e.target.value }))
                  }
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Goal date</span>
                <input
                  className="theme-input"
                  type="date"
                  value={athlete.goal_date}
                  onChange={(e) => setAthlete((prev) => ({ ...prev, goal_date: e.target.value }))}
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Goal distance (km)</span>
                <input
                  className="theme-input"
                  type="number"
                  min={0}
                  step="0.1"
                  value={athlete.goal_distance_km}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, goal_distance_km: e.target.value }))
                  }
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Goal target time (HH:MM:SS)</span>
                <input
                  className="theme-input"
                  placeholder="03:30:00"
                  value={athlete.goal_target_time}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, goal_target_time: e.target.value }))
                  }
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Goal sport type</span>
                <select
                  className="theme-select"
                  value={athlete.goal_sport_type}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, goal_sport_type: e.target.value }))
                  }
                >
                  <option value="">Not specified</option>
                  <option value="running">Running</option>
                  <option value="trail_running">Trail running</option>
                </select>
              </label>
              <label className="theme-field">
                <span className="theme-label">Goal elevation gain (m)</span>
                <input
                  className="theme-input"
                  type="number"
                  min={0}
                  step="1"
                  value={athlete.goal_elevation_gain_m}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, goal_elevation_gain_m: e.target.value }))
                  }
                />
              </label>
              <label className="theme-field">
                <span className="theme-label">Additional information</span>
                <textarea
                  className="theme-textarea"
                  rows={3}
                  placeholder="Anything else your coach should know."
                  value={athlete.additional_info}
                  onChange={(e) =>
                    setAthlete((prev) => ({ ...prev, additional_info: e.target.value }))
                  }
                />
              </label>
              <div className="rounded-md border border-gray-200 bg-gray-50 p-3">
                <p className="text-xs font-medium text-gray-700">Optional ideas</p>
                <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-gray-600">
                  <li>Injury history or current limitations</li>
                  <li>Weekly schedule constraints</li>
                  <li>Preferred terrain or running environment</li>
                  <li>Training preferences or things to avoid</li>
                  <li>Race priorities, motivation, or coaching style preferences</li>
                </ul>
              </div>
            </div>
            <div className="mt-6 flex gap-3">
              <button
                type="button"
                className="theme-btn theme-btn-outline w-1/2"
                onClick={() => setStep(1)}
                disabled={saving}
              >
                Back
              </button>
              <button
                type="button"
                className="theme-btn theme-btn-primary w-1/2"
                onClick={handleSubmit}
                disabled={saving}
              >
                {saving ? 'Saving...' : 'Save Runner Profile'}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
