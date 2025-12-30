import { FolderLock, Settings, RefreshCw } from 'lucide-react';
import { usePermissions } from '../../hooks/usePermissions';
import { cn } from '../../lib/utils';

interface PermissionErrorViewProps {
  path: string;
}

export function PermissionErrorView({ path }: PermissionErrorViewProps) {
  const { openSystemPreferences, checkPermissions, isChecking } = usePermissions();

  const folderName = path.split('/').pop() || path;

  return (
    <div className="flex flex-col items-center justify-center h-full p-8 text-center">
      <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
        <FolderLock className="text-red-500 dark:text-red-400" size={32} />
      </div>

      <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
        Cannot Access {folderName}
      </h2>

      <p className="text-gray-600 dark:text-gray-400 max-w-md mb-6">
        macOS requires your permission for Sentinel to access this folder. Please grant Full Disk
        Access in System Settings.
      </p>

      <div className="space-y-3">
        <button
          onClick={openSystemPreferences}
          className={cn(
            'flex items-center justify-center gap-2 w-full px-4 py-2.5',
            'bg-orange-500 text-white rounded-lg font-medium',
            'hover:bg-orange-600 transition-colors'
          )}
        >
          <Settings size={18} />
          Open System Settings
        </button>

        <button
          onClick={checkPermissions}
          disabled={isChecking}
          className={cn(
            'flex items-center justify-center gap-2 w-full px-4 py-2.5',
            'text-gray-700 dark:text-gray-300 rounded-lg font-medium',
            'hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors',
            isChecking && 'opacity-50 cursor-not-allowed'
          )}
        >
          <RefreshCw size={18} className={cn(isChecking && 'animate-spin')} />
          I've Granted Permission
        </button>
      </div>

      <div className="mt-8 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg text-left max-w-md">
        <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2">
          How to grant Full Disk Access:
        </h3>
        <ol className="text-sm text-gray-600 dark:text-gray-400 space-y-1 list-decimal list-inside">
          <li>Click "Open System Settings" above</li>
          <li>Navigate to Privacy & Security → Full Disk Access</li>
          <li>Click the lock icon and enter your password</li>
          <li>Enable the toggle next to "Sentinel"</li>
          <li>Return here and click "I've Granted Permission"</li>
        </ol>
      </div>
    </div>
  );
}
