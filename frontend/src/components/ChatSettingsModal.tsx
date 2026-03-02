import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { getModelCostTiers, listModels } from '../api/chats';
import type { ModelCostCategory, ModelCostTier, ModelInfo } from '../types';

interface ChatSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (model: string, conversationLength: number) => void;
  defaultModel?: string;
  defaultConversationLength?: number;
  /** Hide the model selector (use defaultModel as-is) */
  hideModelSelector?: boolean;
  /** Hide the conversation length field */
  hideConversationLength?: boolean;
  /** Custom modal title */
  title?: string;
  /** Custom confirm button label */
  confirmLabel?: string;
}

export default function ChatSettingsModal({
  isOpen,
  onClose,
  onConfirm,
  defaultModel = 'google/gemini-2.5-flash',
  defaultConversationLength = 20,
  hideModelSelector = false,
  hideConversationLength = false,
  title = 'Chat Settings',
  confirmLabel = 'Continue to Chat',
}: ChatSettingsModalProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelCostTiers, setModelCostTiers] = useState<ModelCostTier[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState(defaultModel);
  const [conversationLength, setConversationLength] = useState(defaultConversationLength);

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
    if (isOpen && !hideModelSelector) {
      setLoading(true);
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
        })
        .finally(() => setLoading(false));
    }
  }, [isOpen, hideModelSelector]);

  useEffect(() => {
    if (isOpen) {
      setSelectedModel(defaultModel);
      setConversationLength(defaultConversationLength);
    }
  }, [isOpen, defaultModel, defaultConversationLength]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onConfirm(selectedModel, conversationLength);
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="modal-card max-w-md">
        <div className="flex items-center justify-between px-4 py-4 border-b sm:px-6">
          <h3 className="text-lg font-semibold">{title}</h3>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 text-2xl leading-none"
          >
            &times;
          </button>
        </div>
        <form onSubmit={handleSubmit} className="px-4 py-4 space-y-4 sm:px-6">
          {!hideModelSelector && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                LLM Model
              </label>
              {loading ? (
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
          )}

          {!hideConversationLength && (
            <div>
              <label
                htmlFor="conversationLength"
                className="block text-sm font-medium text-gray-700 mb-2"
              >
                Conversation Length (number of messages)
              </label>
              <input
                id="conversationLength"
                type="number"
                min="1"
                max="100"
                value={conversationLength}
                onChange={(e) => setConversationLength(parseInt(e.target.value) || 20)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
                required
              />
              <p className="text-xs text-gray-500 mt-1">
                Maximum number of messages to keep in context (excluding system message)
              </p>
            </div>
          )}

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
              disabled={loading || !selectedModel}
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
