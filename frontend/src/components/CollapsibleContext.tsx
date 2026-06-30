import { useState } from 'react';

interface CollapsibleContextProps {
  label: string;
  content: string;
}

export default function CollapsibleContext({ label, content }: CollapsibleContextProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="w-full border border-purple-200 bg-purple-50 rounded-lg overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-4 py-2 text-sm text-purple-700 hover:bg-purple-100 transition-colors"
      >
        <span className="font-medium">Context: {label}</span>
        <span className="text-purple-400">{expanded ? '▲' : '▼'}</span>
      </button>
      {expanded && (
        <pre className="px-4 py-3 text-xs text-gray-700 border-t border-purple-200 bg-white whitespace-pre-wrap overflow-x-auto max-h-96 overflow-y-auto">
          {content}
        </pre>
      )}
    </div>
  );
}
