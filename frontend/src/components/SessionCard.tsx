import PrescriptionDisplay from './PrescriptionDisplay';
import type { SessionStatus, SessionType, TrainingSession } from '../types';

function statusBadgeClass(status: SessionStatus): string {
  switch (status) {
    case 'suggested':
      return 'bg-blue-100 text-blue-800';
    case 'planned':
      return 'bg-amber-100 text-amber-800';
    case 'done':
      return 'bg-green-100 text-green-800';
    case 'skipped':
      return 'bg-gray-100 text-gray-700';
    case 'rejected':
      return 'bg-red-100 text-red-700';
  }
}

function formatSessionType(t: SessionType): string {
  return t.replace('_', ' ');
}

function formatExpiry(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  });
}

interface ActionButtonsProps {
  session: TrainingSession;
  onStatus: (status: SessionStatus) => void;
  pending: boolean;
}

function ActionButtons({ session, onStatus, pending }: ActionButtonsProps) {
  const buttons: { label: string; status: SessionStatus }[] = [];
  if (session.status === 'suggested') {
    buttons.push({ label: 'Accept', status: 'planned' });
    buttons.push({ label: 'Reject', status: 'rejected' });
  } else if (session.status === 'planned') {
    buttons.push({ label: 'Mark done', status: 'done' });
    buttons.push({ label: 'Skip', status: 'skipped' });
  }
  if (buttons.length === 0) return null;
  return (
    <div className="flex gap-2 flex-wrap items-center">
      {buttons.map((b) => (
        <button
          key={b.status}
          type="button"
          onClick={() => onStatus(b.status)}
          disabled={pending}
          className="text-xs px-3 py-1 rounded border border-gray-300 text-gray-700 hover:bg-gray-50 disabled:opacity-50"
        >
          {b.label}
        </button>
      ))}
      {session.status === 'planned' && (
        <span
          className="text-xs px-2 py-0.5 rounded-full bg-blue-50 text-blue-700 border border-blue-200 cursor-help"
          title="Auto-matching to Strava activities isn't live yet — for now, mark sessions as done manually once you've run them. Automatic matching is coming in the next release."
        >
          ⓘ Matching with activity coming soon
        </span>
      )}
    </div>
  );
}

interface SessionCardProps {
  session: TrainingSession;
  onStatus: (status: SessionStatus) => void;
  pending: boolean;
}

export default function SessionCard({
  session,
  onStatus,
  pending,
}: SessionCardProps) {
  return (
    <article className="bg-white rounded-lg shadow p-4 space-y-3">
      <div className="flex items-start justify-between gap-3 flex-wrap">
        <div>
          <h2 className="text-lg font-semibold text-gray-800">
            {session.title}
          </h2>
          <div className="text-xs text-gray-500 mt-1 flex gap-2 items-center">
            <span className="capitalize">
              {formatSessionType(session.session_type)}
            </span>
            {session.expiry && (
              <>
                <span>·</span>
                <span>by {formatExpiry(session.expiry)}</span>
              </>
            )}
          </div>
        </div>
        <span
          className={`text-xs px-2 py-0.5 rounded-full ${statusBadgeClass(session.status)}`}
        >
          {session.status}
        </span>
      </div>

      <PrescriptionDisplay prescriptionJson={session.prescription_json} />

      <ActionButtons session={session} onStatus={onStatus} pending={pending} />
    </article>
  );
}
