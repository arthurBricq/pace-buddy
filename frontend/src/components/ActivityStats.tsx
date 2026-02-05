import { Activity } from '../types';

interface StatProps {
  label: string;
  value: string;
}

function StatCard({ label, value }: StatProps) {
  return (
    <div className="bg-white rounded-lg shadow p-4">
      <p className="text-xs text-gray-500 uppercase tracking-wide">{label}</p>
      <p className="text-lg font-semibold text-gray-900 mt-1">{value}</p>
    </div>
  );
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}h ${m}m ${s}s`;
  return `${m}m ${s}s`;
}

function formatPace(avgSpeed: number): string {
  if (avgSpeed <= 0) return '-';
  const paceSeconds = 1000 / avgSpeed;
  const m = Math.floor(paceSeconds / 60);
  const s = Math.round(paceSeconds % 60);
  return `${m}:${s.toString().padStart(2, '0')} /km`;
}

export default function ActivityStats({ activity }: { activity: Activity }) {
  const stats: StatProps[] = [
    { label: 'Distance', value: (activity.distance / 1000).toFixed(2) + ' km' },
    { label: 'Moving Time', value: formatDuration(activity.moving_time) },
    { label: 'Elapsed Time', value: formatDuration(activity.elapsed_time) },
    { label: 'Pace', value: formatPace(activity.average_speed) },
    { label: 'Elevation', value: activity.total_elevation_gain.toFixed(0) + ' m' },
  ];

  if (activity.average_heartrate) {
    stats.push({ label: 'Avg HR', value: Math.round(activity.average_heartrate) + ' bpm' });
  }
  if (activity.max_heartrate) {
    stats.push({ label: 'Max HR', value: Math.round(activity.max_heartrate) + ' bpm' });
  }
  if (activity.average_cadence) {
    stats.push({ label: 'Avg Cadence', value: Math.round(activity.average_cadence * 2) + ' spm' });
  }
  if (activity.calories) {
    stats.push({ label: 'Calories', value: Math.round(activity.calories) + ' kcal' });
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
      {stats.map((s) => (
        <StatCard key={s.label} {...s} />
      ))}
    </div>
  );
}
