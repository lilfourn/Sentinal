import { ChevronRight } from 'lucide-react';
import { cn } from '../../lib/utils';
import { FolderIcon } from '../icons/FolderIcon';
import { useNavigationStore } from '../../stores/navigation-store';

export function Breadcrumbs() {
  const { currentPath, navigateTo } = useNavigationStore();

  if (!currentPath) {
    return (
      <div className="px-4 py-2 text-sm text-gray-400">
        Select a folder to browse
      </div>
    );
  }

  // Parse path into segments
  const segments = currentPath.split('/').filter(Boolean);

  // Build paths for each segment
  const paths: { name: string; path: string }[] = segments.map((segment, index) => ({
    name: segment,
    path: '/' + segments.slice(0, index + 1).join('/'),
  }));

  // Add root if on Unix-like system
  if (currentPath.startsWith('/')) {
    paths.unshift({ name: '/', path: '/' });
  }

  return (
    <div className="px-4 py-2 flex items-center gap-1 overflow-x-auto no-select">
      <FolderIcon size={14} className="flex-shrink-0" />
      {paths.map(({ name, path }, index) => (
        <div key={path} className="flex items-center gap-1">
          {index > 0 && (
            <ChevronRight size={14} className="text-gray-400 flex-shrink-0" />
          )}
          <button
            onClick={() => navigateTo(path)}
            className={cn(
              'text-sm px-1.5 py-0.5 rounded transition-colors max-w-32 truncate',
              index === paths.length - 1
                ? 'text-gray-900 dark:text-gray-100 font-medium'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
            )}
            title={path}
          >
            {name}
          </button>
        </div>
      ))}
    </div>
  );
}
