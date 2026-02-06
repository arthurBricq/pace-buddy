import type { ActivityTag } from '../types';

const TAG_STYLES: Record<ActivityTag, string> = {
  normal: 'bg-gray-100 text-gray-700',
  intervals: 'bg-orange-100 text-orange-700',
  race: 'bg-red-100 text-red-700',
};

export default function TagBadge({ tag }: { tag: ActivityTag }) {
  return (
    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${TAG_STYLES[tag]}`}>
      {tag}
    </span>
  );
}
