import { usePhotoStore } from '../../stores/photo-store';
import { RefreshCw } from 'lucide-react';
import { cn } from '../../lib/utils';

export function PhotoToolbar() {
  const { totalCount, sortBy, setSortBy, scanPhotos, isScanning } = usePhotoStore();

  return (
    <div className="flex items-center justify-between px-6 py-4">
      <div className="flex items-center gap-3">
        <h1 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Photos</h1>
        <span className="text-sm text-gray-500 dark:text-gray-500">
          {totalCount.toLocaleString()}
        </span>
      </div>

      <div className="flex items-center gap-1">
        <div className="flex items-center bg-gray-100 dark:bg-gray-800 rounded-lg p-0.5 mr-2">
          {(['date', 'name', 'size'] as const).map((option) => (
            <button
              key={option}
              onClick={() => setSortBy(option)}
              className={cn(
                'px-3 py-1 text-xs font-medium rounded-md transition-all',
                sortBy === option
                  ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm'
                  : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
              )}
            >
              {option.charAt(0).toUpperCase() + option.slice(1)}
            </button>
          ))}
        </div>

        <button
          onClick={scanPhotos}
          disabled={isScanning}
          title="Refresh photos"
          className={cn(
            'p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-all',
            isScanning && 'opacity-50 cursor-not-allowed'
          )}
        >
          <RefreshCw size={16} className={cn(isScanning && 'animate-spin')} />
        </button>
      </div>
    </div>
  );
}
