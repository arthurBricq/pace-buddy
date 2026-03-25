import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getModelCostTiers, listModels } from '../api/chats';
import type { ModelCostCategory, ModelCostTier, ModelInfo, RunningCoachSettings } from '../types';

interface CoachSettingsModalProps {
  isOpen: boolean;
  initial: RunningCoachSettings | null;
  onClose: () => void;
  onSave: (next: RunningCoachSettings) => Promise<void>;
  onResetCoach: () => Promise<void>;
}

export default function CoachSettingsModal({
  isOpen,
  initial,
  onClose,
  onSave,
  onResetCoach,
}: CoachSettingsModalProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [tiers, setTiers] = useState<ModelCostTier[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const [model, setModel] = useState('google/gemini-2.5-flash');
  const [personality, setPersonality] = useState('');
  const [considerTrailRunsAsRuns, setConsiderTrailRunsAsRuns] = useState(false);
  const [volumeWeeks, setVolumeWeeks] = useState(8);
  const [lastWorkoutsCount, setLastWorkoutsCount] = useState(8);
  const [lastLongRunsCount, setLastLongRunsCount] = useState(6);
  const [lastRacesCount, setLastRacesCount] = useState(4);
  const [newActivitiesCount, setNewActivitiesCount] = useState(8);
  const [normalizerEveryNMessages, setNormalizerEveryNMessages] = useState(6);

  useEffect(() => {
    if (!isOpen) return;
    if (initial) {
      setModel(initial.model);
      setPersonality(initial.personality);
      setConsiderTrailRunsAsRuns(initial.consider_trail_runs_as_runs);
      setVolumeWeeks(initial.volume_weeks);
      setLastWorkoutsCount(initial.last_workouts_count);
      setLastLongRunsCount(initial.last_long_runs_count);
      setLastRacesCount(initial.last_races_count);
      setNewActivitiesCount(initial.new_activities_count);
      setNormalizerEveryNMessages(initial.normalizer_every_n_messages);
    }
  }, [isOpen, initial]);

  useEffect(() => {
    if (!isOpen) return;
    setLoadingModels(true);
    Promise.all([
      listModels(),
      getModelCostTiers().catch(() => []),
    ])
      .then(([loadedModels, loadedTiers]) => {
        setModels(loadedModels);
        setTiers(loadedTiers);
      })
      .catch((err) => setError(err.message || 'Failed to load models'))
      .finally(() => setLoadingModels(false));
  }, [isOpen]);

  if (!isOpen || !initial) return null;

  const categoryByModelId = new Map(tiers.map((tier) => [tier.model_id, tier.category]));

  const categoryStyle = (category?: ModelCostCategory) => {
    switch (category) {
      case 'economical':
        return 'bg-green-100 text-green-800 border border-green-200';
      case 'standard':
        return 'bg-blue-100 text-blue-800 border border-blue-200';
      case 'expensive':
        return 'bg-amber-100 text-amber-800 border border-amber-200';
      default:
        return 'bg-gray-100 text-gray-700 border border-gray-200';
    }
  };

  const categoryLabel = (category?: ModelCostCategory) => {
    switch (category) {
      case 'economical':
        return 'Economical';
      case 'standard':
        return 'Standard';
      case 'expensive':
        return 'Expensive';
      default:
        return 'Unrated';
    }
  };

  const buildSettings = (): RunningCoachSettings => ({
    ...initial,
    model,
    personality: personality.trim(),
    consider_trail_runs_as_runs: considerTrailRunsAsRuns,
    volume_weeks: volumeWeeks,
    last_workouts_count: lastWorkoutsCount,
    last_long_runs_count: lastLongRunsCount,
    last_races_count: lastRacesCount,
    new_activities_count: newActivitiesCount,
    normalizer_every_n_messages: normalizerEveryNMessages,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    if (!personality.trim()) {
      setError('Personality cannot be empty');
      return;
    }
    setSaving(true);
    try {
      await onSave(buildSettings());
      onClose();
    } catch (err: any) {
      setError(err.message || 'Failed to save settings');
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    if (!confirm('Reset coach memory, settings, and messages?')) return;
    setSaving(true);
    setError('');
    try {
      await onResetCoach();
      onClose();
    } catch (err: any) {
      setError(err.message || 'Failed to reset coach');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="modal-card max-w-2xl w-full max-h-[92vh] overflow-y-auto">
        <div className="flex items-center justify-between px-4 py-4 border-b sm:px-6">
          <h3 className="text-lg font-semibold">Coach Settings</h3>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 text-2xl leading-none"
          >
            &times;
          </button>
        </div>

        <form onSubmit={handleSubmit} className="px-4 py-4 space-y-5 sm:px-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">Coach Personality</label>
            <textarea
              value={personality}
              onChange={(e) => setPersonality(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
              placeholder="Opinionated but constructive running coach..."
            />
          </div>

          <div className="rounded-md border border-gray-200 bg-gray-50 p-3">
            <label className="flex items-start gap-3">
              <input
                type="checkbox"
                checked={considerTrailRunsAsRuns}
                onChange={(e) => setConsiderTrailRunsAsRuns(e.target.checked)}
                className="mt-1 h-4 w-4 rounded border-gray-300 text-purple-600 focus:ring-purple-500"
              />
              <div>
                <p className="text-sm font-medium text-gray-800">Consider trail runs as runs</p>
                <p className="mt-1 text-xs text-gray-600">
                  When enabled, coach requests for runs also include Strava TrailRun activities.
                </p>
              </div>
            </label>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">LLM Model</label>
            {loadingModels ? (
              <p className="text-sm text-gray-500">Loading models...</p>
            ) : (
              <div className="max-h-56 overflow-y-auto space-y-2">
                {models.map((candidate) => (
                  <button
                    key={candidate.id}
                    type="button"
                    onClick={() => setModel(candidate.id)}
                    className={`w-full rounded-md border p-3 text-left transition-colors ${
                      model === candidate.id
                        ? 'border-purple-400 ring-2 ring-purple-200 bg-purple-50'
                        : 'border-gray-200 hover:border-gray-300 bg-white'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="text-sm font-medium text-gray-900">{candidate.name}</p>
                        <p className="text-xs text-gray-500 mt-1">{candidate.id}</p>
                      </div>
                      <span
                        className={`text-xs px-2 py-0.5 rounded ${categoryStyle(categoryByModelId.get(candidate.id))}`}
                      >
                        {categoryLabel(categoryByModelId.get(candidate.id))}
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            )}
            <div className="mt-2 flex justify-end">
              <Link to="/help#llm-models-and-costs" className="text-xs text-blue-600 hover:underline">
                Learn about model cost categories
              </Link>
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <NumberInput label="Volume window (weeks)" value={volumeWeeks} onChange={setVolumeWeeks} min={1} max={24} />
            <NumberInput label="Last workouts" value={lastWorkoutsCount} onChange={setLastWorkoutsCount} min={1} max={25} />
            <NumberInput label="Last long runs" value={lastLongRunsCount} onChange={setLastLongRunsCount} min={1} max={25} />
            <NumberInput label="Last races" value={lastRacesCount} onChange={setLastRacesCount} min={1} max={25} />
            <NumberInput label="New activities per exchange" value={newActivitiesCount} onChange={setNewActivitiesCount} min={1} max={25} />
            <NumberInput label="Normalize memory every N messages" value={normalizerEveryNMessages} onChange={setNormalizerEveryNMessages} min={1} max={20} />
          </div>

          {error && <p className="text-sm text-red-600">{error}</p>}

          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-between sm:items-center">
            <button
              type="button"
              onClick={handleReset}
              disabled={saving}
              className="bg-red-600 text-white px-4 py-2 rounded-md hover:bg-red-700 disabled:opacity-50 text-sm"
            >
              Reset Coach
            </button>
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:gap-3">
              <button
                type="button"
                onClick={onClose}
                className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={saving}
                className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 text-sm"
              >
                Save Settings
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}

function NumberInput({
  label,
  value,
  onChange,
  min,
  max,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
}) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 mb-1">{label}</label>
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(parseInt(e.target.value, 10) || min)}
        className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
      />
    </div>
  );
}
