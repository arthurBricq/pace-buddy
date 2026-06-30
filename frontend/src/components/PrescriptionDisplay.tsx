import type {
  OpenBlock,
  Prescription,
  PrescriptionSet,
  RecoveryBlock,
  Target,
  WorkBlock,
} from '../types';
import type { ReactElement } from 'react';

interface Props {
  prescriptionJson: string;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  if (s === 0) return `${m} min`;
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function formatDistance(meters: number): string {
  if (meters >= 1000 && meters % 1000 === 0) return `${meters / 1000} km`;
  if (meters >= 1000) return `${(meters / 1000).toFixed(2)} km`;
  return `${meters} m`;
}

function formatPace(secondsPerKm: number): string {
  const m = Math.floor(secondsPerKm / 60);
  const s = Math.round(secondsPerKm % 60);
  return `${m}:${s.toString().padStart(2, '0')}/km`;
}

function formatBlockSize(
  block: { duration_s?: number; distance_m?: number },
): string | null {
  if (block.distance_m != null) return formatDistance(block.distance_m);
  if (block.duration_s != null) return formatDuration(block.duration_s);
  return null;
}

function formatTarget(target: Target): string {
  switch (target.type) {
    case 'pace':
      return `${formatPace(target.min_s_per_km)}–${formatPace(target.max_s_per_km)}`;
    case 'speed':
      return `${target.min_mps.toFixed(2)}–${target.max_mps.toFixed(2)} m/s`;
    case 'heart_rate':
      return `${target.min_bpm}–${target.max_bpm} bpm`;
    case 'percent_mas':
      return `${target.min.toFixed(0)}–${target.max.toFixed(0)}%MAS`;
    case 'rpe':
      return `RPE ${target.min}–${target.max}`;
    case 'effort':
      return target.label;
  }
}

function formatOpenBlock(block: OpenBlock, label: string): string {
  const parts: string[] = [`**${label}**`];
  const size = formatBlockSize(block);
  if (size) parts.push(`— ${size}`);
  if (block.notes) parts.push(block.notes);
  return parts.join(' ');
}

function formatWork(work: WorkBlock): string {
  const size = formatBlockSize(work);
  const target = formatTarget(work.target);
  if (size) return `${size} @ ${target}`;
  return `@ ${target}`;
}

function formatRecovery(rec: RecoveryBlock): string {
  const size = formatBlockSize(rec);
  const targetLabel =
    rec.target && rec.target.type === 'effort' ? rec.target.label : null;
  if (size && targetLabel) return `with ${size} ${targetLabel} recovery`;
  if (size) return `with ${size} recovery`;
  return 'with recovery';
}

function formatSet(set: PrescriptionSet): string {
  const reps = set.repeat > 1 ? `${set.repeat} × ` : '';
  const work = formatWork(set.work);
  const rec = set.recovery ? ` ${formatRecovery(set.recovery)}` : '';
  return `${reps}${work}${rec}`;
}

/**
 * Render a single line that may contain `**bold**` segments. Splits on the
 * markdown-style asterisk pairs and wraps matched runs in <strong>.
 */
function renderLine(line: string, key: number): ReactElement {
  const parts = line.split(/(\*\*[^*]+\*\*)/g);
  return (
    <p key={key} className="leading-relaxed">
      {parts.map((part, i) => {
        if (part.startsWith('**') && part.endsWith('**')) {
          return <strong key={i}>{part.slice(2, -2)}</strong>;
        }
        return <span key={i}>{part}</span>;
      })}
    </p>
  );
}

export default function PrescriptionDisplay({ prescriptionJson }: Props) {
  let parsed: Prescription;
  try {
    parsed = JSON.parse(prescriptionJson) as Prescription;
  } catch {
    return (
      <div className="text-sm">
        <p className="text-xs text-amber-700 mb-1">
          Couldn't parse prescription JSON.
        </p>
        <pre className="text-xs text-gray-500 whitespace-pre-wrap">
          {prescriptionJson}
        </pre>
      </div>
    );
  }

  const lines: string[] = [];
  if (parsed.warmup) lines.push(formatOpenBlock(parsed.warmup, 'Warmup'));
  if (parsed.sets) {
    for (const set of parsed.sets) lines.push(`**${formatSet(set)}**`);
  }
  if (parsed.cooldown) lines.push(formatOpenBlock(parsed.cooldown, 'Cooldown'));

  if (lines.length === 0 && !parsed.notes) {
    return (
      <p className="text-xs text-gray-400 italic">No prescription details.</p>
    );
  }

  return (
    <div className="text-sm text-gray-700 space-y-1">
      {lines.map((line, i) => renderLine(line, i))}
      {parsed.notes && (
        <p className="text-xs text-gray-500 italic mt-2">{parsed.notes}</p>
      )}
    </div>
  );
}
