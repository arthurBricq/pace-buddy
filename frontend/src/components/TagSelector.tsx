import type { ActivityTag } from '../types';

const TAGS: ActivityTag[] = ['normal', 'intervals', 'long_run', 'race'];

interface Props {
  current: ActivityTag;
  onChange: (tag: ActivityTag) => void;
}

function label(tag: ActivityTag): string {
  return tag.replace('_', ' ');
}

export default function TagSelector({ current, onChange }: Props) {
  return (
    <div className="flex gap-1">
      {TAGS.map((tag) => (
        <button
          key={tag}
          onClick={() => onChange(tag)}
          className={`px-2 py-1 rounded text-xs font-medium border transition-colors ${
            tag === current
              ? 'bg-blue-600 text-white border-blue-600'
              : 'bg-white text-gray-600 border-gray-300 hover:border-blue-400'
          }`}
        >
          {label(tag)}
        </button>
      ))}
    </div>
  );
}
