import type { IntervalResult } from '../types';

interface Props {
  intervals: IntervalResult;
  masCurrent?: number | null; // Current MAS in km/h
  autoDetected?: boolean;
}

function formatPace(paceSecondsPerKm: number): string {
  if (paceSecondsPerKm <= 0 || !isFinite(paceSecondsPerKm)) return '-';
  const m = Math.floor(paceSecondsPerKm / 60);
  const s = Math.round(paceSecondsPerKm % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function formatDistance(meters: number): string {
  if (meters >= 1000) return `${(meters / 1000).toFixed(2)} km`;
  return `${Math.round(meters)} m`;
}

function scoreColor(score: number): string {
  if (score >= 0.7) return 'bg-green-100 text-green-800';
  if (score >= 0.4) return 'bg-yellow-100 text-yellow-800';
  return 'bg-gray-100 text-gray-600';
}

function calculateMASPercent(avgSpeedMps: number, masKmh: number | null | undefined): number | null {
  if (!masKmh || masKmh <= 0) return null;
  const masMps = masKmh / 3.6;
  return (avgSpeedMps / masMps) * 100;
}

export default function IntervalRecap({ intervals, masCurrent, autoDetected }: Props) {
  const { reps, interval_score } = intervals;

  return (
    <div className="bg-white rounded-lg shadow p-4 space-y-4">
      {autoDetected && (
        <div className="text-xs text-gray-500 italic">
          Auto-detected interval workout — based on the session's speed pattern, not your tag.
        </div>
      )}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-800">
          {reps.length} rep{reps.length !== 1 ? 's' : ''} detected
        </h2>
        <div className="flex items-center gap-2">
          <span
            className="inline-flex h-5 w-5 items-center justify-center rounded-full border border-gray-300 text-xs text-gray-500 cursor-help"
            title="Session score combines rep count, work/recovery alternation, and pace consistency. Higher is more interval-like and consistent."
            aria-label="Session score explanation"
          >
            ?
          </span>
          <span className={`text-base font-semibold px-3 py-1.5 rounded-full ${scoreColor(interval_score)}`}>
            Session score: {(interval_score * 100).toFixed(0)}%
          </span>
        </div>
      </div>

      {reps.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[640px] text-sm">
            <thead>
              <tr className="text-left text-gray-500 border-b">
                <th className="pb-2 pr-4">#</th>
                <th className="pb-2 pr-4">Distance</th>
                <th className="pb-2 pr-4">Duration</th>
                <th className="pb-2 pr-4">Pace</th>
                <th className="pb-2 pr-4">MAS%</th>
                <th className="pb-2">Recovery</th>
              </tr>
            </thead>
            <tbody>
              {reps.map((rep) => {
                const masPercent = masCurrent ? calculateMASPercent(rep.avg_speed_mps, masCurrent) : null;
                return (
                  <tr key={rep.rep_index} className="border-b border-gray-100">
                    <td className="py-2 pr-4 text-gray-600">{rep.rep_index + 1}</td>
                    <td className="py-2 pr-4">{formatDistance(rep.distance_m)}</td>
                    <td className="py-2 pr-4">{formatDuration(rep.duration_s)}</td>
                    <td className="py-2 pr-4">{formatPace(rep.avg_pace_s_per_km)} /km</td>
                    <td className="py-2 pr-4">
                      {masPercent !== null ? (
                        <span className={`font-medium ${
                          masPercent >= 100 ? 'text-red-600' :
                          masPercent >= 90 ? 'text-orange-600' :
                          masPercent >= 80 ? 'text-yellow-600' :
                          'text-green-600'
                        }`}>
                          {masPercent.toFixed(1)}%
                        </span>
                      ) : (
                        <span className="text-gray-400" title="Set your MAS on the Races page to see percentages">
                          -
                        </span>
                      )}
                    </td>
                    <td className="py-2 text-gray-500">
                      {rep.recovery_duration_s != null
                        ? formatDuration(rep.recovery_duration_s)
                        : '-'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
