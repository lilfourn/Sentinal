import { useUserDirectories } from '../../hooks/useDirectory';
import { useNavigationStore } from '../../stores/navigation-store';
import { SidebarItem } from '../sidebar/SidebarItem';
import { Image } from 'lucide-react';
import { cn } from '../../lib/utils';

export function Sidebar() {
  const { data: directories, isLoading } = useUserDirectories();
  const { currentPath, appMode, navigateTo, setAppMode } = useNavigationStore();

  const handleClick = (path: string) => {
    setAppMode('files');
    navigateTo(path);
  };

  const handlePhotosClick = () => {
    setAppMode('photos');
  };

  return (
    <aside className="glass-sidebar w-52 flex-shrink-0 h-full overflow-hidden flex flex-col">
      {/* Favorites section */}
      <div className="p-3 flex-1 overflow-y-auto">
        <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2 px-1">
          Favorites
        </h3>
        <nav className="space-y-0.5">
          {isLoading ? (
            <div className="px-3 py-2 text-sm text-gray-400">Loading...</div>
          ) : (
            directories
              ?.filter(({ name }) => name !== 'Pictures') // Photos library handles this
              .map(({ name, path }) => (
                <SidebarItem
                  key={path}
                  name={name}
                  path={path}
                  isActive={appMode === 'files' && currentPath === path}
                  onClick={handleClick}
                />
              ))
          )}
        </nav>

        {/* Library section */}
        <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2 mt-4 px-1">
          Library
        </h3>
        <nav className="space-y-0.5">
          <button
            onClick={handlePhotosClick}
            className={cn(
              'flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-sm transition-colors',
              appMode === 'photos'
                ? 'bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300'
                : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
            )}
          >
            <Image size={16} className={appMode === 'photos' ? 'text-orange-500' : 'text-gray-500'} />
            Photos
          </button>
        </nav>
      </div>
    </aside>
  );
}
