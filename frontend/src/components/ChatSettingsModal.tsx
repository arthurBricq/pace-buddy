import { useState, useEffect } from 'react';
import { listModels } from '../api/chats';
import type { ModelInfo } from '../types';

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
  const [loading, setLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState(defaultModel);
  const [conversationLength, setConversationLength] = useState(defaultConversationLength);

  useEffect(() => {
    if (isOpen && !hideModelSelector && models.length === 0) {
      setLoading(true);
      listModels()
        .then(setModels)
        .catch((err) => {
          console.error('Failed to load models:', err);
        })
        .finally(() => setLoading(false));
    }
  }, [isOpen, hideModelSelector, models.length]);

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
      <div className="bg-white rounded-xl shadow-2xl max-w-md w-full">
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <h3 className="text-lg font-semibold">{title}</h3>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 text-2xl leading-none"
          >
            &times;
          </button>
        </div>
        <form onSubmit={handleSubmit} className="px-6 py-4 space-y-4">
          {!hideModelSelector && (
            <div>
              <label htmlFor="model" className="block text-sm font-medium text-gray-700 mb-2">
                LLM Model
              </label>
              {loading ? (
                <div className="flex items-center gap-2 text-gray-500 text-sm">
                  <div className="animate-spin h-4 w-4 border-2 border-purple-600 border-t-transparent rounded-full" />
                  Loading models...
                </div>
              ) : (
                <select
                  id="model"
                  value={selectedModel}
                  onChange={(e) => setSelectedModel(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 text-sm"
                  required
                >
                  {models.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.name}
                    </option>
                  ))}
                </select>
              )}
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

          <div className="flex gap-3 pt-2">
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
