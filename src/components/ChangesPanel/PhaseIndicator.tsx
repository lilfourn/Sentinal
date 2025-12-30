import { cn } from '../../lib/utils';
import type { OrganizePhase } from '../../stores/organize-store';

interface PhaseIndicatorProps {
  /** Current phase */
  currentPhase: OrganizePhase;
  /** Optional additional class names */
  className?: string;
}

/** All phases in order */
const PHASES: OrganizePhase[] = [
  'indexing',
  'planning',
  'simulation',
  'review',
  'committing',
];

/** Human-readable phase labels */
const PHASE_LABELS: Record<OrganizePhase, string> = {
  idle: 'Idle',
  indexing: 'Indexing',
  planning: 'Planning',
  simulation: 'Preview',
  review: 'Review',
  committing: 'Applying',
  rolling_back: 'Undoing',
  complete: 'Done',
  failed: 'Failed',
};

/**
 * Visual phase progress indicator showing dots for each phase.
 * Current phase pulses orange, completed phases are green, pending are gray.
 */
export function PhaseIndicator({ currentPhase, className }: PhaseIndicatorProps) {
  // Don't show for terminal states
  if (currentPhase === 'idle' || currentPhase === 'complete' || currentPhase === 'failed') {
    return null;
  }

  const currentIndex = PHASES.indexOf(currentPhase);

  return (
    <div className={cn('flex items-center gap-1', className)}>
      {PHASES.map((phase, index) => {
        const isComplete = index < currentIndex;
        const isCurrent = phase === currentPhase;
        const isPending = index > currentIndex;

        return (
          <div key={phase} className="flex items-center">
            {/* Phase dot */}
            <div
              className={cn(
                'w-2 h-2 rounded-full transition-all duration-300',
                isComplete && 'bg-green-500',
                isCurrent && 'bg-orange-500 animate-pulse shadow-[0_0_8px_rgba(249,115,22,0.5)]',
                isPending && 'bg-gray-600'
              )}
              title={PHASE_LABELS[phase]}
            />

            {/* Connector line (except after last) */}
            {index < PHASES.length - 1 && (
              <div
                className={cn(
                  'w-4 h-0.5 mx-0.5 transition-colors duration-300',
                  index < currentIndex ? 'bg-green-500/50' : 'bg-gray-600/50'
                )}
              />
            )}
          </div>
        );
      })}

      {/* Current phase label */}
      <span className="ml-2 text-[10px] font-medium text-gray-400 uppercase tracking-wider">
        {PHASE_LABELS[currentPhase]}
      </span>
    </div>
  );
}

/**
 * Compact version showing just the current phase name.
 */
export function PhaseLabel({ currentPhase }: { currentPhase: OrganizePhase }) {
  const isTerminal = currentPhase === 'complete' || currentPhase === 'failed';
  const isError = currentPhase === 'failed' || currentPhase === 'rolling_back';

  return (
    <span
      className={cn(
        'text-xs font-medium',
        isError && 'text-red-400',
        currentPhase === 'complete' && 'text-green-400',
        !isTerminal && !isError && 'text-orange-400'
      )}
    >
      {PHASE_LABELS[currentPhase]}
    </span>
  );
}
