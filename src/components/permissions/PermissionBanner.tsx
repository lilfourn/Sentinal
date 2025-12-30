import { ShieldAlert, Settings, RefreshCw } from 'lucide-react';
import { usePermissions } from '../../hooks/usePermissions';
import { cn } from '../../lib/utils';

interface PermissionBannerProps {
  className?: string;
}

export function PermissionBanner({ className }: PermissionBannerProps) {
  const { inaccessibleDirs, hasPermissionIssues, isChecking, checkPermissions, openSystemPreferences } =
    usePermissions();

  // Don't show if no permission issues
  if (!hasPermissionIssues) {
    return null;
  }

  return (
    <div
      className={cn(
        'bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800',
        'px-4 py-3',
        className
      )}
    >
      <div className="flex items-start gap-3">
        <ShieldAlert className="text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" size={20} />

        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-amber-800 dark:text-amber-200">
            Permission Required
          </p>
          <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
            Sentinel needs Full Disk Access to browse protected folders
            {inaccessibleDirs.length > 0 && (
              <span className="font-medium"> ({inaccessibleDirs.map((d) => d.name).join(', ')})</span>
            )}
            .
          </p>

          <div className="flex items-center gap-3 mt-3">
            <button
              onClick={openSystemPreferences}
              className={cn(
                'inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg',
                'bg-amber-600 text-white hover:bg-amber-700',
                'transition-colors'
              )}
            >
              <Settings size={14} />
              Open System Settings
            </button>

            <button
              onClick={checkPermissions}
              disabled={isChecking}
              className={cn(
                'inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg',
                'text-amber-700 dark:text-amber-300 hover:bg-amber-100 dark:hover:bg-amber-800/50',
                'transition-colors',
                isChecking && 'opacity-50 cursor-not-allowed'
              )}
            >
              <RefreshCw size={14} className={cn(isChecking && 'animate-spin')} />
              Check Again
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
