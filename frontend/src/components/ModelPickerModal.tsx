import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getModelCostTiers, listModels } from '../api/models';
import type { ModelCostCategory, ModelCostTier, ModelInfo } from '../types';

interface ModelPickerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (model: string) => void;
  defaultModel?: string;
  title?: string;
  confirmLabel?: string;
}

export default function ModelPickerModal({
  isOpen,
  onClose,
  onConfirm,
  defaultModel = 'google/gemini-2.5-flash',
  title = 'Choose Model',
  confirmLabel = 'Continue',
}: ModelPickerModalProps) {
  const [models, setModels] = useState<ModelInfo[] | null>(null);
  const [modelCostTiers, setModelCostTiers] = useState<ModelCostTier[]>([]);
  const [selectedModel, setSelectedModel] = useState(defaultModel);

  const categoryByModelId = new Map(modelCostTiers.map((tier) => [tier.model_id, tier.category]));

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

  useEffect(() => {
    if (!isOpen || models !== null) return;
    Promise.all([
      listModels(),
      getModelCostTiers().catch((err) => {
        console.warn('Failed to load model cost tiers:', err);
        return [];
      }),
    ])
      .then(([loadedModels, loadedTiers]) => {
        setModels(loadedModels);
        setModelCostTiers(loadedTiers);
      })
      .catch((err) => {
        console.error('Failed to load models:', err);
        setModels([]);
        setModelCostTiers([]);
      });
  }, [isOpen, models]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onConfirm(selectedModel);
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="modal-card max-w-md">
        <div className="flex items-center justify-between px-4 py-4 border-b sm:px-6">
          <h3 className="text-lg font-semibold">{title}</h3>
          <button
            type="button"
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 text-2xl leading-none"
          >
            &times;
          </button>
        </div>
        <form onSubmit={handleSubmit} className="px-4 py-4 space-y-4 sm:px-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              LLM Model
            </label>
            {models === null ? (
              <div className="flex items-center gap-2 text-gray-500 text-sm">
                <div className="animate-spin h-4 w-4 border-2 border-purple-600 border-t-transparent rounded-full" />
                Loading models...
              </div>
            ) : (
              <div className="max-h-64 overflow-y-auto space-y-2">
                {models.map((model) => (
                  <button
                    key={model.id}
                    type="button"
                    onClick={() => setSelectedModel(model.id)}
                    className={`w-full rounded-md border p-3 text-left transition-colors ${
                      selectedModel === model.id
                        ? 'border-purple-400 ring-2 ring-purple-200 bg-purple-50'
                        : 'border-gray-200 hover:border-gray-300 bg-white'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="text-sm font-medium text-gray-900">{model.name}</p>
                        <p className="text-xs text-gray-500 mt-1">{model.id}</p>
                      </div>
                      <span
                        className={`text-xs px-2 py-0.5 rounded ${categoryStyle(categoryByModelId.get(model.id))}`}
                      >
                        {categoryLabel(categoryByModelId.get(model.id))}
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            )}
            <div className="mt-2 flex justify-end">
              <Link
                to="/help#llm-models-and-costs"
                className="text-xs text-blue-600 hover:underline"
              >
                Learn about model cost categories
              </Link>
            </div>
          </div>

          <div className="flex flex-col-reverse gap-2 pt-2 sm:flex-row sm:gap-3">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={models === null || !selectedModel}
              className="flex-1 bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              {confirmLabel}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
