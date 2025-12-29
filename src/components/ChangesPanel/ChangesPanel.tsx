import { useEffect, useRef } from 'react';
import {
  X,
  Loader2,
  CheckCircle,
  XCircle,
  Sparkles,
  Folder,
  Search,
  Brain,
  Lightbulb,
  Play,
  AlertCircle,
} from 'lucide-react';
import { cn } from '../../lib/utils';
import {
  useOrganizeStore,
  type AIThought,
  type ThoughtType,
} from '../../stores/organize-store';

export function ChangesPanel() {
  const {
    isOpen,
    targetFolder,
    thoughts,
    currentPhase,
    currentPlan,
    isExecuting,
    executedOps,
    closeOrganizer,
  } = useOrganizeStore();

  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to show latest thoughts
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [thoughts]);

  if (!isOpen) return null;

  const folderName = targetFolder?.split('/').pop() || 'Folder';

  const completedCount = executedOps.length;
  const totalCount = currentPlan?.operations.length || 0;
  const progress = totalCount > 0 ? (completedCount / totalCount) * 100 : 0;
  const isComplete = currentPhase === 'complete';
  const hasError = currentPhase === 'error';

  return (
    <div className="w-80 h-full flex flex-col border-l border-gray-200/20 glass-main">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-gray-200/20">
        <div className="flex items-center gap-2">
          <Sparkles size={16} className="text-purple-500" />
          <span className="font-medium text-sm text-gray-800 dark:text-gray-200">AI Organizer</span>
        </div>
        <button
          onClick={closeOrganizer}
          disabled={isExecuting}
          className="p-1 rounded hover:bg-gray-500/20 text-gray-500 dark:text-gray-400 disabled:opacity-50"
        >
          <X size={16} />
        </button>
      </div>

      {/* Target folder */}
      <div className="px-3 py-2 glass-file-header border-b border-gray-200/20">
        <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
          <Folder size={14} />
          <span className="truncate">{folderName}</span>
        </div>
      </div>

      {/* Phase indicator */}
      <div className="px-3 py-2 border-b border-gray-200/20">
        <PhaseIndicator phase={currentPhase} />
      </div>

      {/* Progress bar (only show during/after execution) */}
      {currentPlan && (
        <div className="px-3 py-2 border-b border-gray-200/20">
          <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
            <span>{getPhaseLabel(currentPhase)}</span>
            {totalCount > 0 && <span>{completedCount}/{totalCount}</span>}
          </div>
          <div className="h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
            <div
              className={cn(
                'h-full rounded-full transition-all duration-300',
                hasError ? 'bg-red-500' : isComplete ? 'bg-green-500' : 'bg-purple-500'
              )}
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      {/* Thinking stream */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto">
        <div className="p-2 space-y-1">
          {thoughts.map((thought) => (
            <ThoughtBubble key={thought.id} thought={thought} />
          ))}

          {/* Current thinking indicator */}
          {!isComplete && !hasError && (
            <div className="flex items-center gap-2 px-3 py-2 text-purple-500">
              <Loader2 size={14} className="animate-spin" />
              <span className="text-xs animate-pulse">
                {getThinkingLabel(currentPhase)}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Footer status */}
      {isComplete && (
        <div className="p-3 border-t border-gray-200/20 bg-green-500/10">
          <div className="flex items-center gap-2 text-green-400 text-sm">
            <CheckCircle size={16} />
            <span>Organization complete!</span>
          </div>
        </div>
      )}

      {hasError && (
        <div className="p-3 border-t border-gray-200/20 bg-red-500/10">
          <div className="flex items-center gap-2 text-red-400 text-sm">
            <XCircle size={16} />
            <span>Organization failed</span>
          </div>
        </div>
      )}
    </div>
  );
}

// Phase indicator showing current stage
function PhaseIndicator({ phase }: { phase: ThoughtType }) {
  const phases: { key: ThoughtType; label: string; icon: typeof Search }[] = [
    { key: 'scanning', label: 'Scan', icon: Search },
    { key: 'analyzing', label: 'Analyze', icon: Brain },
    { key: 'planning', label: 'Plan', icon: Lightbulb },
    { key: 'executing', label: 'Execute', icon: Play },
    { key: 'complete', label: 'Done', icon: CheckCircle },
  ];

  const currentIndex = phases.findIndex((p) => p.key === phase);

  return (
    <div className="flex items-center justify-between">
      {phases.map((p, i) => {
        const Icon = p.icon;
        const isActive = p.key === phase;
        const isPast = i < currentIndex;
        const isError = phase === 'error';

        return (
          <div key={p.key} className="flex flex-col items-center gap-1">
            <div
              className={cn(
                'w-6 h-6 rounded-full flex items-center justify-center transition-all',
                isActive && !isError && 'bg-purple-500 text-white',
                isPast && 'bg-green-500 text-white',
                !isActive && !isPast && 'bg-gray-200 dark:bg-gray-700 text-gray-400',
                isError && isActive && 'bg-red-500 text-white'
              )}
            >
              {isPast ? (
                <CheckCircle size={12} />
              ) : isActive && phase !== 'complete' && !isError ? (
                <Loader2 size={12} className="animate-spin" />
              ) : isError && isActive ? (
                <XCircle size={12} />
              ) : (
                <Icon size={12} />
              )}
            </div>
            <span
              className={cn(
                'text-[10px]',
                isActive ? 'text-purple-600 dark:text-purple-400 font-medium' : 'text-gray-400'
              )}
            >
              {p.label}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// Individual thought bubble
function ThoughtBubble({ thought }: { thought: AIThought }) {
  const typeConfig: Record<ThoughtType, { icon: typeof Search; color: string; bg: string }> = {
    scanning: { icon: Search, color: 'text-orange-500', bg: 'bg-orange-50 dark:bg-orange-900/20' },
    analyzing: { icon: Brain, color: 'text-purple-500', bg: 'bg-purple-50 dark:bg-purple-900/20' },
    planning: { icon: Lightbulb, color: 'text-amber-500', bg: 'bg-amber-50 dark:bg-amber-900/20' },
    thinking: { icon: Brain, color: 'text-purple-500', bg: 'bg-purple-50 dark:bg-purple-900/20' },
    executing: { icon: Play, color: 'text-green-500', bg: 'bg-green-50 dark:bg-green-900/20' },
    complete: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-50 dark:bg-green-900/20' },
    error: { icon: AlertCircle, color: 'text-red-500', bg: 'bg-red-50 dark:bg-red-900/20' },
  };

  const config = typeConfig[thought.type];
  const Icon = config.icon;

  return (
    <div className={cn('rounded-lg px-3 py-2', config.bg)}>
      <div className="flex items-start gap-2">
        <Icon size={14} className={cn('mt-0.5 flex-shrink-0', config.color)} />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {thought.content}
          </p>
          {thought.detail && (
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">
              {thought.detail}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function getPhaseLabel(phase: ThoughtType): string {
  switch (phase) {
    case 'scanning': return 'Scanning folder...';
    case 'analyzing': return 'Analyzing content...';
    case 'planning': return 'Creating plan...';
    case 'executing': return 'Executing...';
    case 'complete': return 'Complete';
    case 'error': return 'Error';
    default: return 'Processing...';
  }
}

function getThinkingLabel(phase: ThoughtType): string {
  switch (phase) {
    case 'scanning': return 'Reading folder contents...';
    case 'analyzing': return 'AI analyzing structure...';
    case 'planning': return 'Designing organization...';
    case 'executing': return 'Applying changes...';
    default: return 'Processing...';
  }
}
