import { AlertTriangle, X, Play, Trash2 } from 'lucide-react';
import { useOrganizeStore } from '../../stores/organize-store';

export function InterruptedJobBanner() {
  const {
    hasInterruptedJob,
    interruptedJob,
    dismissInterruptedJob,
    resumeInterruptedJob,
  } = useOrganizeStore();

  if (!hasInterruptedJob || !interruptedJob) {
    return null;
  }

  const progress = interruptedJob.totalOps > 0
    ? Math.round((interruptedJob.completedOps / interruptedJob.totalOps) * 100)
    : 0;

  const timeAgo = getTimeAgo(interruptedJob.startedAt);

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-sm animate-in slide-in-from-bottom-4 duration-300">
      <div className="bg-amber-900/90 backdrop-blur-sm border border-amber-700 rounded-lg shadow-xl p-4">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 p-2 bg-amber-800 rounded-full">
            <AlertTriangle size={20} className="text-amber-300" />
          </div>

          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-amber-100">
              Interrupted Organization Job
            </h3>
            <p className="mt-1 text-xs text-amber-300">
              An organization job for <span className="font-medium text-amber-100">{interruptedJob.folderName}</span> was interrupted {timeAgo}.
            </p>

            {interruptedJob.totalOps > 0 && (
              <div className="mt-2">
                <div className="flex items-center justify-between text-xs text-amber-400 mb-1">
                  <span>Progress</span>
                  <span>{interruptedJob.completedOps} / {interruptedJob.totalOps} operations</span>
                </div>
                <div className="h-1.5 bg-amber-950 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-amber-500 rounded-full transition-all"
                    style={{ width: `${progress}%` }}
                  />
                </div>
              </div>
            )}

            <div className="mt-3 flex items-center gap-2">
              <button
                onClick={resumeInterruptedJob}
                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-amber-900 bg-amber-400 hover:bg-amber-300 rounded transition-colors"
              >
                <Play size={12} />
                Resume
              </button>
              <button
                onClick={dismissInterruptedJob}
                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-amber-300 hover:text-amber-100 hover:bg-amber-800 rounded transition-colors"
              >
                <Trash2 size={12} />
                Dismiss
              </button>
            </div>
          </div>

          <button
            onClick={dismissInterruptedJob}
            className="flex-shrink-0 p-1 text-amber-400 hover:text-amber-200 hover:bg-amber-800 rounded transition-colors"
          >
            <X size={16} />
          </button>
        </div>
      </div>
    </div>
  );
}

function getTimeAgo(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days} day${days > 1 ? 's' : ''} ago`;
  }
  if (hours > 0) {
    return `${hours} hour${hours > 1 ? 's' : ''} ago`;
  }
  if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
  }
  return 'just now';
}
