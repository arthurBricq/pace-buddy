import { useEffect, useMemo, useState } from 'react';
import { getModelCostTiers } from '../api/chats';
import Navbar from '../components/Navbar';
import type { ModelCostCategory, ModelCostTier } from '../types';

function categoryStyle(category: ModelCostCategory): string {
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
}

function categoryLabel(category: ModelCostCategory): string {
  switch (category) {
    case 'economical':
      return 'Economical';
    case 'standard':
      return 'Standard';
    case 'expensive':
      return 'Expensive';
    default:
      return category;
  }
}

export default function HelpPage() {
  const [tiers, setTiers] = useState<ModelCostTier[]>([]);
  const [tiersLoading, setTiersLoading] = useState(true);
  const [tiersError, setTiersError] = useState('');

  useEffect(() => {
    getModelCostTiers()
      .then(setTiers)
      .catch((err: any) => setTiersError(err.message || 'Failed to load model cost tiers'))
      .finally(() => setTiersLoading(false));
  }, []);

  const grouped = useMemo(() => {
    const base: Record<ModelCostCategory, ModelCostTier[]> = {
      economical: [],
      standard: [],
      expensive: [],
    };
    for (const tier of tiers) {
      base[tier.category].push(tier);
    }
    return base;
  }, [tiers]);

  const lastUpdated = useMemo(() => {
    if (tiers.length === 0) return null;
    const latest = tiers
      .map((tier) => new Date(tier.computed_at).getTime())
      .reduce((max, current) => Math.max(max, current), 0);
    return Number.isFinite(latest) ? new Date(latest) : null;
  }, [tiers]);

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-narrow section-stack">
        <div>
          <h1 className="text-2xl font-bold">Help</h1>
          <p className="text-sm text-gray-500 mt-1">
            Quick guide to understand the main concepts and get the most out of Pace Buddy.
          </p>
        </div>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">How to use the app</h2>
          <div className="space-y-4 text-sm text-gray-700">
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">What is a training?</h3>
              <p>
                A training is a time window with a goal. The app derives quality activities inside
                that range (intervals, long runs, races), then uses them for analysis and insights.
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">What is an LLM chat?</h3>
              <p>
                An LLM chat is your coaching conversation with AI. You can start from scratch, or
                continue from a training insight, and add contextual data directly in the chat.
              </p>
            </div>
          </div>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">Tips</h2>
          <ul className="list-disc pl-5 space-y-2 text-sm text-gray-700">
            <li>
              Tag activities directly in Strava (races, workouts, long runs) so they are imported
              correctly and you avoid manual friction.
            </li>
            <li>
              Use meaningful activity names, especially for interval sessions. Clear names improve
              your own review flow and AI interpretation quality.
            </li>
          </ul>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">About</h2>
          <p className="text-sm text-gray-700">
            Pace Buddy is a personal project under active development. Features and behavior may
            evolve quickly, so always use your own judgment for training decisions.
          </p>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">LLM models and costs</h2>
          <p className="text-sm text-gray-700 mb-3">
            Some LLM models are more expensive than others. Model cost differences are already
            reflected in usage, and cost controls will be improved further soon.
          </p>
          {tiersLoading ? (
            <p className="text-sm text-gray-500">Loading model cost categories...</p>
          ) : tiersError ? (
            <p className="text-sm text-red-600">{tiersError}</p>
          ) : tiers.length === 0 ? (
            <p className="text-sm text-gray-500">No model cost categories available yet.</p>
          ) : (
            <div className="space-y-3">
              {(['economical', 'standard', 'expensive'] as ModelCostCategory[]).map((category) => (
                <div key={category} className="rounded-md border border-gray-200 p-3">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <span className={`text-xs px-2 py-1 rounded ${categoryStyle(category)}`}>
                      {categoryLabel(category)}
                    </span>
                    <span className="text-xs text-gray-500">
                      {grouped[category].length} model{grouped[category].length === 1 ? '' : 's'}
                    </span>
                  </div>
                  {grouped[category].length === 0 ? (
                    <p className="text-sm text-gray-700 mt-2">None</p>
                  ) : (
                    <ul className="mt-2 space-y-1">
                      {grouped[category].map((tier) => (
                        <li key={tier.model_id} className="text-sm text-gray-700">
                          {tier.model_name}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}
              {lastUpdated && (
                <p className="text-xs text-gray-500">
                  Updated: {lastUpdated.toLocaleDateString()} {lastUpdated.toLocaleTimeString()}
                </p>
              )}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
